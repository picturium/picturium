use crate::enums::filter::FilterValue;
use crate::enums::image_fit::ImageFit;
use crate::enums::image_gravity::ImageGravity;
use crate::enums::upsize::Upsize;
use crate::params::aspect_ratio::AspectRatio;
use crate::params::crop::Crop;
use crate::params::limits::Dimension;
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
    let geometry = resolve_geometry(
        (request.parameters.width, request.parameters.height),
        request.parameters.aspect_ratio,
        request.parameters.scale * request.parameters.dpr,
        request.parameters.upsize.clone(),
        request.parameters.fit,
        original,
    );

    limit_geometry(geometry, request.parameters.limits.dimension.unwrap_or_default())
}

fn limit_geometry(geometry: SizeGeometry, limit: Dimension) -> SizeGeometry {
    let (width, height) = geometry.canvas.unwrap_or(geometry.content);

    let axis = |limit: Option<u16>, value: u16| match limit {
        Some(limit) if value > limit => Some(limit as f64 / value as f64),
        _ => None,
    };

    let scale = match (axis(limit.width, width), axis(limit.height, height)) {
        (Some(horizontal), Some(vertical)) => horizontal.min(vertical),
        (Some(scale), None) | (None, Some(scale)) => scale,
        (None, None) => return geometry,
    };

    SizeGeometry {
        content: (
            scaled_dimension(geometry.content.0, scale),
            scaled_dimension(geometry.content.1, scale),
        ),
        canvas: geometry
            .canvas
            .map(|(width, height)| (scaled_dimension(width, scale), scaled_dimension(height, scale))),
    }
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
    let original = (image.get_width() as u16, image.get_height() as u16);

    let Some(crop) = request.parameters.crop else {
        return processing_size(request, original);
    };

    let (_, _, crop_width, crop_height) = calculate_crop_rectangle(&crop, original);
    let target = processing_size(request, (crop_width as u16, crop_height as u16));

    let output = &request.state.config.output;
    apply_crop(
        target,
        original,
        (crop_width, crop_height),
        (output.max_width, output.max_height),
    )
}

fn apply_crop(
    (width, height): (i32, i32),
    original: (u16, u16),
    crop: (i32, i32),
    limit: (u32, u32),
) -> (i32, i32) {
    let axis = |size: i32, original: u16, crop: i32, limit: u32| {
        let size = (f64::from(size) * f64::from(original) / f64::from(crop)).round() as i32;

        match i32::try_from(limit).ok().filter(|limit| *limit > 0) {
            Some(limit) => size.min(limit),
            None => size,
        }
        .max(1)
    };

    (
        axis(width, original.0, crop.0, limit.0),
        axis(height, original.1, crop.1, limit.1),
    )
}

pub(crate) fn calculate_crop_rectangle(crop: &Crop, source: (u16, u16)) -> (i32, i32, i32, i32) {
    let (source_width, source_height) = (source.0.max(1), source.1.max(1));

    let ratio = match crop.aspect_ratio {
        Some(AspectRatio::Value(ratio)) => Some(ratio),
        _ => None,
    };

    let (width, height) = match (crop.width, crop.height, ratio) {
        (Some(width), Some(height), _) => (width, height),
        (Some(width), None, Some(ratio)) => (width, (width as f32 / ratio).round() as u16),
        (None, Some(height), Some(ratio)) => ((height as f32 * ratio).round() as u16, height),
        (Some(width), None, None) => (width, source_height),
        (None, Some(height), None) => (source_width, height),
        (None, None, Some(ratio)) => largest_area(ratio, (source_width, source_height)),
        (None, None, None) => (source_width, source_height),
    };

    let (source_width, source_height) = (i32::from(source_width), i32::from(source_height));
    let width = i32::from(width).clamp(1, source_width);
    let height = i32::from(height).clamp(1, source_height);

    let (left, top) = gravity_offset(
        crop.gravity.unwrap_or_default(),
        (source_width, source_height),
        (width, height),
    );

    (
        (left + i32::from(crop.x.unwrap_or(0))).clamp(0, source_width - width),
        (top + i32::from(crop.y.unwrap_or(0))).clamp(0, source_height - height),
        width,
        height,
    )
}

fn largest_area(ratio: f32, (width, height): (u16, u16)) -> (u16, u16) {
    match width as f32 / height as f32 > ratio {
        true => ((height as f32 * ratio).round() as u16, height),
        false => (width, (width as f32 / ratio).round() as u16),
    }
}

pub(crate) fn gravity_offset(
    gravity: ImageGravity,
    outer: (i32, i32),
    inner: (i32, i32),
) -> (i32, i32) {
    let remaining_x = outer.0.saturating_sub(inner.0).max(0);
    let remaining_y = outer.1.saturating_sub(inner.1).max(0);
    let center = (remaining_x / 2, remaining_y / 2);

    match gravity {
        ImageGravity::Top => (center.0, 0),
        ImageGravity::Right => (remaining_x, center.1),
        ImageGravity::Bottom => (center.0, remaining_y),
        ImageGravity::Left => (0, center.1),
        ImageGravity::TopLeft => (0, 0),
        ImageGravity::TopRight => (remaining_x, 0),
        ImageGravity::BottomLeft => (0, remaining_y),
        ImageGravity::BottomRight => (remaining_x, remaining_y),
        ImageGravity::Center | ImageGravity::Attention | ImageGravity::Entropy => center,
    }
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

    #[test]
    fn dimension_limit_scales_content_and_canvas_together() {
        let geometry = SizeGeometry { content: (800, 400), canvas: Some((400, 400)) };

        assert_eq!(
            limit_geometry(geometry, Dimension { width: Some(200), height: None }),
            SizeGeometry { content: (400, 200), canvas: Some((200, 200)) },
        );
        assert_eq!(
            limit_geometry(geometry, Dimension { width: Some(200), height: Some(100) }),
            SizeGeometry { content: (200, 100), canvas: Some((100, 100)) },
        );
        assert_eq!(
            limit_geometry(geometry, Dimension { width: Some(800), height: Some(800) }),
            geometry,
        );
    }

    fn crop(definition: &str) -> Crop {
        definition.parse().unwrap()
    }

    #[test]
    fn basic_gravity_offsets_position_content_in_all_nine_directions() {
        let outer = (100, 80);
        let inner = (40, 20);

        assert_eq!(gravity_offset(ImageGravity::Top, outer, inner), (30, 0));
        assert_eq!(gravity_offset(ImageGravity::Right, outer, inner), (60, 30));
        assert_eq!(gravity_offset(ImageGravity::Bottom, outer, inner), (30, 60));
        assert_eq!(gravity_offset(ImageGravity::Left, outer, inner), (0, 30));
        assert_eq!(gravity_offset(ImageGravity::TopLeft, outer, inner), (0, 0));
        assert_eq!(gravity_offset(ImageGravity::TopRight, outer, inner), (60, 0));
        assert_eq!(gravity_offset(ImageGravity::BottomLeft, outer, inner), (0, 60));
        assert_eq!(gravity_offset(ImageGravity::BottomRight, outer, inner), (60, 60));
        assert_eq!(gravity_offset(ImageGravity::Center, outer, inner), (30, 30));
        assert_eq!(gravity_offset(ImageGravity::Attention, outer, inner), (30, 30));
        assert_eq!(gravity_offset(ImageGravity::Entropy, outer, inner), (30, 30));
    }

    #[test]
    fn an_inner_box_larger_than_the_outer_one_is_pinned_to_the_corner() {
        assert_eq!(gravity_offset(ImageGravity::BottomRight, (40, 20), (100, 80)), (0, 0));
    }

    #[test]
    fn a_crop_area_is_placed_by_gravity() {
        assert_eq!(
            calculate_crop_rectangle(&crop("w:400|h:200|g:center"), (1000, 600)),
            (300, 200, 400, 200),
        );
        assert_eq!(
            calculate_crop_rectangle(&crop("w:400|h:200|g:top-left"), (1000, 600)),
            (0, 0, 400, 200),
        );
        assert_eq!(
            calculate_crop_rectangle(&crop("w:400|h:200|g:bottom-right"), (1000, 600)),
            (600, 400, 400, 200),
        );
    }

    #[test]
    fn a_single_crop_dimension_spans_the_other_axis_without_an_aspect_ratio() {
        assert_eq!(
            calculate_crop_rectangle(&crop("w:400"), (1000, 600)),
            (300, 0, 400, 600),
        );
        assert_eq!(
            calculate_crop_rectangle(&crop("h:200"), (1000, 600)),
            (0, 200, 1000, 200),
        );
    }

    #[test]
    fn a_crop_aspect_ratio_derives_the_missing_dimension() {
        assert_eq!(
            calculate_crop_rectangle(&crop("w:400|ar:2/1"), (1000, 600)),
            (300, 200, 400, 200),
        );
        assert_eq!(
            calculate_crop_rectangle(&crop("h:200|ar:2/1"), (1000, 600)),
            (300, 200, 400, 200),
        );
    }

    #[test]
    fn a_crop_aspect_ratio_alone_takes_the_largest_matching_area() {
        // Source is wider than 1:1, so the height is the constraint.
        assert_eq!(
            calculate_crop_rectangle(&crop("ar:square"), (1000, 600)),
            (200, 0, 600, 600),
        );
        // Source is taller than 16:9, so the width is the constraint.
        assert_eq!(
            calculate_crop_rectangle(&crop("ar:video"), (1000, 1000)),
            (0, 218, 1000, 563),
        );
    }

    #[test]
    fn a_crop_area_larger_than_the_source_is_clamped_to_it() {
        assert_eq!(
            calculate_crop_rectangle(&crop("w:9000|h:9000"), (1000, 600)),
            (0, 0, 1000, 600),
        );
    }

    #[test]
    fn crop_offsets_shift_from_the_gravity_and_clamp_at_the_edges() {
        assert_eq!(
            calculate_crop_rectangle(&crop("w:400|h:200|x:100|y:-50"), (1000, 600)),
            (400, 150, 400, 200),
        );
        assert_eq!(
            calculate_crop_rectangle(&crop("w:400|h:200|x:9000|y:-9000"), (1000, 600)),
            (600, 0, 400, 200),
        );
    }

    #[test]
    fn a_crop_scales_the_load_size_up_so_the_region_reaches_the_target() {
        // 4000x3000 source, a 500x500 crop asked for at 500x500 output: loading the
        // whole canvas at 500x375 would leave the region at 62px.
        assert_eq!(
            apply_crop((500, 500), (4000, 3000), (500, 500), (0, 0)),
            (4000, 3000),
        );
    }

    #[test]
    fn the_compensated_load_size_is_capped_at_the_output_limits() {
        assert_eq!(
            apply_crop((500, 500), (40000, 30000), (1, 1), (5000, 5000)),
            (5000, 5000),
        );
    }

    #[test]
    fn dimension_parsing_supports_both_axes() {
        use std::str::FromStr;

        assert_eq!(Dimension::from_str("100").unwrap(), Dimension { width: Some(100), height: Some(100) });
        assert_eq!(Dimension::from_str("100x50").unwrap(), Dimension { width: Some(100), height: Some(50) });
        assert_eq!(Dimension::from_str("x50").unwrap(), Dimension { width: None, height: Some(50) });
        assert_eq!(Dimension::from_str("100x").unwrap(), Dimension { width: Some(100), height: None });
        assert!(Dimension::from_str("abc").is_err());
    }
}
