use libvips::{ops, VipsImage};
use libvips::ops::Angle;

use crate::parameters::{Rotate, UrlParameters};
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::vips::get_error_message;

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let angle = match url_parameters.rotate {
        Rotate::Right => Angle::D270,
        Rotate::UpsideDown => Angle::D180,
        Rotate::Left => Angle::D90,
        _ => Angle::D0
    };

    match ops::rot(&image, angle) {
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