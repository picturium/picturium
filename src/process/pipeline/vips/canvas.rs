use crate::enums::image_extend::ImageExtend;
use crate::enums::image_gravity::ImageGravity;
use crate::params::padding::Padding;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::services::size::{calculate_contain_canvas_size, calculate_cover_crop_size};
use anyhow::{Context, Result, anyhow};
use picturium_libvips::gravity::VipsGravity;
use picturium_libvips::{
    EmbedOptions, VipsColors, VipsCompassDirection, VipsImage, VipsInterpretation, VipsOperations,
};

pub(super) fn process(request: &PipelineRequest, mut image: VipsImage) -> Result<VipsImage> {
    if let Some((width, height)) = calculate_cover_crop_size(request, &image)
        && (width, height) != image.get_dimensions()
    {
        image = image
            .with_gravity(
                compass_direction(request.parameters.gravity),
                width,
                height,
                None,
            )
            .map_err(|error| anyhow!("failed to crop cover image: {error}"))?;
    }

    if let Some(canvas) = calculate_contain_canvas_size(request) {
        let offset = gravity_offset(request.parameters.gravity, canvas, image.get_dimensions())?;
        image = embed(
            image,
            offset,
            canvas,
            request.parameters.extend,
            request.parameters.background,
        )
        .context("failed to create contain canvas")?;
    }

    if let Some(padding) = request.parameters.padding {
        let dimensions = padded_dimensions(image.get_dimensions(), padding)?;
        let offset = (
            i32::try_from(padding.left).context("left padding exceeds libvips limits")?,
            i32::try_from(padding.top).context("top padding exceeds libvips limits")?,
        );
        image = embed(
            image,
            offset,
            dimensions,
            request.parameters.extend,
            request.parameters.background,
        )
        .context("failed to apply image padding")?;
    }

    Ok(image)
}

fn compass_direction(gravity: ImageGravity) -> VipsCompassDirection {
    match gravity {
        ImageGravity::Top => VipsCompassDirection::North,
        ImageGravity::Right => VipsCompassDirection::East,
        ImageGravity::Bottom => VipsCompassDirection::South,
        ImageGravity::Left => VipsCompassDirection::West,
        ImageGravity::TopLeft => VipsCompassDirection::NorthWest,
        ImageGravity::TopRight => VipsCompassDirection::NorthEast,
        ImageGravity::BottomLeft => VipsCompassDirection::SouthWest,
        ImageGravity::BottomRight => VipsCompassDirection::SouthEast,
        ImageGravity::Center | ImageGravity::Attention | ImageGravity::Entropy => {
            VipsCompassDirection::Centre
        }
    }
}

fn embed(
    image: VipsImage,
    offset: (i32, i32),
    dimensions: (i32, i32),
    extend: ImageExtend,
    background: Option<crate::params::background::Background>,
) -> Result<VipsImage> {
    if offset == (0, 0) && dimensions == image.get_dimensions() {
        return Ok(image);
    }

    let rgba = resolve_background(background);
    let image = prepare_for_background(image, extend, rgba[3])?;
    let background = background_for_bands(rgba, image.get_bands());

    image
        .embed(
            offset.0,
            offset.1,
            dimensions.0,
            dimensions.1,
            Some(EmbedOptions {
                extend: extend.into(),
                background: &background,
            }),
        )
        .map_err(|error| anyhow!("libvips embed failed: {error}"))
}

fn prepare_for_background(
    mut image: VipsImage,
    extend: ImageExtend,
    alpha: f64,
) -> Result<VipsImage> {
    if extend != ImageExtend::Bg {
        return Ok(image);
    }

    if matches!(
        image.get_interpretation(),
        VipsInterpretation::BlackWhite | VipsInterpretation::GREY16 | VipsInterpretation::CMYK
    ) {
        image = image
            .set_colorspace(VipsInterpretation::sRGB)
            .map_err(|error| anyhow!("failed to convert image to sRGB for background: {error}"))?;
    }

    if alpha < 255.0 && !image.is_transparent() {
        image = image
            .add_alpha()
            .map_err(|error| anyhow!("failed to add alpha channel for background: {error}"))?;
    }

    Ok(image)
}

fn background_for_bands(rgba: [f64; 4], bands: i32) -> Vec<f64> {
    let luma = rgba[0] * 0.2126 + rgba[1] * 0.7152 + rgba[2] * 0.0722;

    match bands {
        0 | 1 => vec![luma],
        2 => vec![luma, rgba[3]],
        3 => rgba[..3].to_vec(),
        _ => rgba.to_vec(),
    }
}

fn gravity_offset(
    gravity: ImageGravity,
    outer: (i32, i32),
    inner: (i32, i32),
) -> Result<(i32, i32)> {
    let remaining_x = outer
        .0
        .checked_sub(inner.0)
        .filter(|value| *value >= 0)
        .ok_or_else(|| {
            anyhow!(
                "contain canvas width {} is smaller than content width {}",
                outer.0,
                inner.0
            )
        })?;
    let remaining_y = outer
        .1
        .checked_sub(inner.1)
        .filter(|value| *value >= 0)
        .ok_or_else(|| {
            anyhow!(
                "contain canvas height {} is smaller than content height {}",
                outer.1,
                inner.1
            )
        })?;

    let center = (remaining_x / 2, remaining_y / 2);
    Ok(match gravity {
        ImageGravity::Top => (center.0, 0),
        ImageGravity::Right => (remaining_x, center.1),
        ImageGravity::Bottom => (center.0, remaining_y),
        ImageGravity::Left => (0, center.1),
        ImageGravity::TopLeft => (0, 0),
        ImageGravity::TopRight => (remaining_x, 0),
        ImageGravity::BottomLeft => (0, remaining_y),
        ImageGravity::BottomRight => (remaining_x, remaining_y),
        ImageGravity::Center | ImageGravity::Attention | ImageGravity::Entropy => center,
    })
}

fn padded_dimensions(dimensions: (i32, i32), padding: Padding) -> Result<(i32, i32)> {
    let horizontal = padding
        .left
        .checked_add(padding.right)
        .context("horizontal padding overflow")?;
    let vertical = padding
        .top
        .checked_add(padding.bottom)
        .context("vertical padding overflow")?;
    let horizontal =
        i32::try_from(horizontal).context("horizontal padding exceeds libvips limits")?;
    let vertical = i32::try_from(vertical).context("vertical padding exceeds libvips limits")?;

    Ok((
        dimensions
            .0
            .checked_add(horizontal)
            .context("padded image width overflow")?,
        dimensions
            .1
            .checked_add(vertical)
            .context("padded image height overflow")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_gravity_offsets_position_content_in_all_nine_directions() {
        let outer = (100, 80);
        let inner = (40, 20);

        assert_eq!(
            gravity_offset(ImageGravity::Top, outer, inner).unwrap(),
            (30, 0)
        );
        assert_eq!(
            gravity_offset(ImageGravity::Right, outer, inner).unwrap(),
            (60, 30)
        );
        assert_eq!(
            gravity_offset(ImageGravity::Bottom, outer, inner).unwrap(),
            (30, 60)
        );
        assert_eq!(
            gravity_offset(ImageGravity::Left, outer, inner).unwrap(),
            (0, 30)
        );
        assert_eq!(
            gravity_offset(ImageGravity::TopLeft, outer, inner).unwrap(),
            (0, 0)
        );
        assert_eq!(
            gravity_offset(ImageGravity::TopRight, outer, inner).unwrap(),
            (60, 0)
        );
        assert_eq!(
            gravity_offset(ImageGravity::BottomLeft, outer, inner).unwrap(),
            (0, 60)
        );
        assert_eq!(
            gravity_offset(ImageGravity::BottomRight, outer, inner).unwrap(),
            (60, 60)
        );
        assert_eq!(
            gravity_offset(ImageGravity::Center, outer, inner).unwrap(),
            (30, 30)
        );
        assert_eq!(
            gravity_offset(ImageGravity::Attention, outer, inner).unwrap(),
            (30, 30)
        );
        assert_eq!(
            gravity_offset(ImageGravity::Entropy, outer, inner).unwrap(),
            (30, 30)
        );
    }

    #[test]
    fn cover_crop_maps_image_gravity_to_libvips_compass_direction() {
        assert!(matches!(
            compass_direction(ImageGravity::TopLeft),
            VipsCompassDirection::NorthWest
        ));
        assert!(matches!(
            compass_direction(ImageGravity::BottomRight),
            VipsCompassDirection::SouthEast
        ));
        assert!(matches!(
            compass_direction(ImageGravity::Center),
            VipsCompassDirection::Centre
        ));
    }

    #[test]
    fn asymmetric_padding_increases_dimensions_by_each_side() {
        assert_eq!(
            padded_dimensions(
                (100, 50),
                Padding {
                    top: 2,
                    right: 3,
                    bottom: 5,
                    left: 7,
                },
            )
            .unwrap(),
            (110, 57),
        );
    }

    #[test]
    fn padded_dimensions_report_overflow() {
        let error = padded_dimensions(
            (i32::MAX, 10),
            Padding {
                right: 1,
                ..Default::default()
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("padded image width overflow"));
    }
}
