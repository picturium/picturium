mod loader;
mod finish;
mod resize;
mod rotate;
mod filter;

use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::finish::finish_image;
use anyhow::Result;
use picturium_libvips::VipsOperations;
use crate::params::rotate::Rotate;

/// Run the vips image pipeline on the given source path.
/// The path may point to the original source file (for natively vips-supported
/// formats) or to a temporary intermediate file produced by a pre-pipeline
/// (video / office).
pub fn process(request: &PipelineRequest, source_path: &str) -> Result<Vec<u8>> {
    let image = match loader::load_file(request, source_path) {
        Ok(file) => file,
        Err(e) => return Err(anyhow::anyhow!("Failed to load file: {:?}", e)),
    };

    let mut image = image.autorotate()
        .map_err(|e| anyhow::anyhow!("Failed to autorotate image: {:?}", e))?;

    if request.parameters.rotate != Rotate::No {
        image = rotate::process(request, image)?;
    }

    // TODO > Crop

    // Resize
    image = resize::process(request, image)?;

    // Filters
    image = filter::process(request, image)?;

    // TODO > Watermark

    finish_image(request, image)
}
