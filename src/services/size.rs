use crate::params::aspect_ratio::AspectRatio;
use crate::process::pipeline::request::PipelineRequest;
use picturium_libvips::VipsImage;
use tracing::debug;
use crate::enums::image_fit::ImageFit;
use crate::enums::upsize::Upsize;

pub fn calculate_processing_size(request: &PipelineRequest, image: &VipsImage) -> (i32, i32) {
    let original_width = image.get_width() as u16;
    let original_height = image.get_height() as u16;

    let (width, height) = (request.parameters.width, request.parameters.height);
    let (width, height) = set_missing_dimensions(
        request.parameters.aspect_ratio,
        (width, height),
        image,
    );

    let (width, height) = (
        width.unwrap_or(original_width),
        height.unwrap_or(original_height),
    );

    let (width, height) = match request.parameters.upsize {
        Upsize::True => (width, height),
        Upsize::False => clamp_dimensions(
            width, height,
            original_width, original_height
        ),
    };

    debug!("Calculated size: {}x{}", width, height);
    (width as i32, height as i32)
}

fn set_missing_dimensions(
    aspect_ratio: AspectRatio,
    (width, height): (Option<u16>, Option<u16>),
    image: &VipsImage,
) -> (Option<u16>, Option<u16>) {
    if width.is_some() && height.is_some() {
        return (width, height);
    }

    let (mut width, mut height) = (width, height);

    if width.is_none() && height.is_none() {
        width = Some(image.get_width() as u16);
    }

    let aspect_ratio = match aspect_ratio {
        AspectRatio::Auto => image.get_width() as f32 / image.get_height() as f32,
        AspectRatio::Value(ratio) => ratio,
    };

    if width.is_none() {
        width = Some((height.unwrap() as f32 * aspect_ratio).round() as u16);
    } else if height.is_none() {
        height = Some((width.unwrap() as f32 / aspect_ratio).round() as u16);
    }

    (width, height)
}

fn clamp_dimensions(
    requested_width: u16, requested_height: u16,
    original_width: u16, original_height: u16
) -> (u16, u16) {
    (requested_width.min(original_width), requested_height.min(original_height))
}

fn enable_fit_contain(request: &PipelineRequest, image: &VipsImage) -> bool {
    request.parameters.fit == ImageFit::Contain
        && request.parameters.width.is_some()
        && request.parameters.height.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_dimensions() {
        assert_eq!(clamp_dimensions(200, 200, 100, 100), (100, 100));
    }

    #[test]
    fn test_clamp_dimensions_smaller_always_allowed() {
        // Dimensions smaller than original should never be clamped
        assert_eq!(clamp_dimensions(50, 50, 100, 100), (50, 50));
    }

    #[test]
    fn test_clamp_dimensions_partial_clamping() {
        // Only clamp dimensions that exceed original
        assert_eq!(clamp_dimensions(200, 50, 100, 100), (100, 50));
        assert_eq!(clamp_dimensions(50, 200, 100, 100), (50, 100));
    }
}
