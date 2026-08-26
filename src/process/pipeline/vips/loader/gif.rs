use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::loader::default_load;
use anyhow::Result;
use picturium_libvips::VipsImage;

pub fn load(request: &PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let mut params = vec![];
    let pages = request.parameters.animate.frames.unwrap_or(1);

    let pages: String = format!("{pages}");
    params.push(("n", pages.as_str()));

    default_load(source_path, Some(params))
}
