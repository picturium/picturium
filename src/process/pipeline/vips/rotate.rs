use crate::params::rotate::Rotate;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::pages;
use anyhow::Result;
use picturium_libvips::{VipsImage, VipsOperations};

pub fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    let angle = request.parameters.rotate;

    if angle == Rotate::No {
        return Ok(image);
    }

    pages::per_page(image, |image| {
        image
            .rotate(angle.into())
            .map_err(|e| anyhow::anyhow!("Failed to rotate image: {:?}", e))
    })
}
