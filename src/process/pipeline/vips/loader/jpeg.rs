use crate::enums::boolean::Boolean;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::loader::{default_load, get_shrink_factor};
use anyhow::Result;
use picturium_libvips::VipsImage;

pub fn load(request: &mut PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let mut params = vec![];

    if request.parameters.auto_rotate == Boolean::True {
        params.push(("autorotate", "true".into()));
    }

    if let Some(shrink) = get_shrink_factor(request, source_path)? {
        request.source.shrink = shrink.parse().unwrap_or(1.0);
        params.push(("shrink", shrink.into()));
    }

    default_load(source_path, Some(params))
}
