use crate::enums::boolean::Boolean;
use crate::enums::input::VipsInputFormat;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use picturium_libvips::{VipsImage, VipsOperations};

pub fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    if request.parameters.auto_rotate == Boolean::False {
        return Ok(image);
    }

    if request.input_format == VipsInputFormat::Jpeg
        || request.input_format == VipsInputFormat::Tiff
    {
        return Ok(image);
    }

    println!("Auto-rotating image");

    image
        .autorotate()
        .map_err(|e| anyhow::anyhow!("Failed to autorotate image: {:?}", e))
}
