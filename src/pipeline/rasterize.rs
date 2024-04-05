use libvips::{ops, VipsImage};

use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::pipeline::resize::get_dimensions;
use crate::services::vips::get_error_message;

// Rasterize SVG image to bitmap
pub(crate) async fn run(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, _) = get_dimensions(image, url_parameters);

    match ops::thumbnail(&url_parameters.path.to_string_lossy(), width) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to rasterize SVG image: {}", get_error_message())))
    }

}