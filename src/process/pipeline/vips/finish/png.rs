use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use picturium_libvips::{PngSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: &VipsImage,
    keep: VipsKeep,
    quality: u8,
) -> anyhow::Result<Vec<u8>> {
    let output = &request.state.config.output;
    let config = &output.encoder.png;

    let background = resolve_background(request.parameters.background);
    let lossless = quality as i32 >= config.lossless_quality;

    image
        .save_png(Some(PngSaveOptions {
            q: quality as i32,
            effort: config.effort.get(output.effort),
            keep,
            palette: !lossless,
            compression: match lossless {
                true => config.lossless_compression,
                false => config.compression
            },
            dither: match lossless {
                true => config.lossless_dither,
                false => config.dither
            },
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save PNG image: {:?}", e))
}
