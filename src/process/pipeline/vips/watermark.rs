use crate::enums::image_gravity::ImageGravity;
use crate::enums::watermark_position::WatermarkPosition;
use crate::params::padding::Padding;
use crate::params::watermark::{ResolvedWatermark, WatermarkSource};
use crate::process::pipeline::request::PipelineRequest;
use crate::services::size::gravity_offset;
use crate::process::source::Source;
use anyhow::{Context, Result, anyhow};
use picturium_libvips::{
    Composite2Options, EmbedOptions, FromFileOptions, TextOptions, VipsBlendMode, VipsColors,
    VipsExtend, VipsFilters, VipsImage, VipsInterpretation, VipsOperations, VipsText,
};

const TRANSPARENT: [f64; 4] = [0.0, 0.0, 0.0, 0.0];

pub(super) fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    let Some(watermark) = &request.parameters.watermark else {
        return Ok(image);
    };

    let page_height = image.get_page_height().max(1);
    let pages = (image.get_height() / page_height).max(1);
    let canvas = (image.get_width(), page_height);

    let inset = inset_canvas(canvas, watermark.padding)?;

    let mut overlay = build_overlay(request, watermark)?;
    overlay = fit_to_canvas(overlay, scale_limit(canvas, inset, watermark.max_scale))?;

    let (overlay, offset) = place(overlay, watermark, canvas, inset)?;
    let (overlay, offset) = stack_over_pages(overlay, offset, canvas, pages)?;

    image
        .composite2(
            overlay,
            VipsBlendMode::Over,
            Some(Composite2Options {
                compositing_space: VipsInterpretation::sRGB,
                x: offset.0,
                y: offset.1,
                ..Default::default()
            }),
        )
        .map_err(|error| anyhow!("failed to composite watermark: {error}"))
}

fn stack_over_pages(overlay: VipsImage, offset: (i32, i32), canvas: (i32, i32), pages: i32) -> Result<(VipsImage, (i32, i32))> {
    if pages <= 1 {
        return Ok((overlay, offset));
    }

    let page = pad(overlay, offset, canvas).context("failed to place watermark on page canvas")?;

    let stacked = page
        .replicate(1, pages)
        .map_err(|error| anyhow!("failed to repeat watermark across pages: {error}"))?;

    Ok((stacked, (0, 0)))
}

fn build_overlay(request: &PipelineRequest, watermark: &ResolvedWatermark) -> Result<VipsImage> {
    let overlay = match &watermark.source {
        WatermarkSource::Image {
            path,
            from_request,
            scale,
        } => image_overlay(request, path, *from_request, *scale)?,
        WatermarkSource::Text {
            text,
            font,
            size,
            color,
        } => text_overlay(text, font, *size, color)?,
    };

    let overlay = with_alpha(overlay)?;

    let overlay = if watermark.rotate % 360.0 == 0.0 {
        overlay
    } else {
        overlay.rotate_by(watermark.rotate, None).map_err(|error| anyhow!("failed to rotate watermark: {error}"))?
    };

    if watermark.opacity >= 100 {
        return Ok(overlay);
    }

    let bands = overlay.get_bands().max(1) as usize;
    let mut multipliers = vec![1.0; bands];
    multipliers[bands - 1] = f64::from(watermark.opacity) / 100.0;

    overlay.linear_uchar(&multipliers, &vec![0.0; bands]).map_err(|error| anyhow!("failed to apply watermark opacity: {error}"))
}

fn image_overlay(request: &PipelineRequest, path: &str, from_request: bool, scale: f32) -> Result<VipsImage> {
    let path = resolve_image_path(request, path, from_request)?;

    let overlay = VipsImage::new_from_file(
        &format!("{path}[revalidate=true]"),
        Some(FromFileOptions::default()),
    ).map_err(|error| anyhow!("failed to load watermark image: {error}"))?;

    let overlay = if (scale - 1.0).abs() < f32::EPSILON {
        overlay
    } else {
        overlay
            .resize(f64::from(scale), None)
            .map_err(|error| anyhow!("failed to scale watermark image: {error}"))?
    };

    Ok(overlay)
}

fn resolve_image_path(request: &PipelineRequest, path: &str, from_request: bool) -> Result<String> {
    if !from_request {
        return Ok(path.to_string());
    }

    let data_dir = &request.state.config.data.dir;
    let candidate = format!("{data_dir}/{path}");

    Source::get_path(&candidate, data_dir)
        .map(|path| path.to_string_lossy().into_owned())
        .ok_or_else(|| anyhow!("watermark image not found: {path}"))
}

fn text_overlay(text: &str, font: &str, size: u32, color: &crate::params::color::Color) -> Result<VipsImage> {
    let mask = VipsImage::text(
        text,
        Some(TextOptions {
            font: format!("{font} {size}px"),
            ..Default::default()
        }),
    ).map_err(|error| anyhow!("failed to render watermark text: {error}"))?;

    let (red, green, blue, _) = color.to_rgb_with_bit_depth(8);

    VipsImage::new_from_image(&mask, &[red, green, blue])
        .map_err(|error| anyhow!("failed to color watermark text: {error}"))?
        .with_interpretation(VipsInterpretation::sRGB)
        .map_err(|error| anyhow!("failed to tag watermark text overlay: {error}"))?
        .bandjoin2(mask)
        .map_err(|error| anyhow!("failed to build watermark text overlay: {error}"))
}

fn with_alpha(overlay: VipsImage) -> Result<VipsImage> {
    if overlay.is_transparent() {
        return Ok(overlay);
    }

    overlay.add_alpha().map_err(|error| anyhow!("failed to add watermark alpha channel: {error}"))
}

fn fit_to_canvas(overlay: VipsImage, canvas: (i32, i32)) -> Result<VipsImage> {
    let (width, height) = overlay.get_dimensions();

    if width <= canvas.0 && height <= canvas.1 {
        return Ok(overlay);
    }

    let scale = (f64::from(canvas.0) / f64::from(width)).min(f64::from(canvas.1) / f64::from(height));

    overlay.resize(scale, None).map_err(|error| anyhow!("failed to shrink watermark to fit: {error}"))
}

fn scale_limit(canvas: (i32, i32), inset: (i32, i32), max_scale: f32) -> (i32, i32) {
    let cap = |side: i32| ((side as f32 * max_scale).round() as i32).max(1);

    (inset.0.min(cap(canvas.0)), inset.1.min(cap(canvas.1)))
}

fn inset_canvas(canvas: (i32, i32), padding: Padding) -> Result<(i32, i32)> {
    let inset = (
        canvas.0 - horizontal(padding)?,
        canvas.1 - vertical(padding)?,
    );

    if inset.0 <= 0 || inset.1 <= 0 {
        return Err(anyhow!(
            "watermark padding leaves no room on a {}x{} image",
            canvas.0,
            canvas.1
        ));
    }

    Ok(inset)
}

fn place(overlay: VipsImage, watermark: &ResolvedWatermark, canvas: (i32, i32), inset: (i32, i32)) -> Result<(VipsImage, (i32, i32))> {
    if watermark.anchor == WatermarkPosition::Repeat {
        return Ok((tile(overlay, watermark.padding, canvas)?, (0, 0)));
    }

    let padding = watermark.padding;

    let offset = gravity_offset(gravity(watermark.anchor), inset, overlay.get_dimensions());

    let left = i32::try_from(padding.left).context("watermark padding exceeds libvips limits")?;
    let top = i32::try_from(padding.top).context("watermark padding exceeds libvips limits")?;

    Ok((overlay, (offset.0 + left, offset.1 + top)))
}

fn tile(overlay: VipsImage, padding: Padding, canvas: (i32, i32)) -> Result<VipsImage> {
    let (width, height) = overlay.get_dimensions();
    let left = i32::try_from(padding.left).context("watermark padding exceeds libvips limits")?;
    let top = i32::try_from(padding.top).context("watermark padding exceeds libvips limits")?;
    let tile = (width + horizontal(padding)?, height + vertical(padding)?);

    let padded = pad(overlay, (left, top), tile).context("failed to pad watermark tile")?;

    let tiled = padded.replicate(repeats(canvas.0, tile.0), repeats(canvas.1, tile.1))
        .map_err(|error| anyhow!("failed to repeat watermark: {error}"))?;

    pad(tiled, (0, 0), canvas).context("failed to trim repeated watermark")
}

fn pad(overlay: VipsImage, offset: (i32, i32), dimensions: (i32, i32)) -> Result<VipsImage> {
    if offset == (0, 0) && dimensions == overlay.get_dimensions() {
        return Ok(overlay);
    }

    overlay
        .embed(
            offset.0,
            offset.1,
            dimensions.0,
            dimensions.1,
            Some(EmbedOptions {
                extend: VipsExtend::Background,
                background: &TRANSPARENT,
            }),
        )
        .map_err(|error| anyhow!("libvips embed failed: {error}"))
}

fn repeats(canvas: i32, tile: i32) -> i32 {
    if tile <= 0 {
        return 1;
    }

    ((canvas + tile - 1) / tile).max(1)
}

fn horizontal(padding: Padding) -> Result<i32> {
    let total = padding.left.checked_add(padding.right).context("horizontal watermark padding overflow")?;

    i32::try_from(total).context("horizontal watermark padding exceeds libvips limits")
}

fn vertical(padding: Padding) -> Result<i32> {
    let total = padding.top.checked_add(padding.bottom).context("vertical watermark padding overflow")?;

    i32::try_from(total).context("vertical watermark padding exceeds libvips limits")
}

fn gravity(anchor: WatermarkPosition) -> ImageGravity {
    match anchor {
        WatermarkPosition::Top => ImageGravity::Top,
        WatermarkPosition::Right => ImageGravity::Right,
        WatermarkPosition::Bottom => ImageGravity::Bottom,
        WatermarkPosition::Left => ImageGravity::Left,
        WatermarkPosition::TopLeft => ImageGravity::TopLeft,
        WatermarkPosition::TopRight => ImageGravity::TopRight,
        WatermarkPosition::BottomLeft => ImageGravity::BottomLeft,
        WatermarkPosition::BottomRight => ImageGravity::BottomRight,
        WatermarkPosition::Center | WatermarkPosition::Repeat => ImageGravity::Center,
    }
}
