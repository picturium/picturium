use log::{debug, error};
use picturium_libvips::{ThumbnailOptions, Vips, VipsImage, VipsInteresting, VipsSize, VipsThumbnails};
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::dimensions::get_output_dimensions;

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, height) = get_output_dimensions(&image, url_parameters);
    debug!("Resizing image to {}x{}", width, height);

    let image = image.thumbnail_image(width, ThumbnailOptions {
        height,
        size: VipsSize::Down,
        crop: VipsInteresting::Centre,
        ..ThumbnailOptions::default()
    }.into());

    Ok(match image {
        Ok(image) => image,
        Err(_) => {
            error!("Failed to resize image {} with dimensions {width}x{height}: {}", url_parameters.path.to_string_lossy(), Vips::get_error());
            return Err(PipelineError("Failed to resize image".to_string()))
        }
    })

}
