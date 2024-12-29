use picturium_libvips::{Vips, VipsImage, VipsOperations};
use crate::pipeline::{PipelineError, PipelineResult};

pub(crate) async fn transform(image: VipsImage) -> PipelineResult<VipsImage> {
    match image.icc_transform("sRGB", None) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to transform image to sRGB: {}", Vips::get_error())))
    }
}