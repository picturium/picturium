mod extraction;

use crate::params::parsed::Parameters;
use crate::process::pipeline::ResolvedSource;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::source_key;
use anyhow::{Context, Result};
use bytes::Bytes;
use std::path::Path;
use std::time::Duration;

use self::extraction::{Frame, extract_frame};

const START: Frame = Frame::Index(0);

pub async fn process(request: &PipelineRequest<'_>) -> Result<ResolvedSource> {
    let source_path = request.source.path.clone();
    let config = &request.state.config.video;
    let requested = resolve_frame(request.parameters);
    let frame = requested.clone().unwrap_or_else(|| Frame::Time(config.default_time.clone()));
    let variant = frame.variant();
    
    let key = source_key(
        "video:frame",
        &request.state.etag_seed,
        &source_path,
        &variant,
    ).await?;
    
    let extraction_timeout = Duration::from_secs(config.extraction_timeout);
    let extract = move || async move {
        match extract(&source_path, &frame, extraction_timeout).await {
            Ok(value) => Ok(value),
            Err(_) if requested.is_none() && frame != START => {
                extract(&source_path, &START, extraction_timeout).await
            }
            Err(error) => Err(error),
        }
    };
    
    let value = request.state.cache.resolve(key, request.forced, extract).await?;

    ResolvedSource::materialize(&value, ".png").await
}

async fn extract(source_path: &Path, frame: &Frame, timeout: Duration) -> Result<Bytes> {
    let output = tempfile::Builder::new()
        .prefix("picturium-frame-")
        .suffix(".png")
        .tempfile()
        .context("failed to create temporary video frame")?;
    
    let path = output.path().to_string_lossy().into_owned();
    extract_frame(source_path, &path, frame, timeout).await?;
    
    tokio::fs::read(output.path())
        .await
        .map(Bytes::from)
        .context("failed to read extracted video frame")
}

fn resolve_frame(parameters: &Parameters) -> Option<Frame> {
    if let Some(time) = &parameters.time {
        return Some(Frame::Time(time.0.clone()));
    }

    parameters
        .pages
        .as_ref()
        .and_then(|pages| pages.first())
        .map(|page| Frame::Index(page - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::params::RequestParams;
    use crate::params::pages::Pages;
    use crate::params::time::Time;
    use std::sync::Arc;

    fn parameters(params: RequestParams) -> Parameters {
        Parameters::new(&Arc::new(Config::default()), params)
    }

    #[test]
    fn without_a_selection_no_frame_is_requested() {
        assert_eq!(resolve_frame(&parameters(RequestParams::default())), None);
    }

    #[test]
    fn a_page_selects_a_frame_counting_from_zero() {
        let parameters = parameters(RequestParams {
            pages: Some(Pages(vec![30])),
            ..Default::default()
        });

        assert_eq!(resolve_frame(&parameters), Some(Frame::Index(29)));
    }

    #[test]
    fn a_time_wins_over_a_page() {
        let parameters = parameters(RequestParams {
            time: Some(Time("2.5".into())),
            pages: Some(Pages(vec![30])),
            ..Default::default()
        });

        assert_eq!(resolve_frame(&parameters), Some(Frame::Time("2.5".into())));
    }
}
