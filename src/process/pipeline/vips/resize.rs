use crate::enums::image_fit::ImageFit;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::pages;
use crate::services::size::calculate_processing_size;
use anyhow::Result;
use picturium_libvips::{ResizeOptions, VipsImage, VipsOperations};
use tracing::debug;

pub fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    let (width, height) = calculate_processing_size(request);

    let scale = get_scale(request, &image, width, height);
    let vertical_scale = get_vertical_scale(request, &image, height);

    if is_identity(scale, vertical_scale) {
        return Ok(image);
    }

    debug!(
        "Resizing image with scale {scale:.2} x {:.2} ({width} x {height})",
        vertical_scale.unwrap_or(scale)
    );

    pages::per_page(image, |image| {
        image
            .resize(
                scale,
                Some(ResizeOptions {
                    kernel: request.parameters.resample.into(),
                    vertical_scale,
                    ..Default::default()
                }),
            )
            .map_err(|e| anyhow::anyhow!("Failed to resize image: {:?}", e))
    })
}

fn is_identity(scale: f64, vertical_scale: Option<f64>) -> bool {
    let unchanged = |scale: f64| (scale - 1.0).abs() < f64::EPSILON;

    unchanged(scale) && vertical_scale.is_none_or(unchanged)
}

fn get_scale(request: &PipelineRequest, image: &VipsImage, width: i32, height: i32) -> f64 {
    if request.parameters.fit != ImageFit::Force
        && request.parameters.width.is_none()
        && request.parameters.height.is_some()
    {
        return height as f64 / pages::page_height(image) as f64;
    }

    width as f64 / image.get_width() as f64
}

fn get_vertical_scale(request: &PipelineRequest, image: &VipsImage, height: i32) -> Option<f64> {
    match request.parameters.fit == ImageFit::Force {
        true => Some(height as f64 / pages::page_height(image) as f64),
        false => None,
    }
}
