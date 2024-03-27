use libvips::{ops, VipsImage};

use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};

pub(crate) async fn run(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let image = match ops::autorot(image) {
        Ok(image) => image,
        Err(error) => return Err(PipelineError(format!("Failed to autorotate image: {}", error)))
    };
    
    let rotate = url_parameters.rotate as isize as f64;
    
    match ops::rotate(&image, rotate) {
        Ok(rotated_image) => Ok(rotated_image),
        Err(error) => Err(PipelineError(format!("Failed to rotate image: {}", error)))
    }

}