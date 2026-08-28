use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::task::JoinSet;
use tokio::time::timeout;

/// Samples at least this far apart cost less as one seek each than as a linear
/// decode of everything between them. A seek lands on the nearest keyframe and
/// decodes forward from there, so the crossover sits at the keyframe interval,
/// which is a handful of seconds in anything a browser will play.
const SPARSE_SAMPLE_SECONDS: f64 = 10.0;

/// Samples taken at once. Every one holds a decoder open, and a decoder costs a
/// buffer of frames at the source resolution, so this is what keeps a long clip
/// from opening hundreds of them at the same time.
const SAMPLE_CONCURRENCY: usize = 4;

/// Seconds into the video an ffmpeg timestamp points at, whether it is written
/// as plain seconds or as a timecode.
fn seconds(time: &str) -> Option<f64> {
    time.rsplit(':').enumerate().try_fold(0.0, |seconds, (place, part)| {
        let part: f64 = part.parse().ok().filter(|part| *part >= 0.0)?;
        Some(seconds + part * 60f64.powi(place as i32))
    })
}

/// What to pull out of a video: one still frame, or a run of frames sampled at
/// a fixed rate. A video is addressed by time throughout, because a seek to a
/// timestamp costs a keyframe interval where a frame number costs every frame
/// before it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Clip {
    /// Where the clip starts, as an ffmpeg timestamp.
    pub(super) start: String,
    pub(super) animation: Option<Animation>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Animation {
    pub(super) frames: u32,
    pub(super) fps: f64,
    /// Side of the box the vips pipeline scales the clip into, if it is known.
    /// Encoding a clip at full resolution costs orders of magnitude more than
    /// the pipeline ever reads back, so ffmpeg downscales while it decodes.
    pub(super) bound: Option<u16>,
}

impl Animation {
    /// Downscale to cover a `bound` x `bound` box, never up. Whatever the frame
    /// aspect, the shorter side lands on `bound`, so any output that fits in
    /// that box is still reachable after cropping, padding or rotating.
    fn scale_filter(&self) -> Option<String> {
        self.bound.map(|bound| {
            format!("scale=w=min(iw\\,max({bound}\\,{bound}*iw/ih)):h=-1")
        })
    }

    fn interval(&self) -> f64 {
        1.0 / self.fps
    }

    /// Whether the samples sit far enough apart to seek to each one instead of
    /// decoding the whole run between the first and the last.
    fn is_sparse(&self) -> bool {
        self.fps > 0.0 && self.frames > 1 && self.interval() >= SPARSE_SAMPLE_SECONDS
    }
}

impl Clip {
    /// A clip is decoded into an animated WebP so the ordinary animated-WebP
    /// loader reads its frames, delays and count back.
    pub(super) fn suffix(&self) -> &'static str {
        match self.animation {
            Some(_) => ".webp",
            None => ".png",
        }
    }

    /// Cache variant. Two selections that differ in any way must never share a
    /// key, or a still render and a clip of the same video collide.
    pub(super) fn variant(&self) -> String {
        match &self.animation {
            Some(animation) => format!(
                "{}-a{}x{:.4}-b{}",
                self.start_variant(),
                animation.frames,
                animation.fps,
                animation.bound.map_or(0, u16::from)
            ),
            None => self.start_variant(),
        }
    }

    fn start_variant(&self) -> String {
        format!("t-{}", self.start.replace([':', '.'], "-"))
    }
}

pub(super) async fn extract_clip(
    source_path: &Path,
    output_path: &str,
    clip: &Clip,
    extraction_timeout: Duration,
) -> Result<()> {
    // Sparse sampling is the one case a single pass handles badly: the `fps`
    // filter has to decode every frame between the first sample and the last,
    // which for a clip spanning a whole film is the whole film.
    let sparse = clip
        .animation
        .as_ref()
        .filter(|animation| animation.is_sparse())
        .zip(seconds(&clip.start));

    match sparse {
        Some((animation, start)) => {
            let sampled = sample_clip(source_path, output_path, animation, start);

            timeout(extraction_timeout, sampled).await.map_err(|_| {
                anyhow!(
                    "ffmpeg clip sampling timed out after {} seconds",
                    extraction_timeout.as_secs()
                )
            })??;
        }
        None => decode_clip(source_path, output_path, clip, extraction_timeout).await?,
    }

    let written = tokio::fs::metadata(output_path)
        .await
        .is_ok_and(|metadata| metadata.len() > 0);

    match written {
        true => Ok(()),
        false => Err(anyhow!(
            "ffmpeg produced no frames for {clip:?}, the video is likely shorter than that"
        )),
    }
}

/// Seek to every sample and decode the one frame it lands on, then mux the
/// stills back into the animation a single pass would have written.
async fn sample_clip(
    source_path: &Path,
    output_path: &str,
    animation: &Animation,
    start: f64,
) -> Result<()> {
    let directory = tempfile::Builder::new()
        .prefix("picturium-samples-")
        .tempdir()
        .context("failed to create temporary clip directory")?;

    let scale = animation.scale_filter();
    let mut taken = vec![false; animation.frames as usize];

    for batch in (0..animation.frames).collect::<Vec<_>>().chunks(SAMPLE_CONCURRENCY) {
        let mut samples = JoinSet::new();

        for index in batch.iter().copied() {
            let source_path = source_path.to_path_buf();
            let output = directory.path().join(sample_name(index));
            let scale = scale.clone();
            let at = start + f64::from(index) * animation.interval();

            samples.spawn(async move {
                (index, sample_frame(&source_path, &output, at, scale.as_deref()).await)
            });
        }

        while let Some(sample) = samples.join_next().await {
            let (index, taken_at) = sample.context("a clip sample task failed")?;
            taken[index as usize] = taken_at?;
        }

        // The stills are muxed back as a numbered sequence, so the run has to
        // stay contiguous. A gap means the video ended, and so does the clip.
        if batch.iter().any(|index| !taken[*index as usize]) {
            break;
        }
    }

    let frames = taken.iter().take_while(|taken| **taken).count();

    if frames == 0 {
        return Ok(());
    }

    mux_samples(directory.path(), output_path, animation, frames).await
}

/// One frame, seeked to rather than decoded up to. `-ss` ahead of `-i` puts the
/// seek in the demuxer, so it costs a keyframe interval instead of the whole
/// run from the start of the video.
async fn sample_frame(
    source_path: &Path,
    output: &Path,
    at: f64,
    scale: Option<&str>,
) -> Result<bool> {
    let mut command = Command::new("ffmpeg");

    command
        .kill_on_drop(true)
        .arg("-nostdin")
        .arg("-v")
        .arg("error")
        .arg("-ss")
        .arg(at.to_string())
        .arg("-i")
        .arg(source_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-an")
        .arg("-sn")
        .arg("-dn");

    if let Some(scale) = scale {
        command.arg("-vf").arg(scale);
    }

    let status = command
        .arg("-frames:v")
        .arg("1")
        .arg("-f")
        .arg("image2")
        .arg("-c:v")
        .arg("png")
        .arg("-update")
        .arg("1")
        .arg("-y")
        .arg(output)
        .status()
        .await
        .with_context(|| format!("failed to sample the video frame at {at} seconds"))?;

    // Seeking past the end is not an error: ffmpeg exits cleanly having written
    // nothing, and that is how the end of a clip is found.
    Ok(status.success()
        && tokio::fs::metadata(output)
            .await
            .is_ok_and(|metadata| metadata.len() > 0))
}

async fn mux_samples(
    directory: &Path,
    output_path: &str,
    animation: &Animation,
    frames: usize,
) -> Result<()> {
    let status = Command::new("ffmpeg")
        .kill_on_drop(true)
        .arg("-nostdin")
        .arg("-v")
        .arg("error")
        // The same rate a single pass tags the clip with, so both paths hand
        // the vips stage the frame delays it would have seen either way.
        .arg("-framerate")
        .arg(animation.fps.to_string())
        .arg("-start_number")
        .arg("0")
        .arg("-i")
        .arg(directory.join(SAMPLE_PATTERN))
        .arg("-frames:v")
        .arg(frames.to_string())
        .arg("-c:v")
        .arg("libwebp_anim")
        .arg("-lossless")
        .arg("1")
        .arg("-loop")
        .arg("0")
        .arg("-f")
        .arg("webp")
        .arg("-y")
        .arg(output_path)
        .status()
        .await
        .context("failed to mux the sampled video frames")?;

    match status.success() {
        true => Ok(()),
        false => Err(anyhow!(
            "ffmpeg failed to mux {frames} sampled frames with exit code: {:?}",
            status.code()
        )),
    }
}

const SAMPLE_PATTERN: &str = "%05d.png";

fn sample_name(index: u32) -> String {
    format!("{index:05}.png")
}

async fn decode_clip(
    source_path: &Path,
    output_path: &str,
    clip: &Clip,
    extraction_timeout: Duration,
) -> Result<()> {
    let mut command = Command::new("ffmpeg");

    command
        .kill_on_drop(true)
        .arg("-nostdin")
        .arg("-v")
        .arg("error")
        .arg("-ss")
        .arg(&clip.start)
        .arg("-i")
        .arg(source_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-an")
        .arg("-sn")
        .arg("-dn");

    match &clip.animation {
        Some(animation) => {
            // Sampling first, so the scaler only ever sees a frame that is kept.
            let mut filters = vec![format!("fps={}", animation.fps)];
            filters.extend(animation.scale_filter());

            command.arg("-vf").arg(filters.join(","));

            command
                .arg("-frames:v")
                .arg(animation.frames.to_string())
                .arg("-c:v")
                .arg("libwebp_anim")
                .arg("-lossless")
                .arg("1")
                .arg("-loop")
                .arg("0")
                .arg("-f")
                .arg("webp");
        }
        None => {
            command
                .arg("-frames:v")
                .arg("1")
                .arg("-f")
                .arg("image2")
                .arg("-c:v")
                .arg("png")
                .arg("-update")
                .arg("1");
        }
    }

    let mut child = command
        .arg("-y")
        .arg(output_path)
        .spawn()
        .with_context(|| "Failed to spawn ffmpeg command")?;

    let status = match timeout(extraction_timeout, child.wait()).await {
        Ok(status) => status.with_context(|| "Failed to wait on ffmpeg process")?,
        Err(_) => {
            child.kill().await.ok();
            return Err(anyhow!(
                "ffmpeg frame extraction timed out after {} seconds",
                extraction_timeout.as_secs()
            ));
        }
    };

    if !status.success() {
        return Err(anyhow!("ffmpeg failed with exit code: {:?}", status.code()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(frames: u32, fps: f64) -> Clip {
        Clip {
            start: "5".into(),
            animation: Some(Animation {
                frames,
                fps,
                bound: None,
            }),
        }
    }

    fn still(start: &str) -> Clip {
        Clip {
            start: start.into(),
            animation: None,
        }
    }

    #[test]
    fn a_variant_is_flat_enough_to_read_in_the_cache_tree() {
        assert_eq!(still("00:01:30.5").variant(), "t-00-01-30-5");
        assert_eq!(still("5").variant(), "t-5");
    }

    #[test]
    fn different_selections_never_share_a_variant() {
        assert_ne!(still("5").variant(), still("15").variant());
        assert_ne!(still("5").variant(), clip(10, 12.0).variant());
        assert_ne!(clip(10, 12.0).variant(), clip(20, 12.0).variant());
        assert_ne!(clip(10, 12.0).variant(), clip(10, 24.0).variant());

        let mut bounded = clip(10, 12.0);
        bounded.animation.as_mut().unwrap().bound = Some(200);
        assert_ne!(clip(10, 12.0).variant(), bounded.variant());
    }

    #[test]
    fn a_clip_is_only_scaled_when_a_bound_is_known() {
        assert_eq!(
            Animation { frames: 10, fps: 12.0, bound: None }.scale_filter(),
            None
        );
        assert_eq!(
            Animation { frames: 10, fps: 12.0, bound: Some(200) }.scale_filter().as_deref(),
            Some("scale=w=min(iw\\,max(200\\,200*iw/ih)):h=-1"),
        );
    }

    #[test]
    fn a_timestamp_is_read_in_whatever_shape_ffmpeg_accepts() {
        assert_eq!(seconds("0"), Some(0.0));
        assert_eq!(seconds("5"), Some(5.0));
        assert_eq!(seconds("2.5"), Some(2.5));
        assert_eq!(seconds("01:30"), Some(90.0));
        assert_eq!(seconds("00:01:30.5"), Some(90.5));
        assert_eq!(seconds("nonsense"), None);
    }

    #[test]
    fn only_widely_spaced_samples_are_worth_a_seek_each() {
        let animation = |frames, fps| Animation { frames, fps, bound: None };

        // 240 frames a second apart: a linear decode reads them all anyway.
        assert!(!animation(10, 1.0).is_sparse());
        // One frame every two minutes, the shape a big stride produces.
        assert!(animation(10, 1.0 / 120.0).is_sparse());
        // Exactly at the crossover, and a single frame is never a run.
        assert!(animation(10, 0.1).is_sparse());
        assert!(!animation(1, 1.0 / 120.0).is_sparse());
    }

    #[test]
    fn a_sample_is_named_so_ffmpeg_demuxes_it_back_in_order() {
        assert_eq!(sample_name(0), "00000.png");
        assert_eq!(sample_name(42), "00042.png");
        assert_eq!(SAMPLE_PATTERN, "%05d.png");
    }

    #[test]
    fn a_clip_is_decoded_into_an_animated_container() {
        assert_eq!(still("0").suffix(), ".png");
        assert_eq!(clip(10, 12.0).suffix(), ".webp");
    }
}
