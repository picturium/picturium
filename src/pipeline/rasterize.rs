use log::debug;
use picturium_libvips::{ThumbnailOptions, Vips, VipsImage, VipsIntent, VipsInteresting, VipsThumbnails};
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::dimensions::get_rasterize_dimensions;

/// Some vector images need to be rasterized before further processing to ensure output quality.
/// 
/// The problem is that vector images are loaded in their defined dimensions and although they 
/// are vectors, vips rasterizes them according to those defined dimensions before processing.
/// This means that if the defined dimensions are smaller than the output resolution, we would get 
/// low quality output.
pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, height) = get_rasterize_dimensions(&image, url_parameters);
    
    if width <= image.get_width() && height <= image.get_height() {
        return Ok(image);
    }
    
    debug!("Rasterizing SVG image to {}x{}", width, height);

    match VipsImage::thumbnail(&url_parameters.path.to_string_lossy(), width, ThumbnailOptions {
        height,
        intent: VipsIntent::Perceptual,
        crop: VipsInteresting::Centre,
        ..ThumbnailOptions::default()
    }.into()) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to rasterize SVG image: {}", Vips::get_error())))
    }

}