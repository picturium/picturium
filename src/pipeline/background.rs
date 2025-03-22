use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use picturium_libvips::{Vips, VipsImage, VipsOperations};

pub(crate) async fn run(
    image: VipsImage,
    url_parameters: &UrlParameters<'_>,
) -> PipelineResult<VipsImage> {
    if let None = url_parameters.background {
        return Ok(image);
    }

    let background = url_parameters.background.as_ref().unwrap();

    if background.is_invisible() {
        return Ok(image);
    }

    let background = Vec::from(background);

    match image.background_color(&background) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!(
            "Failed to embed background: {}",
            Vips::get_error()
        ))),
    }
}
