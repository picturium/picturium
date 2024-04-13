use libvips::{ops, VipsImage};
use libvips::ops::{Intent, Interesting, ThumbnailOptions};
use log::debug;

use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::pipeline::resize::get_rasterize_dimensions;
use crate::services::vips::get_error_message;

// Rasterize SVG image to bitmap
pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, height) = get_rasterize_dimensions(&image, url_parameters);
    debug!("Rasterizing SVG image to {}x{}", width, height);

    match ops::thumbnail_with_opts(&url_parameters.path.to_string_lossy(), width, &ThumbnailOptions {
        height,
        import_profile: "sRGB".to_string(),
        export_profile: "sRGB".to_string(),
        intent: Intent::Perceptual,
        crop: Interesting::Centre,
        ..ThumbnailOptions::default()
    }) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to rasterize SVG image: {}", get_error_message())))
    }

}