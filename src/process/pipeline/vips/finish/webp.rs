use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::process::pipeline::vips::finish::quality::calculate_area;
use crate::process::pipeline::vips::pages;
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
    let page_height = pages::page_height(image);
    let animated = image.get_height() > page_height;
    let keyframes = animated && config.animation_kmin >= 1;
    let defaults = WebpSaveOptions::default();

    image
        .save_webp(Some(WebpSaveOptions {
            q: quality,
            preset: match area <= config.text_preset_area {
                true => config.preset_small.into(),
                false => config.preset_large.into()
            },
            smart_subsample: config.smart_subsample && !animated,
            keep,
            alpha_q: quality.max(config.min_alpha_quality),
            effort: config.effort,
            background: &background,
            page_height,
            min_size: animated && config.animation_min_size,
            kmin: match keyframes {
                true => config.animation_kmin,
                false => defaults.kmin,
            },
            kmax: match keyframes {
                true => config.animation_kmax,
                false => defaults.kmax,
            },
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save WebP image: {:?}", e))
}
