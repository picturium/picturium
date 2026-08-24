use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::loader::{default_load, get_shrink_factor_precise};
use anyhow::Result;
use picturium_libvips::VipsImage;

pub fn load(request: &mut PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let mut params = vec![];
    let shrink_factor = get_shrink_factor_precise(request, source_path)?;

    request.source.shrink = shrink_factor.parse().unwrap_or(1.0);
    params.push(("shrink", shrink_factor.as_str()));

    default_load(source_path, Some(params))
}
