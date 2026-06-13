mod webp;
mod jpeg;
mod png;
mod gif;
mod avif;
mod jxl;

use crate::enums::output_format::OutputFormat;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use picturium_libvips::{VipsImage};

pub fn finish_image(request: &PipelineRequest, image: VipsImage) -> Result<Vec<u8>> {
    match request.output_format {
        OutputFormat::Jpeg => jpeg::finish_image(request, image),
        OutputFormat::Webp => webp::finish_image(request, image),
        OutputFormat::Avif => avif::finish_image(request, image),
        // OutputFormat::Jxl => jxl::finish_image(request, image),
        OutputFormat::Png => png::finish_image(request, image),
        OutputFormat::Gif => gif::finish_image(request, image),
        _ => return Err(anyhow::anyhow!("Unsupported output format: {:?}", request.output_format)),
    }.map_err(|e| anyhow::anyhow!("Failed to generate output image: {:?}", e))
}

fn calculate_area(image: &VipsImage) -> f64 {
    let width = image.get_width() as f64;
    let height = image.get_height() as f64;

    width * height / 1000000.0
}