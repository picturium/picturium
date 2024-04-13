use libvips::{ops, VipsImage};

use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::vips::get_error_message;

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let rotate = url_parameters.rotate as isize as f64;
    
    match ops::rotate(&image, rotate) {
        Ok(rotated_image) => Ok(rotated_image),
        Err(_) => Err(PipelineError(format!("Failed to rotate image: {}", get_error_message())))
    }

}

pub(crate) async fn autorotate(image: VipsImage) -> PipelineResult<VipsImage> {
    match ops::autorot(&image) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to autorotate image: {}", get_error_message())))
    }
}