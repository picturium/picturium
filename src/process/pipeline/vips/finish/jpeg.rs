use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::{resolve_background, resolve_opaque_matte};
use anyhow::Result;
use picturium_libvips::{
    FlattenOptions, JpegSaveOptions, VipsBufferSaving, VipsColors, VipsImage, VipsInterpretation,
    VipsKeep, VipsOperations,
};

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: &VipsImage,
    keep: VipsKeep,
    quality: u8,
) -> Result<Vec<u8>> {
    let config = &request.state.config.output.encoder.jpeg;
    let background = resolve_background(request.parameters.background);
    let flattened = prepare_for_jpeg(request, image)?;

    flattened
        .as_ref()
        .unwrap_or(image)
        .save_jpeg(Some(JpegSaveOptions {
            q: quality as i32,
            optimize_coding: config.optimize_coding,
            keep,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save JPEG image: {:?}", e))
}

fn prepare_for_jpeg(request: &PipelineRequest, image: &VipsImage) -> Result<Option<VipsImage>> {
    if !image.is_transparent() {
        return Ok(None);
    }

    let mut image = image.clone();

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
        .map(Some)
        .map_err(|error| anyhow::anyhow!("Failed to flatten alpha image for JPEG: {error}"))
}
