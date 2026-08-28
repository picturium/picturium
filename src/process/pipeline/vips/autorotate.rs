use crate::enums::boolean::Boolean;
use crate::enums::input::VipsInputFormat;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::pages;
use anyhow::Result;
use picturium_libvips::{VipsAnimations, VipsImage, VipsOperations};

pub fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    if request.parameters.auto_rotate == Boolean::False {
        return Ok(image);
    }

    if request.input_format == VipsInputFormat::Jpeg
        || request.input_format == VipsInputFormat::Tiff
    {
        return Ok(image);
    }

    if image.get_orientation().unwrap_or(1) == 1 {
        return Ok(image);
    }

    pages::per_page(image, |image| {
        image
            .autorotate()
            .map_err(|e| anyhow::anyhow!("Failed to autorotate image: {:?}", e))
    })
}
