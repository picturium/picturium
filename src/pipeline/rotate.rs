use libvips::{ops, VipsImage};
use libvips::ops::RotateOptions;

use crate::parameters::{Rotate, UrlParameters};
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::vips::get_error_message;

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let rotate = url_parameters.rotate as isize as f64;
    
    let (idx, idy) = match url_parameters.rotate {
        Rotate::Right => (0.5, -0.5),
        Rotate::UpsideDown => (0.5, 0.5),
        Rotate::Left => (-0.5, 0.5),
        _ => (0.0, 0.0)
    };
    
    match ops::rotate_with_opts(&image, rotate, &RotateOptions {
        idx,
        idy,
        background: vec![0.0, 0.0, 0.0],
        ..RotateOptions::default()
    }) {
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