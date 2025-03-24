use std::mem::swap;
use log::debug;
use picturium_libvips::VipsImage;
use crate::parameters::{Rotate, UrlParameters};

/// Calculate minimum required dimensions for rasterizing vector images / documents pages before processing
pub(crate) fn get_rasterize_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> (i32, i32) {

    let (mut return_width, mut return_height) = get_requested_dimensions(image, url_parameters);

    apply_rotation(image, url_parameters, (&mut return_width, &mut return_height));

    // TODO > Check crop, if not set, return output_width, output_height

    debug!("Preprocessing image to {}x{}", return_width, return_height);
    (return_width, return_height)

}

/// Calculate final dimensions of the output image
pub(crate) fn get_output_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> (i32, i32) {

    let (mut width, mut height) = (url_parameters.width, url_parameters.height);

    if url_parameters.rotate == Rotate::Left || url_parameters.rotate == Rotate::Right {
        swap(&mut width, &mut height);
    }

    if width.is_none() && height.is_none() {
        width = Some(image.get_width() as u16);
    }

    let (original_width, original_height) = get_image_dimensions(image);
    let ratio = original_width as f64 / original_height as f64;

    if width.is_none() {
        width = Some((height.unwrap() as f64 * ratio).round() as u16);
    }

    if height.is_none() {
        height = Some((width.unwrap() as f64 / ratio).round() as u16);
    }

    (width.unwrap().into(), height.unwrap().into())

}

fn apply_rotation(image: &VipsImage, url_parameters: &UrlParameters<'_>, (return_width, return_height): (&mut i32, &mut i32)) {

    if url_parameters.rotate == Rotate::No || url_parameters.rotate == Rotate::UpsideDown {
        return;
    }

    let (original_width, original_height) = get_image_dimensions(image);
    let ratio = *return_width as f64 / original_height as f64;

    if return_width > return_height {
        *return_width = *return_height + 2;
        *return_height = (original_height as f64 * ratio).round() as i32;
        return;
    }

    *return_height = *return_width + 2;
    *return_width = (original_width as f64 * ratio).round() as i32;

}

fn get_image_dimensions(image: &VipsImage) -> (i32, i32) {
    (image.get_width(), image.get_height())
}

fn get_requested_dimensions(image: &VipsImage, url_parameters: &UrlParameters<'_>) -> (i32, i32) {

    let (mut width, mut height) = (url_parameters.width, url_parameters.height);

    if width.is_none() && height.is_none() {
        width = Some(image.get_width() as u16);
    }

    let (original_width, original_height) = get_image_dimensions(image);
    let ratio = original_width as f64 / original_height as f64;

    if width.is_none() {
        width = Some((height.unwrap() as f64 * ratio).round() as u16);
    }

    if height.is_none() {
        height = Some((width.unwrap() as f64 / ratio).round() as u16);
    }

    (width.unwrap().into(), height.unwrap().into())

}