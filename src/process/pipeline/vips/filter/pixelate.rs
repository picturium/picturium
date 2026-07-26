use crate::process::pipeline::request::PipelineRequest;
use crate::services::size::calculate_requested_size;
use anyhow::Result;
use picturium_libvips::{ResizeOptions, VipsImage, VipsKernel, VipsOperations};

/// The image is already downscaled in services/size.rs, this method scales the image back to its requested size
pub fn apply(request: &PipelineRequest, image: VipsImage, value: u16) -> Result<VipsImage> {
    if value < 2 {
        return Ok(image);
    }

    let (original_width, original_height) = (
        request
            .source
            .width
            .unwrap_or_else(|| image.get_width() as u16),
        request
            .source
            .height
            .unwrap_or_else(|| image.get_height() as u16),
    );

    let (width, height) = calculate_requested_size(request, (original_width, original_height));

    let horizontal_scale = width as f64 / image.get_width() as f64;
    let vertical_scale = height as f64 / image.get_height() as f64;

    image
        .resize(
            horizontal_scale,
            Some(ResizeOptions {
                vertical_scale: Some(vertical_scale),
                kernel: VipsKernel::Nearest,
                ..Default::default()
            }),
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to upscale image back to apply pixelate filter: {:?}",
                e
            )
        })
}
