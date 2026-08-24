use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::process::pipeline::vips::finish::quality::calculate_area;
use picturium_libvips::{VipsBufferSaving, VipsImage, VipsKeep, WebpSaveOptions};

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: &VipsImage,
    keep: VipsKeep,
    quality: u8,
) -> anyhow::Result<Vec<u8>> {
    let output = &request.state.config.output;
    let config = &output.encoder.webp;

    let background = resolve_background(request.parameters.background);
    let area = calculate_area(image);
    let quality = quality as i32;

    image
        .save_webp(Some(WebpSaveOptions {
            q: quality,
            preset: match area <= config.text_preset_area {
                true => config.preset_small.into(),
                false => config.preset_large.into()
            },
            smart_subsample: config.smart_subsample,
            keep,
            alpha_q: quality.max(config.min_alpha_quality),
            effort: config.effort,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save WebP image: {:?}", e))
}
