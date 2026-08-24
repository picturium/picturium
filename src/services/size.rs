use crate::enums::filter::FilterValue;
use crate::enums::image_fit::ImageFit;
use crate::enums::upsize::Upsize;
use crate::params::aspect_ratio::AspectRatio;
use crate::process::pipeline::request::PipelineRequest;
use picturium_libvips::VipsImage;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SizeGeometry {
    content: (u16, u16),
    canvas: Option<(u16, u16)>,
}

pub fn calculate_requested_size(request: &PipelineRequest) -> (u16, u16) {
    let geometry = geometry(request, source_dimensions(request));
    geometry.canvas.unwrap_or(geometry.content)
}

fn source_dimensions(request: &PipelineRequest) -> (u16, u16) {
    (
        request.source.width.unwrap_or(1).max(1),
        request.source.height.unwrap_or(1).max(1),
    )
}

fn geometry(request: &PipelineRequest, original: (u16, u16)) -> SizeGeometry {
    resolve_geometry(
        (request.parameters.width, request.parameters.height),
        request.parameters.aspect_ratio,
        request.parameters.scale * request.parameters.dpr,
        request.parameters.upsize.clone(),
        request.parameters.fit,
        original,
    )
}

fn resolve_geometry(
    (width, height): (Option<u16>, Option<u16>),
    aspect_ratio: AspectRatio,
    modifier: f32,
    upsize: Upsize,
    fit: ImageFit,
    (original_width, original_height): (u16, u16),
) -> SizeGeometry {
    let has_bounding_box = width.is_some() && height.is_some();
    let (width, height) = set_missing_dimensions(
        aspect_ratio,
        (width, height),
        (original_width, original_height),
    );

    let (width, height) = (
        width.unwrap_or(original_width),
        height.unwrap_or(original_height),
    );
    let (width, height) = apply_size_modifier(width, height, modifier);

    if has_bounding_box && matches!(fit, ImageFit::Contain | ImageFit::Cover) {
        let (width, height) = match upsize {
            Upsize::True => (width, height),
            Upsize::False => clamp_dimensions(width, height, original_width, original_height),
        };

        let horizontal_scale = width as f64 / original_width as f64;
        let vertical_scale = height as f64 / original_height as f64;
        let content_scale = match fit {
            ImageFit::Contain => horizontal_scale.min(vertical_scale),
            ImageFit::Cover => horizontal_scale.max(vertical_scale),
            ImageFit::Force => unreachable!("force fit is excluded above"),
        };

        let content = (
            scaled_dimension(original_width, content_scale),
            scaled_dimension(original_height, content_scale),
        );
        let output = match fit {
            ImageFit::Contain => (width, height),
            ImageFit::Cover => (width.min(content.0), height.min(content.1)),
            ImageFit::Force => unreachable!("force fit is excluded above"),
        };

        return SizeGeometry {
            content,
            canvas: Some(output),
        };
    }

    let content = match upsize {
        Upsize::True => (width, height),
        Upsize::False => clamp_dimensions(width, height, original_width, original_height),
    };

    SizeGeometry {
        content,
        canvas: None,
    }
}

fn apply_size_modifier(width: u16, height: u16, modifier: f32) -> (u16, u16) {
    (
        (width as f32 * modifier) as u16,
        (height as f32 * modifier) as u16,
    )
}

fn scaled_dimension(dimension: u16, scale: f64) -> u16 {
    ((dimension as f64 * scale).round() as u16).max(1)
}

fn canvas_size(request: &PipelineRequest) -> Option<(u16, u16)> {
    let (width, height) = geometry(request, source_dimensions(request)).canvas?;
    Some(apply_pixelize_filter(request, width, height))
}

pub(crate) fn calculate_contain_canvas_size(request: &PipelineRequest) -> Option<(i32, i32)> {
    if request.parameters.fit != ImageFit::Contain {
        return None;
    }

    let (width, height) = canvas_size(request)?;
    Some((i32::from(width), i32::from(height)))
}

pub(crate) fn calculate_cover_crop_size(
    request: &PipelineRequest,
    image: &VipsImage,
) -> Option<(i32, i32)> {
    if request.parameters.fit != ImageFit::Cover {
        return None;
    }

    let (width, height) = canvas_size(request)?;
    Some((
        i32::from(width).min(image.get_width()),
        i32::from(height).min(image.get_height()),
    ))
}

/// Target size for the resize step. Derived from the original source dimensions,
/// because the loaded image may already be shrunk on load.
pub fn calculate_processing_size(request: &PipelineRequest) -> (i32, i32) {
    processing_size(request, source_dimensions(request))
}

/// Target size relative to a not-yet-shrunk image, used by the loaders to pick
/// their shrink-on-load / render scale factor.
pub fn calculate_load_size(request: &PipelineRequest, image: &VipsImage) -> (i32, i32) {
    processing_size(request, (image.get_width() as u16, image.get_height() as u16))
}

fn processing_size(request: &PipelineRequest, original: (u16, u16)) -> (i32, i32) {
    let geometry = geometry(request, original);
    let (width, height) = geometry.content;
    let (width, height) = apply_pixelize_filter(request, width, height);

    debug!("Calculated size: {}x{}", width, height);
    (width as i32, height as i32)
}

fn set_missing_dimensions(
    aspect_ratio: AspectRatio,
    (width, height): (Option<u16>, Option<u16>),
    (original_width, original_height): (u16, u16),
) -> (Option<u16>, Option<u16>) {
    if width.is_some() && height.is_some() {
        return (width, height);
    }

    let (mut width, mut height) = (width, height);

    if width.is_none() && height.is_none() {
        width = Some(original_width);
    }

    let aspect_ratio = match aspect_ratio {
        AspectRatio::Auto => original_width as f32 / original_height as f32,
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
    requested_width: u16,
    requested_height: u16,
    original_width: u16,
    original_height: u16,
) -> (u16, u16) {
    (
        requested_width.min(original_width),
        requested_height.min(original_height),
    )
}

fn apply_pixelize_filter(request: &PipelineRequest, width: u16, height: u16) -> (u16, u16) {
    let filter = request
        .parameters
        .filter
        .0
        .iter()
        .find(|f| matches!(f, FilterValue::Pixelate(_)));

    match filter {
        Some(FilterValue::Pixelate(size)) => {
            let new_width = (width as f32 / *size as f32).round() as u16;
            let new_height = (height as f32 / *size as f32).round() as u16;

            (new_width.max(1), new_height.max(1))
        }
        _ => (width, height),
    }
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

    #[test]
    fn test_apply_size_modifier_scales_default_dimensions() {
        assert_eq!(apply_size_modifier(100, 50, 1.5), (150, 75));
    }

    #[test]
    fn test_default_dimensions_are_scaled_without_upsizing() {
        assert_eq!(
            resolve_geometry(
                (None, None),
                AspectRatio::Auto,
                0.5,
                Upsize::False,
                ImageFit::Cover,
                (100, 50),
            ),
            SizeGeometry {
                content: (50, 25),
                canvas: None,
            },
        );
    }

    #[test]
    fn test_default_dimensions_are_scaled_when_upsizing_is_enabled() {
        assert_eq!(
            resolve_geometry(
                (None, None),
                AspectRatio::Auto,
                2.0,
                Upsize::True,
                ImageFit::Cover,
                (100, 50),
            ),
            SizeGeometry {
                content: (200, 100),
                canvas: None,
            },
        );
    }

    #[test]
    fn contain_uses_the_smaller_axis_for_content() {
        assert_eq!(
            resolve_geometry(
                (Some(300), Some(300)),
                AspectRatio::Auto,
                1.0,
                Upsize::True,
                ImageFit::Contain,
                (400, 200),
            ),
            SizeGeometry {
                content: (300, 150),
                canvas: Some((300, 300)),
            },
        );
    }

    #[test]
    fn cover_uses_the_larger_axis_for_content() {
        assert_eq!(
            resolve_geometry(
                (Some(300), Some(300)),
                AspectRatio::Auto,
                1.0,
                Upsize::True,
                ImageFit::Cover,
                (400, 200),
            ),
            SizeGeometry {
                content: (600, 300),
                canvas: Some((300, 300)),
            },
        );
    }

    #[test]
    fn cover_without_upsizing_crops_to_available_content() {
        assert_eq!(
            resolve_geometry(
                (Some(300), Some(300)),
                AspectRatio::Auto,
                1.0,
                Upsize::False,
                ImageFit::Cover,
                (400, 200),
            ),
            SizeGeometry {
                content: (400, 200),
                canvas: Some((300, 200)),
            },
        );
    }

    #[test]
    fn contain_without_upsizing_does_not_pad_an_already_contained_image() {
        // Both requested dimensions exceed the original, so there is nothing to contain.
        assert_eq!(
            resolve_geometry(
                (Some(600), Some(800)),
                AspectRatio::Auto,
                1.0,
                Upsize::False,
                ImageFit::Contain,
                (400, 400),
            ),
            SizeGeometry {
                content: (400, 400),
                canvas: Some((400, 400)),
            },
        );
    }

    #[test]
    fn contain_without_upsizing_limits_each_axis_separately() {
        // The original caps only the height, the 400px width still applies.
        assert_eq!(
            resolve_geometry(
                (Some(400), Some(200)),
                AspectRatio::Auto,
                1.0,
                Upsize::False,
                ImageFit::Contain,
                (400, 400),
            ),
            SizeGeometry {
                content: (200, 200),
                canvas: Some((400, 200)),
            },
        );
    }

    #[test]
    fn contain_without_upsizing_applies_the_size_modifier_before_limiting() {
        assert_eq!(
            resolve_geometry(
                (Some(400), Some(300)),
                AspectRatio::Auto,
                2.0,
                Upsize::False,
                ImageFit::Contain,
                (100, 50),
            ),
            SizeGeometry {
                content: (100, 50),
                canvas: Some((100, 50)),
            },
        );
    }

    #[test]
    fn one_requested_dimension_does_not_create_a_contain_canvas() {
        assert_eq!(
            resolve_geometry(
                (Some(200), None),
                AspectRatio::Auto,
                1.0,
                Upsize::True,
                ImageFit::Contain,
                (400, 200),
            ),
            SizeGeometry {
                content: (200, 100),
                canvas: None,
            },
        );
    }
}
