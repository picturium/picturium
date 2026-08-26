mod lock;
mod office;
pub(crate) mod request;
pub(crate) mod svg;
mod video;
mod vips;

use crate::enums::input::InputFormat;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;

/// Resolve the source path, running async pre-pipelines (office, video) as needed.
pub async fn resolve_source_path(request: &PipelineRequest<'_>) -> Result<String> {
    match request.source.format {
        InputFormat::Office(_) => office::process(request).await,
        InputFormat::Video(_) => video::process(request).await,
        _ => Ok(request.source.path.to_string_lossy().to_string()),
    }
}

/// Run the vips image pipeline on a resolved source path.
pub fn process_image(request: &mut PipelineRequest<'_>, source_path: &str) -> Result<Vec<u8>> {
    vips::process(request, source_path)
}
