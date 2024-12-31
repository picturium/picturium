use picturium_libvips::{VipsAngle, VipsImage, VipsOperations};
use crate::parameters::{Rotate, UrlParameters};
use crate::pipeline::{PipelineError, PipelineResult};

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let angle = match url_parameters.rotate {
        Rotate::Right => VipsAngle::Right,
        Rotate::UpsideDown => VipsAngle::UpsideDown,
        Rotate::Left => VipsAngle::Left,
        _ => VipsAngle::None
    };

    match image.rotate(angle) {
        Ok(rotated_image) => Ok(rotated_image),
        Err(e) => Err(PipelineError(e.to_string()))
    }

}

pub(crate) async fn autorotate(image: VipsImage) -> PipelineResult<VipsImage> {
    match image.autorotate() {
        Ok(image) => Ok(image),
        Err(e) => Err(PipelineError(e.to_string()))
    }
}