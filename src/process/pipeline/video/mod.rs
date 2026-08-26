mod extraction;

use crate::params::parsed::Parameters;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::path_generator::generate_intermediate_path;
use crate::services::cache::sidecar;
use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;

use self::extraction::{Frame, extract_frame};
use super::lock::acquire_conversion_lock;

/// Fallback frame - video start
const START: Frame = Frame::Index(0);

pub async fn process(request: &PipelineRequest<'_>) -> Result<String> {
    let source_path = &request.source.path;
    let config = &request.state.config.video;
    let requested = resolve_frame(request.parameters);
    let frame = requested.clone().unwrap_or_else(|| Frame::Time(config.default_time.clone()));
    let frame_path = generate_intermediate_path(request, source_path, &format!("{}.png", frame.variant()));

    if sidecar::is_valid(&frame_path, source_path).await {
        return Ok(frame_path);
    }

    let frame_dir = Path::new(&frame_path).parent().with_context(|| "Invalid frame path")?;

    tokio::fs::create_dir_all(frame_dir).await?;

    let _extraction_lock = acquire_conversion_lock(&frame_path).await?;

    if sidecar::is_valid(&frame_path, source_path).await {
        return Ok(frame_path);
    }

    let temporary_path = format!("{frame_path}.tmp");

    let extraction_timeout = Duration::from_secs(config.extraction_timeout);
    let mut extracted =
        extract_frame(source_path, &temporary_path, &frame, extraction_timeout).await;

    if extracted.is_err() && requested.is_none() && frame != START {
        tokio::fs::remove_file(&temporary_path).await.ok();
        extracted = extract_frame(source_path, &temporary_path, &START, extraction_timeout).await;
    }

    if let Err(err) = extracted {
        tokio::fs::remove_file(&temporary_path).await.ok();
        return Err(err);
    }

    tokio::fs::rename(&temporary_path, &frame_path).await?;
    sidecar::write(&frame_path, source_path).await?;

    Ok(frame_path)
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
