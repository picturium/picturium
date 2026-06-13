use crate::enums::output_quality::OutputQuality;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::finish::calculate_area;
use anyhow::Result;
use picturium_libvips::{JpegSaveOptions, VipsBufferSaving, VipsImage, VipsKeep, WebpSaveOptions};
use tracing::debug;

const MIN_QUALITY: f64 = 28.0;
const MAX_QUALITY: f64 = 75.0;
const MIN_QUALITY_AREA: f64 = 8.0;
const MAX_QUALITY_AREA: f64 = 0.25;
const LOW_QUALITY_MODIFIER: i32 = -12;
const HIGH_QUALITY_MODIFIER: i32 = 12;
const MAXIMUM_QUALITY_MODIFIER: i32 = 20;

pub(crate) fn finish_image(request: &PipelineRequest, image: VipsImage) -> Result<Vec<u8>> {
    image.save_jpeg(Some(JpegSaveOptions {
        q: match request.parameters.quality {
            OutputQuality::Value(quality) => quality as i32,
            quality => get_default_quality(&image, &quality),
        },
        optimize_coding: true,
        keep: VipsKeep::None,
        ..Default::default()
    })).map_err(|e| anyhow::anyhow!("Failed to save JPEG image: {:?}", e))
}

fn get_default_quality(image: &VipsImage, output_quality: &OutputQuality) -> i32 {
    let area = calculate_area(image);

    if area < MIN_QUALITY_AREA {
        return 90;
    }

    // Dynamic JPEG quality based on image area, min. 28, max. 75
    let quality = (MIN_QUALITY_AREA - area).clamp(0.0, MAX_QUALITY_AREA) * (MAX_QUALITY - MIN_QUALITY) / (MIN_QUALITY_AREA - MAX_QUALITY_AREA) + MIN_QUALITY;
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