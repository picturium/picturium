mod office;
pub(crate) mod request;
pub(crate) mod svg;
mod video;
mod vips;

use crate::enums::input::InputFormat;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::{Context, Result};
use bytes::Bytes;
use std::path::{Path, PathBuf};

pub struct ResolvedSource {
    path: PathBuf,
    _temporary: Option<tempfile::NamedTempFile>,
}

impl ResolvedSource {
    fn existing(path: PathBuf) -> Self {
        Self {
            path,
            _temporary: None,
        }
    }

    pub async fn materialize(value: &Bytes, suffix: &str) -> Result<Self> {
        let temporary = tempfile::Builder::new()
            .prefix("picturium-")
            .suffix(suffix)
            .tempfile()
            .context("failed to create temporary cached file")?;
        
        tokio::fs::write(temporary.path(), value)
            .await
            .context("failed to materialize cached file")?;

        Ok(Self {
            path: temporary.path().to_path_buf(),
            _temporary: Some(temporary),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Resolve the source path, running async pre-pipelines (office, video) as needed.
pub async fn resolve_source_path(request: &PipelineRequest<'_>) -> Result<ResolvedSource> {
    match request.source.format {
        InputFormat::Office(_) => office::process(request).await,
        InputFormat::Video(_) => video::process(request).await,
        _ => Ok(ResolvedSource::existing(request.source.path.clone())),
    }
}

/// Run the vips image pipeline on a resolved source path.
pub fn process_image(request: &mut PipelineRequest<'_>, source_path: &str) -> Result<Vec<u8>> {
    vips::process(request, source_path)
}
