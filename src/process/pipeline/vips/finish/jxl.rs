use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use picturium_libvips::{JxlSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: &VipsImage,
    keep: VipsKeep,
    quality: u8,
) -> anyhow::Result<Vec<u8>> {
    let output = &request.state.config.output;
    let config = &output.encoder.jxl;

    let background = resolve_background(request.parameters.background);

    image
        .save_jxl(Some(JxlSaveOptions {
            tier: config.tier,
            distance: config.distance_per_quality * (100 - quality) as f64,
            effort: config.effort.get(output.effort),
            lossless: config.lossless,
            keep,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save JXL image: {:?}", e))
}
