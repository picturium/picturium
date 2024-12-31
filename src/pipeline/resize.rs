use std::mem::swap;
use log::{debug, error};
use picturium_libvips::{ThumbnailOptions, Vips, VipsImage, VipsInteresting, VipsSize, VipsThumbnails};
use crate::parameters::{Rotate, UrlParameters};
use crate::pipeline::{PipelineError, PipelineResult};

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let (width, height) = get_pipeline_dimensions(&image, url_parameters);
    debug!("Resizing image to {}x{}", width, height);

    let image = image.thumbnail_image(width, ThumbnailOptions {
        height,
        size: VipsSize::Down,
        crop: VipsInteresting::Centre,
        ..ThumbnailOptions::default()
    }.into());

    Ok(match image {
        Ok(image) => image,
        Err(_) => {
            error!("Failed to resize image {} with dimensions {width}x{height}: {}", url_parameters.path.to_string_lossy(), Vips::get_error());
            return Err(PipelineError("Failed to resize image".to_string()))
        }
    })

}

pub(crate) fn get_original_dimensions(image: &VipsImage) -> (i32, i32) {
    (image.get_width(), image.get_height())
}

pub(crate) fn get_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> (i32, i32) {

    let (mut width, mut height) = (url_parameters.width, url_parameters.height);

    if width.is_none() && height.is_none() {
        width = Some(image.get_width() as u16);
    }

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

fn get_pipeline_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> (i32, i32) {

    let (mut width, mut height) = (url_parameters.width, url_parameters.height);

    if url_parameters.rotate == Rotate::Left || url_parameters.rotate == Rotate::Right {
        swap(&mut width, &mut height);
    }

    if width.is_none() && height.is_none() {
        width = Some(image.get_width() as u16);
    }

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

pub(crate) fn get_rasterize_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> (i32, i32) {

    let (output_width, output_height) = get_dimensions(image, url_parameters);
    let (mut return_width, mut return_height) = (output_width, output_height);

    apply_rotate_dimensions(image, url_parameters, (&mut return_width, &mut return_height));

    // TODO > Check crop, if not set, return output_width, output_height

    // let (original_width, original_height) = get_original_dimensions(image);
    // let ratio = original_width as f64 / original_height as f64;

    debug!("Preprocessing image to {}x{}", return_width, return_height);
    (return_width, return_height)

}

fn apply_rotate_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>, (return_width, return_height): (&mut i32, &mut i32)) {

    if url_parameters.rotate == Rotate::No || url_parameters.rotate == Rotate::UpsideDown {
        return;
    }

    let (original_width, original_height) = get_original_dimensions(image);
    let ratio = *return_width as f64 / original_height as f64;
    
    if return_width > return_height {
        *return_width = *return_height + 2;
        *return_height = (original_height as f64 * ratio).round() as i32;
        return;
    }

    *return_height = *return_width + 2;
    *return_width = (original_width as f64 * ratio).round() as i32;

}