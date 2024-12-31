use log::debug;
use picturium_libvips::{ThumbnailOptions, Vips, VipsImage, VipsIntent, VipsInteresting, VipsThumbnails};
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::pipeline::resize::get_rasterize_dimensions;

// Rasterize SVG image to bitmap
pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, height) = get_rasterize_dimensions(&image, url_parameters);
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