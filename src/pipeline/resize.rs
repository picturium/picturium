use libvips::{ops, VipsImage};
use libvips::ops::{Interesting, Size, ThumbnailImageOptions};
use log::error;

use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::vips::get_error_message;

pub(crate) async fn run(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, height) = get_dimensions(image, url_parameters);

    let image = ops::thumbnail_image_with_opts(image, width, &ThumbnailImageOptions {
        height,
        size: Size::Down,
        crop: Interesting::Centre,
        import_profile: "sRGB".into(),
        export_profile: "sRGB".into(),
        ..ThumbnailImageOptions::default()
    });

    Ok(match image {
        Ok(image) => image,
        Err(_) => {
            error!("Failed to resize image {} with dimensions {width}x{height}: {}", url_parameters.path.to_string_lossy(), get_error_message());
            return Err(PipelineError("Failed to resize image".to_string()))
        }
    })

}

pub(crate) fn get_original_dimensions(image: &VipsImage) -> (i32, i32) {
    (image.get_width(), image.get_height())
}

pub(crate) fn get_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> (i32, i32) {

    // At this point, at least one is Some()
    let (mut width, mut height) = (url_parameters.width, url_parameters.height);

    let (original_width, original_height) = get_original_dimensions(image);
    let ratio = original_width as f64 / original_height as f64;

    if width.is_none() {
        width = Some((height.unwrap() as f64 * ratio).round() as u16);
    }

    if height.is_none() {
        height = Some((width.unwrap() as f64 / ratio).round() as u16);
    }

    (width.unwrap().into(), height.unwrap().into())

}