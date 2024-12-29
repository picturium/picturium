use picturium_libvips::{Vips, VipsImage, VipsOperations};
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let background = match &url_parameters.background {
        Some(background) => Vec::from(background),
        None => return Ok(image)
    };
    
    match image.background_color(&background) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to embed background: {}", Vips::get_error())))
    }

}