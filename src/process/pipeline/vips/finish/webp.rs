use crate::enums::output_quality::OutputQuality;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::process::pipeline::vips::finish::calculate_area;
use picturium_libvips::{VipsBufferSaving, VipsImage, VipsKeep, VipsWebpPreset, WebpSaveOptions};
use tracing::debug;

const MIN_QUALITY: f64 = 40.0;
const MAX_QUALITY: f64 = 78.0;
const MIN_QUALITY_AREA: f64 = 8.0;
const MAX_QUALITY_AREA: f64 = 0.25;
const LOW_QUALITY_MODIFIER: i32 = -10;
const HIGH_QUALITY_MODIFIER: i32 = 8;
const MAXIMUM_QUALITY_MODIFIER: i32 = 15;

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: VipsImage,
    keep: VipsKeep,
) -> anyhow::Result<Vec<u8>> {
    let background = resolve_background(request.parameters.background);
    let area = calculate_area(&image);

    let quality = match request.parameters.quality {
        OutputQuality::Value(quality) => quality as i32,
        quality => get_default_quality(&image, &quality),
    };

    image
        .save_webp(Some(WebpSaveOptions {
            q: quality,
            preset: if area <= MAX_QUALITY_AREA {
                VipsWebpPreset::Text
            } else {
                VipsWebpPreset::Default
            },
            smart_subsample: true,
            keep,
            alpha_q: if quality > 75 { quality } else { 75 },
            effort: 2,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save WebP image: {:?}", e))
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
