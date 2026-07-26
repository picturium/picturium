use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use picturium_libvips::{GifSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: VipsImage,
    keep: VipsKeep,
) -> anyhow::Result<Vec<u8>> {
    let background = resolve_background(request.parameters.background);

    image
        .save_gif(Some(GifSaveOptions {
            // effort: 2,
            // dither: 1.0,
            keep,
            // reuse: true,
            // interframe_maxerror: 2.0,
            // interpalette_maxerror: 3.0,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save GIF image: {:?}", e))
}
