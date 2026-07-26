use crate::enums::output_quality::OutputQuality;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::process::pipeline::vips::finish::calculate_area;
use picturium_libvips::{
    HeifSaveOptions, VipsBufferSaving, VipsHeifCompression, VipsHeifEncoder, VipsImage, VipsKeep,
    VipsSubsample,
};
use tracing::debug;

const MIN_QUALITY: f64 = 38.0;
const MAX_QUALITY: f64 = 67.0;
const MIN_QUALITY_AREA: f64 = 8.0;
const MAX_QUALITY_AREA: f64 = 0.25;
const LOW_QUALITY_MODIFIER: i32 = -10;
const HIGH_QUALITY_MODIFIER: i32 = 10;
const MAXIMUM_QUALITY_MODIFIER: i32 = 16;

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: VipsImage,
    keep: VipsKeep,
) -> anyhow::Result<Vec<u8>> {
    let background = resolve_background(request.parameters.background);
    image
        .save_heif(Some(HeifSaveOptions {
            q: match request.parameters.quality {
                OutputQuality::Value(quality) => quality as i32,
                quality => get_default_quality(&image, &quality),
            },
            bitdepth: 8,
            compression: VipsHeifCompression::AV1,
            effort: 1,
            subsample_mode: VipsSubsample::On,
            encoder: VipsHeifEncoder::AOM,
            keep,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save AVIF image: {:?}", e))
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
