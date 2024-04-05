use libvips::{ops, VipsImage};

use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::pipeline::resize::{get_dimensions, get_original_dimensions};
use crate::services::vips::get_error_message;

// Rasterize SVG image to bitmap
pub(crate) async fn run(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, _) = get_dimensions(image, url_parameters);
    let (original_width, original_height) = get_original_dimensions(image);

    let ratio = original_width as f64 / original_height as f64;
    let width = width as f64 * ratio;

    match ops::thumbnail(&url_parameters.path.to_string_lossy(), width.ceil() as i32) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to rasterize SVG image: {}", get_error_message())))
    }

}