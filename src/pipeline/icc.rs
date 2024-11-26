use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::vips::get_error_message;
use libvips::{ops, VipsImage};

pub(crate) async fn transform(image: VipsImage) -> PipelineResult<VipsImage> {
    match ops::icc_transform(&image, "sRGB") {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to transform image to sRGB: {}", get_error_message())))
    }
}