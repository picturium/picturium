mod blur;
mod hue;
mod invert;
mod linear;
mod palette;
mod pixelate;
mod saturate;
mod sepia;
mod sharpen;

use crate::enums::filter::FilterValue;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use picturium_libvips::{VipsBandFormat, VipsImage};

pub fn process(request: &PipelineRequest, mut image: VipsImage) -> Result<VipsImage> {
    let bit_depth = image.get_bit_depth();
    let divider = (1 << bit_depth) as f64;
    let has_clamp = matches!(bit_depth, 8 | 16);

    let filters = &request.parameters.filter.0;
    let last_index = filters.len().saturating_sub(1);

    for (index, filter) in filters.iter().enumerate() {
        image = match filter {
            FilterValue::Brightness(value) => linear::apply(image, *value, 0.0)?,
            FilterValue::Contrast(value) => {
                linear::apply(image, *value, (0.5 - value * 0.5) * divider)?
            }
            FilterValue::Saturate(value) => saturate::apply(image, *value)?,
            FilterValue::Hue(value) => hue::apply(image, *value as f64)?,
            FilterValue::Grayscale(value) => saturate::apply(image, (value * -1.0) + 1.0)?,
            FilterValue::Sepia(value) => sepia::apply(image, (value * -1.0) + 1.0)?,
            FilterValue::Invert(value) => invert::apply(image, *value)?,
            FilterValue::Blur(value) => blur::apply(image, *value)?,
            FilterValue::Sharpen(value) => sharpen::apply(image, *value)?,
            FilterValue::Pixelate(value) => pixelate::apply(request, image, *value)?,
            FilterValue::Palette(value) => palette::apply(image, &value.1, value.0, bit_depth)?,
        };

        if has_clamp && index < last_index {
            let format = match bit_depth {
                8 => VipsBandFormat::UChar,
                16 => VipsBandFormat::UShort,
                _ => unreachable!(),
            };

            image = image.cast(format).map_err(|e| {
                anyhow::anyhow!("Failed to clamp intermediate filter result: {:?}", e)
            })?;
        }
    }

    Ok(image)
}
