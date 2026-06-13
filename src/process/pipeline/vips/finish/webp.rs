use picturium_libvips::{VipsBufferSaving, VipsImage, VipsKeep, VipsWebpPreset, WebpSaveOptions};
use tracing::debug;
use crate::enums::output_quality::OutputQuality;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::finish::calculate_area;

const MIN_QUALITY: f64 = 28.0;
const MAX_QUALITY: f64 = 78.0;
const MIN_QUALITY_AREA: f64 = 8.0;
const MAX_QUALITY_AREA: f64 = 0.25;
const LOW_QUALITY_MODIFIER: i32 = -12;
const HIGH_QUALITY_MODIFIER: i32 = 12;
const MAXIMUM_QUALITY_MODIFIER: i32 = 22;

pub(crate) fn finish_image(request: &PipelineRequest, image: VipsImage) -> anyhow::Result<Vec<u8>> {
    image.save_webp(Some(WebpSaveOptions {
        q: match request.parameters.quality {
            OutputQuality::Value(quality) => quality as i32,
            quality => get_default_quality(&image, &quality),
        },
        preset: VipsWebpPreset::Default,
        smart_subsample: true,
        keep: VipsKeep::None,
        alpha_q: 50,
        ..Default::default()
    })).map_err(|e| anyhow::anyhow!("Failed to save WebP image: {:?}", e))
}

fn get_default_quality(image: &VipsImage, output_quality: &OutputQuality) -> i32 {
    let area = calculate_area(image);

    if area < MIN_QUALITY_AREA {
        return 100;
    }

    // Dynamic WebP quality based on image area, min. 28, max. 78
    let quality = (MIN_QUALITY_AREA - area).clamp(0.0, MIN_QUALITY_AREA - MAX_QUALITY_AREA) * (MAX_QUALITY - MIN_QUALITY) / (MIN_QUALITY_AREA - MAX_QUALITY_AREA) + MIN_QUALITY;
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