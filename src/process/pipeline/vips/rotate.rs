use crate::params::rotate::Rotate;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use picturium_libvips::{VipsImage, VipsOperations};

pub fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    let angle = request.parameters.rotate;

    if angle == Rotate::No {
        return Ok(image);
    }

    let rotated_image = image
        .rotate(angle.into())
        .map_err(|e| anyhow::anyhow!("Failed to rotate image: {:?}", e))?;

    Ok(rotated_image)
}
