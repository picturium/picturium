use crate::enums::boolean::Boolean;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::loader::{animation_params, default_load};
use anyhow::Result;
use picturium_libvips::VipsImage;

pub fn load(request: &PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let mut params = animation_params(request, source_path)?;

    if request.parameters.auto_rotate == Boolean::True {
        params.push(("autorotate", "true".into()));
    }

    default_load(source_path, Some(params))
}
