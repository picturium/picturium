use crate::enums::output_quality::OutputQuality;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::process::pipeline::vips::finish::calculate_area;
use picturium_libvips::{PngSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};
use tracing::debug;

const MIN_QUALITY: f64 = 32.0;
const MAX_QUALITY: f64 = 99.0;
const MIN_QUALITY_AREA: f64 = 8.0;
const MAX_QUALITY_AREA: f64 = 0.25;
const LOW_QUALITY_MODIFIER: i32 = -30;
const HIGH_QUALITY_MODIFIER: i32 = 0;
const MAXIMUM_QUALITY_MODIFIER: i32 = 1;

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: VipsImage,
    keep: VipsKeep,
) -> anyhow::Result<Vec<u8>> {
    let background = resolve_background(request.parameters.background);
    let quality = match request.parameters.quality {
        OutputQuality::Value(quality) => quality as i32,
        quality => get_default_quality(&image, &quality),
    };

    image
        .save_png(Some(PngSaveOptions {
            q: quality,
            effort: 2,
            keep,
            palette: if quality == 100 { false } else { true },
            compression: if quality == 100 { 3 } else { 2 },
            dither: if quality == 100 { 0.0 } else { 0.25 },
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save PNG image: {:?}", e))
}

fn get_default_quality(image: &VipsImage, output_quality: &OutputQuality) -> i32 {
    let area = calculate_area(image);

    let quality = (MIN_QUALITY_AREA - area).clamp(0.0, MIN_QUALITY_AREA - MAX_QUALITY_AREA)
        * (MAX_QUALITY - MIN_QUALITY)
        / (MIN_QUALITY_AREA - MAX_QUALITY_AREA)
        + MIN_QUALITY;
    let quality = quality as i32;

    debug!("Serving image with quality: {}%, {area}MPix", quality);

    match output_quality {
        OutputQuality::Low => quality + LOW_QUALITY_MODIFIER,
        OutputQuality::Medium => quality,
        OutputQuality::High => quality + HIGH_QUALITY_MODIFIER,
        OutputQuality::Maximum => quality + MAXIMUM_QUALITY_MODIFIER,
        _ => quality,
    }
}
