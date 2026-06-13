pub(crate) mod request;
mod vips;
mod office;
mod video;

use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use crate::enums::input::InputFormat;

/// Resolve source path, running async pre-pipelines (office, video) as needed.
pub async fn resolve_source_path(request: &PipelineRequest<'_>) -> Result<String> {
    match request.source.format {
        InputFormat::Office(_) => office::process(request).await,
        _ => Ok(request.source.path.to_string_lossy().to_string()),
    }
}

/// Run the vips image pipeline on a resolved source path. Blocking — run inside spawn_blocking.
pub fn process_image(request: &PipelineRequest<'_>, source_path: &str) -> Result<Vec<u8>> {
    vips::process(request, source_path)
}