use crate::enums::input::{InputFormat, VipsInputFormat};
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::pages;
use anyhow::{Result, anyhow};
use picturium_libvips::{VipsAnimations, VipsImage};

const DEFAULT_DELAY: i32 = 100;

pub(super) fn prepare(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    if !request.output_format.supports_animation() {
        return match request.input_format {
            VipsInputFormat::Pdf => Ok(image),
            _ => pages::flatten(image),
        };
    }

    let animate = &request.parameters.animate;
    let source_delays = image.get_delays();
    let pages = pages::page_count(&image);
    
    let stride = match request.source.format {
        InputFormat::Video(_) => 1,
        _ => animate.stride.map_or(1, i32::from).max(1),
    };
    
    let kept: Vec<i32> = (0..pages).step_by(stride as usize).collect();

    if kept.len() <= 1 {
        return pages::flatten(image);
    }

    let image = match kept.len() as i32 == pages {
        true => image,
        false => pages::select(image, &kept)?,
    };

    let delays = resolve_delays(animate.timing, source_delays.as_deref(), &kept);
    let image = image
        .set_delays(&delays)
        .map_err(|error| anyhow!("failed to set animation frame delays: {error}"))?;

    match animate.loop_count {
        Some(count) => image
            .set_loop(i32::from(count))
            .map_err(|error| anyhow!("failed to set animation loop count: {error}")),
        None => Ok(image),
    }
}

fn resolve_delays(timing: Option<u16>, source: Option<&[i32]>, kept: &[i32]) -> Vec<i32> {
    if let Some(timing) = timing {
        return vec![i32::from(timing); kept.len()];
    }

    let source = source.unwrap_or(&[]);
    let fallback = source.last().copied().unwrap_or(DEFAULT_DELAY);

    kept.iter()
        .map(|index| {
            source
                .get(*index as usize)
                .copied()
                .unwrap_or(fallback)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::output_format::OutputFormat;

    #[test]
    fn only_gif_and_webp_carry_frames() {
        assert!(OutputFormat::Gif.supports_animation());
        assert!(OutputFormat::Webp.supports_animation());
        assert!(!OutputFormat::Jpeg.supports_animation());
        assert!(!OutputFormat::Png.supports_animation());
        assert!(!OutputFormat::Avif.supports_animation());
        assert!(!OutputFormat::Jxl.supports_animation());
    }

    #[test]
    fn an_explicit_timing_replaces_every_source_delay() {
        assert_eq!(
            resolve_delays(Some(40), Some(&[70, 70, 70]), &[0, 1, 2]),
            vec![40, 40, 40],
        );
    }

    #[test]
    fn source_delays_follow_the_frames_that_survived_the_stride() {
        assert_eq!(
            resolve_delays(None, Some(&[10, 20, 30, 40, 50]), &[0, 2, 4]),
            vec![10, 30, 50],
        );
    }

    #[test]
    fn a_short_or_missing_delay_array_is_padded() {
        assert_eq!(resolve_delays(None, Some(&[60]), &[0, 1, 2]), vec![60, 60, 60]);
        assert_eq!(
            resolve_delays(None, None, &[0, 1]),
            vec![DEFAULT_DELAY, DEFAULT_DELAY],
        );
    }
}
