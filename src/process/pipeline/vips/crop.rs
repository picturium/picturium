use crate::enums::image_gravity::ImageGravity;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::size::calculate_crop_rectangle;
use anyhow::{Result, anyhow};
use picturium_libvips::{VipsCrop, VipsImage, VipsInteresting};

pub(super) fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    let Some(crop) = request.parameters.crop else {
        return Ok(image);
    };

    let source = (
        request.source.width.unwrap_or(1).max(1),
        request.source.height.unwrap_or(1).max(1),
    );

    let dimensions = image.get_dimensions();
    let rectangle = calculate_crop_rectangle(&crop, source);
    let (left, top, width, height) = to_image_coordinates(rectangle, source, dimensions);

    if (left, top) == (0, 0) && (width, height) == dimensions {
        return Ok(image);
    }

    match crop.gravity {
        Some(ImageGravity::Attention) => image.smartcrop(width, height, VipsInteresting::Attention),
        Some(ImageGravity::Entropy) => image.smartcrop(width, height, VipsInteresting::Entropy),
        _ => image.extract_area(left, top, width, height),
    }
    .map_err(|error| anyhow!("failed to crop image: {error}"))
}

fn to_image_coordinates(
    (left, top, width, height): (i32, i32, i32, i32),
    source: (u16, u16),
    image: (i32, i32),
) -> (i32, i32, i32, i32) {
    let scale = |value: i32, source: u16, image: i32| {
        (f64::from(value) * f64::from(image) / f64::from(source)).round() as i32
    };

    let width = scale(width, source.0, image.0).clamp(1, image.0);
    let height = scale(height, source.1, image.1).clamp(1, image.1);

    (
        scale(left, source.0, image.0).clamp(0, image.0 - width),
        scale(top, source.1, image.1).clamp(0, image.1 - height),
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_in_source_pixels_maps_into_a_shrunk_image() {
        // 4000x3000 source loaded at shrink 4, so a centred 1000x600 crop halves down.
        assert_eq!(
            to_image_coordinates((1500, 1200, 1000, 600), (4000, 3000), (1000, 750)),
            (375, 300, 250, 150),
        );
    }

    #[test]
    fn a_rectangle_stays_inside_the_image_after_rounding() {
        assert_eq!(
            to_image_coordinates((0, 0, 4000, 3000), (4000, 3000), (999, 749)),
            (0, 0, 999, 749),
        );
    }
}
