use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use picturium_libvips::{HeifSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: &VipsImage,
    keep: VipsKeep,
    quality: u8,
) -> anyhow::Result<Vec<u8>> {
    let output = &request.state.config.output;
    let config = &output.encoder.avif;

    let background = resolve_background(request.parameters.background);

    image
        .save_heif(Some(HeifSaveOptions {
            q: quality as i32,
            bitdepth: config.bitdepth,
            compression: config.compression.into(),
            effort: config.effort,
            subsample_mode: config.subsample.into(),
            encoder: config.encoder.into(),
            keep,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save AVIF image: {:?}", e))
}
