use crate::enums::output_quality::OutputQuality;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::process::pipeline::vips::finish::calculate_area;
use picturium_libvips::{JxlSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};
use tracing::debug;

const MIN_QUALITY: f64 = 5.0;
const MAX_QUALITY: f64 = 2.2;
const MIN_QUALITY_AREA: f64 = 8.0;
const MAX_QUALITY_AREA: f64 = 0.25;
const LOW_QUALITY_MODIFIER: f64 = 1.5;
const HIGH_QUALITY_MODIFIER: f64 = -0.67;
const MAXIMUM_QUALITY_MODIFIER: f64 = -1.75;

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: VipsImage,
    keep: VipsKeep,
) -> anyhow::Result<Vec<u8>> {
    let background = resolve_background(request.parameters.background);
    image
        .save_jxl(Some(JxlSaveOptions {
            tier: 0,
            distance: match request.parameters.quality {
                OutputQuality::Value(quality) => 15.0 / 100.0 * (quality as f64 - 100.0).abs(),
                quality => get_default_quality(&image, &quality),
            },
            effort: 3,
            lossless: false,
            keep,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save JXL image: {:?}", e))
}

fn get_default_quality(image: &VipsImage, output_quality: &OutputQuality) -> f64 {
    let area = calculate_area(image);
    let quality = (MIN_QUALITY_AREA - area).clamp(0.0, MIN_QUALITY_AREA - MAX_QUALITY_AREA)
        * (MAX_QUALITY - MIN_QUALITY)
        / (MIN_QUALITY_AREA - MAX_QUALITY_AREA)
        + MIN_QUALITY;

    debug!("Serving image with quality: {}%, {area}MPix", quality);

    match output_quality {
        OutputQuality::Low => quality + LOW_QUALITY_MODIFIER,
        OutputQuality::Medium => quality,
        OutputQuality::High => quality + HIGH_QUALITY_MODIFIER,
        OutputQuality::Maximum => quality + MAXIMUM_QUALITY_MODIFIER,
        _ => quality,
    }
}
