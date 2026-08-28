mod autorotate;
mod background;
mod canvas;
mod filter;
mod finish;
mod loader;
mod resize;
mod rotate;
mod watermark;

use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::finish::finish_image;
use anyhow::Result;
use picturium_libvips::VipsImage;

/// Run the vips image pipeline on the given source path.
/// The path may point to the original source file (for natively vips-supported
/// formats) or to a temporary intermediate file produced by a pre-pipeline
/// (video / office).
pub fn process(request: &mut PipelineRequest, source_path: &str) -> Result<Vec<u8>> {
    let mut image = match loader::load_file(request, source_path) {
        Ok(file) => file,
        Err(e) => return Err(anyhow::anyhow!("Failed to load file: {:?}", e)),
    };

    image = autorotate::process(request, image)?;
    image = rotate::process(request, image)?;

    update_source_dimensions(request, &image);

    // TODO > Crop

    // Resize
    image = resize::process(request, image)?;

    // Cover crop or contain canvas, followed by padding.
    image = canvas::process(request, image)?;

    // Filters
    image = filter::process(request, image)?;

    // Watermark
    image = watermark::process(request, image)?;

    finish_image(request, image)
}

/// Store the dimensions of the source image, calculated back from the loaded
/// (possibly shrunk on load) image and the shrink factor.
fn update_source_dimensions(request: &mut PipelineRequest, image: &VipsImage) {
    let shrink = request.source.shrink;

    request.source.width = Some((image.get_width() as f64 * shrink).round() as u16);
    request.source.height = Some((image.get_height() as f64 * shrink).round() as u16);
}
