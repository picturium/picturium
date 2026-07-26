use crate::enums::output_quality::OutputQuality;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::{resolve_background, resolve_opaque_matte};
use crate::process::pipeline::vips::finish::calculate_area;
use anyhow::Result;
use picturium_libvips::{
    FlattenOptions, JpegSaveOptions, VipsBufferSaving, VipsColors, VipsImage, VipsInterpretation,
    VipsKeep, VipsOperations,
};
use tracing::debug;

const MIN_QUALITY: f64 = 32.0;
const MAX_QUALITY: f64 = 80.0;
const MIN_QUALITY_AREA: f64 = 8.0;
const MAX_QUALITY_AREA: f64 = 0.25;
const LOW_QUALITY_MODIFIER: i32 = -12;
const HIGH_QUALITY_MODIFIER: i32 = 10;
const MAXIMUM_QUALITY_MODIFIER: i32 = 17;

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: VipsImage,
    keep: VipsKeep,
) -> Result<Vec<u8>> {
    let background = resolve_background(request.parameters.background);
    let image = prepare_for_jpeg(request, image)?;

    image
        .save_jpeg(Some(JpegSaveOptions {
            q: match request.parameters.quality {
                OutputQuality::Value(quality) => quality as i32,
                quality => get_default_quality(&image, &quality),
            },
            optimize_coding: true,
            keep,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save JPEG image: {:?}", e))
}

fn prepare_for_jpeg(request: &PipelineRequest, mut image: VipsImage) -> Result<VipsImage> {
    if !image.is_transparent() {
        return Ok(image);
    }

    if !matches!(image.get_interpretation(), VipsInterpretation::sRGB) {
        image = image
            .set_colorspace(VipsInterpretation::sRGB)
            .map_err(|error| {
                anyhow::anyhow!("Failed to convert alpha image to sRGB for JPEG: {error}")
            })?;
    }

    let matte = resolve_opaque_matte(request.parameters.background);
    image
        .flatten(Some(FlattenOptions { background: &matte }))
        .map_err(|error| anyhow::anyhow!("Failed to flatten alpha image for JPEG: {error}"))
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
