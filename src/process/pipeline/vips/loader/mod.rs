mod svg;

use crate::enums::input::{InputFormat, VipsInputFormat};
use crate::process::pipeline::request::PipelineRequest;
use crate::process::source::Source;
use anyhow::{anyhow, Result};
use picturium_libvips::VipsImage;
use std::path::PathBuf;

/// Load a file that is known to be vips-compatible from `source_path`.
/// The format is re-derived from the path extension so that temporary files
/// produced by conversion pipelines (video / office) are handled correctly.
pub fn load_file(request: &PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let format = match Source::get_format(&PathBuf::from(source_path)) {
        InputFormat::Vips(format) => format,
        _ => return Err(anyhow!("Unsupported vips input format for source path: {}", source_path)),
    };

    Ok(load_vips_file(request, source_path, format)?)
}

fn load_vips_file(request: &PipelineRequest, source_path: &str, format: VipsInputFormat) -> Result<VipsImage> {
    match format {
        VipsInputFormat::Svg => svg::load(request, source_path),
        _ => VipsImage::new_from_file(&(source_path.to_owned() + "[revalidate=true]"), None).map_err(|e| anyhow!(e)),
    }
}