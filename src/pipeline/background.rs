use libvips::{ops, VipsImage};
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::vips::get_error_message;

pub(crate) async fn run(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let background = match &url_parameters.background {
        Some(background) => Vec::from(background),
        None => return Ok(image.clone())
    };

    let background_image = VipsImage::new_from_image(image, &background).unwrap();
    
    match ops::composite_2(&background_image, image, ops::BlendMode::Over) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to embed background: {}", get_error_message())))
    }

}