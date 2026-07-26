use crate::enums::image_fit::ImageFit;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::size::calculate_processing_size;
use anyhow::Result;
use picturium_libvips::{ResizeOptions, VipsImage, VipsOperations};
use tracing::debug;

pub fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    let (width, height) = calculate_processing_size(request, &image);

    let scale = get_scale(request, &image, width, height);
    let vertical_scale = get_vertical_scale(request, &image, height);

    let options = ResizeOptions {
        kernel: request.parameters.resample.into(),
        vertical_scale,
        ..Default::default()
    };

    debug!(
        "Resizing image with scale {scale:.2} x {:.2} ({width} x {height}) and options {options:?}",
        vertical_scale.unwrap_or(0.0)
    );

    let resized_image = image
        .resize(scale, Some(options))
        .map_err(|e| anyhow::anyhow!("Failed to resize image: {:?}", e))?;

    Ok(resized_image)
}

fn get_scale(request: &PipelineRequest, image: &VipsImage, width: i32, height: i32) -> f64 {
    if request.parameters.fit != ImageFit::Force
        && request.parameters.width.is_none()
        && request.parameters.height.is_some()
    {
        return height as f64 / image.get_height() as f64;
    }

    width as f64 / image.get_width() as f64
}

fn get_vertical_scale(request: &PipelineRequest, image: &VipsImage, height: i32) -> Option<f64> {
    match request.parameters.fit == ImageFit::Force {
        true => Some(height as f64 / image.get_height() as f64),
        false => None,
    }
}
