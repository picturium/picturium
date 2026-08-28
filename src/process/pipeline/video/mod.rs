mod extraction;

use crate::config::video::VideoConfig;
use crate::params::aspect_ratio::AspectRatio;
use crate::params::parsed::Parameters;
use crate::process::pipeline::ResolvedSource;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::source_key;
use anyhow::{Context, Result};
use bytes::Bytes;
use std::path::Path;
use std::time::Duration;

use self::extraction::{Animation, Clip, extract_clip};

const START: &str = "0";

pub async fn process(request: &PipelineRequest<'_>) -> Result<ResolvedSource> {
    let source_path = request.source.path.clone();
    let config = &request.state.config.video;
    let requested = request.parameters.time.as_ref().map(|time| time.0.clone());
    
    let clip = Clip {
        start: requested.clone().unwrap_or_else(|| config.default_time.clone()),
        animation: resolve_animation(request, config),
    };
    
    let variant = clip.variant();
    let suffix = clip.suffix();

    let key = source_key(
        "video:frame",
        &request.state.etag_seed,
        &source_path,
        &variant,
    ).await?;

    let extraction_timeout = Duration::from_secs(match clip.animation {
        Some(_) => config.animation_timeout,
        None => config.extraction_timeout,
    });
    
    let extract = move || async move {
        let fallback = Clip {
            start: START.into(),
            ..clip.clone()
        };

        match extract(&source_path, &clip, extraction_timeout).await {
            Ok(value) => Ok(value),
            Err(_) if requested.is_none() && clip != fallback => {
                extract(&source_path, &fallback, extraction_timeout).await
            }
            Err(error) => Err(error),
        }
    };

    let value = request.state.cache.resolve(key, request.forced, extract).await?;

    ResolvedSource::materialize(&value, suffix).await
}

async fn extract(source_path: &Path, clip: &Clip, timeout: Duration) -> Result<Bytes> {
    let output = tempfile::Builder::new()
        .prefix("picturium-frame-")
        .suffix(clip.suffix())
        .tempfile()
        .context("failed to create temporary video frame")?;

    let path = output.path().to_string_lossy().into_owned();
    extract_clip(source_path, &path, clip, timeout).await?;

    tokio::fs::read(output.path())
        .await
        .map(Bytes::from)
        .context("failed to read extracted video frame")
}

fn resolve_animation(request: &PipelineRequest, config: &VideoConfig) -> Option<Animation> {
    let animate = &request.parameters.animate;

    if !animate.is_requested() || !request.output_format.supports_animation() {
        return None;
    }

    let fps = match animate.timing {
        Some(timing) => 1000.0 / f64::from(timing),
        None => config.animation_fps,
    };
    
    let stride = f64::from(animate.stride.unwrap_or(1).max(1));

    let frames = animate
        .frames
        .filter(|frames| *frames > 0)
        .map(|frames| frames as u32)
        .unwrap_or(config.animation_frames)
        .clamp(1, config.max_animation_frames);

    Some(Animation {
        frames,
        fps: fps / stride,
        bound: frame_bound(request.parameters),
    })
}

fn frame_bound(parameters: &Parameters) -> Option<u16> {
    if parameters.crop.is_some() {
        return None;
    }

    let ratio = match parameters.aspect_ratio {
        AspectRatio::Value(ratio) if ratio > 0.0 => Some(f64::from(ratio)),
        _ => None,
    };

    let axis = |value: Option<u16>, counterpart: fn(f64, f64) -> f64| {
        value.map(f64::from).map(|value| match ratio {
            Some(ratio) => value.max(counterpart(value, ratio)),
            None => value,
        })
    };

    let width = axis(parameters.width, |width, ratio| width / ratio);
    let height = axis(parameters.height, |height, ratio| height * ratio);

    let modifier = f64::from(parameters.scale * parameters.dpr).max(1.0);
    let bound = width.unwrap_or(0.0).max(height.unwrap_or(0.0)) * modifier;

    match bound >= 1.0 {
        true => Some((bound.ceil() as u64 + 1).min(u16::MAX as u64) as u16),
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::params::RequestParams;
    use std::sync::Arc;

    fn parameters(params: RequestParams) -> Parameters {
        Parameters::new(&Arc::new(Config::default()), params)
    }

    fn bound(params: RequestParams) -> Option<u16> {
        frame_bound(&parameters(params))
    }

    #[test]
    fn a_request_that_bounds_nothing_leaves_the_frame_alone() {
        assert_eq!(bound(RequestParams::default()), None);
    }

    #[test]
    fn a_crop_is_measured_in_source_pixels_so_the_frame_stays_full_size() {
        assert_eq!(
            bound(RequestParams {
                width: Some(200),
                crop: Some("w:640|h:480".parse().unwrap()),
                ..Default::default()
            }),
            None,
        );
    }

    #[test]
    fn the_bound_covers_the_larger_requested_side() {
        assert_eq!(
            bound(RequestParams { width: Some(200), ..Default::default() }),
            Some(201),
        );
        assert_eq!(
            bound(RequestParams { height: Some(300), ..Default::default() }),
            Some(301),
        );
        assert_eq!(
            bound(RequestParams {
                width: Some(200),
                height: Some(300),
                ..Default::default()
            }),
            Some(301),
        );
    }

    #[test]
    fn an_aspect_ratio_fills_in_the_side_the_request_left_out() {
        // 200 wide at 9/16 needs a 356 tall frame to crop or pad out of.
        assert_eq!(
            bound(RequestParams {
                width: Some(200),
                aspect_ratio: Some("9/16".parse().unwrap()),
                ..Default::default()
            }),
            Some(357),
        );
        // 16/9 the other way round leaves the requested width the larger side.
        assert_eq!(
            bound(RequestParams {
                width: Some(200),
                aspect_ratio: Some("16/9".parse().unwrap()),
                ..Default::default()
            }),
            Some(201),
        );
    }

    #[test]
    fn a_pixel_ratio_raises_the_bound_but_never_lowers_it() {
        assert_eq!(
            bound(RequestParams { width: Some(200), dpr: Some(2.0), ..Default::default() }),
            Some(401),
        );
        assert_eq!(
            bound(RequestParams { width: Some(200), scale: Some(0.5), ..Default::default() }),
            Some(201),
        );
    }

}
