use crate::enums::boolean::Boolean;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::loader::{default_load, generate_params, get_shrink_factor};
use anyhow::{Result, anyhow};
use picturium_libvips::{FromFileOptions, VipsAccess, VipsImage};

pub fn load(request: &PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let mut params = vec![];

    if request.parameters.auto_rotate == Boolean::True {
        params.push(("autorotate", "true"));
    }

    default_load(source_path, Some(params))
}
