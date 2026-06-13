use crate::process::pipeline::request::PipelineRequest;
use picturium_libvips::{GifSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};

pub(crate) fn finish_image(request: &PipelineRequest, image: VipsImage) -> anyhow::Result<Vec<u8>> {
    image.save_gif(Some(GifSaveOptions {
        dither: 0.5,
        keep: VipsKeep::None,
        ..Default::default()
    })).map_err(|e| anyhow::anyhow!("Failed to save GIF image: {:?}", e))
}