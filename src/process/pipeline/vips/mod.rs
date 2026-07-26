mod autorotate;
mod background;
mod canvas;
mod filter;
mod finish;
mod loader;
mod resize;
mod rotate;

use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::finish::finish_image;
use anyhow::Result;

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

    // TODO > Crop

    // Resize
    image = resize::process(request, image)?;

    // Cover crop or contain canvas, followed by padding.
    image = canvas::process(request, image)?;

    // Filters
    image = filter::process(request, image)?;

    // TODO > Watermark

    finish_image(request, image)
}
