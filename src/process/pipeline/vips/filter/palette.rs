use crate::params::background::Background;
use crate::params::color::get_bit_depth_multiplier;
use anyhow::Result;
use picturium_libvips::{VipsBandFormat, VipsColors, VipsFilters, VipsImage, VipsOperations};
use tracing::debug;

const LUMA: [f64; 3] = [0.213, 0.715, 0.072];
const LUT_PIVOT: f64 = 128.0;

pub fn apply(
    image: VipsImage,
    palette: &Vec<Background>,
    intensity: f64,
    bit_depth: u8,
) -> Result<VipsImage> {
    if palette.is_empty() || palette.len() > 2 || intensity == 0.0 {
        return Ok(image);
    }

    match palette.len() {
        1 => apply_monotone(image, palette[0], intensity, bit_depth),
        _ => apply_duotone(image, palette[0], palette[1], intensity, bit_depth),
    }
}

fn apply_monotone(
    image: VipsImage,
    tone: Background,
    intensity: f64,
    bit_depth: u8,
) -> Result<VipsImage> {
    debug!("Monotone: tone={:?}, intensity={}", tone, intensity);

    let luma_image = get_luma_image(image.clone())?;
    let luma_image = luma_image
        .cast(VipsBandFormat::UChar)
        .map_err(|e| anyhow::anyhow!("Failed to cast luma image to UChar: {e:?}"))?;

    let lut = get_lut(tone, bit_depth)?;

    let effect = luma_image
        .map_lut(lut, None)
        .map_err(|e| anyhow::anyhow!("Failed to apply LUT to luma image: {e:?}"))?;

    let image = image
        .linear(&[1.0 - intensity; 3], &[0.0; 3])
        .map_err(|e| anyhow::anyhow!("Failed to apply linear filter: {e:?}"))?;

    let effect = effect
        .linear(&[intensity], &[0.0])
        .map_err(|e| anyhow::anyhow!("Failed to apply linear filter: {e:?}"))?;

    image
        .add(effect)
        .map_err(|e| anyhow::anyhow!("Failed to apply addition: {e:?}"))
}

fn apply_duotone(
    image: VipsImage,
    light: Background,
    dark: Background,
    intensity: f64,
    bit_depth: u8,
) -> Result<VipsImage> {
    debug!(
        "Duotone: light={:?}, dark={:?}, intensity={}",
        light, dark, intensity
    );

    let luma_image = get_luma_image(image.clone())?;

    let scale = [
        intensity * (light.red - dark.red),
        intensity * (light.green - dark.green),
        intensity * (light.blue - dark.blue),
    ];

    let multiplier = get_bit_depth_multiplier(bit_depth);

    let offset = [
        intensity * dark.red * multiplier,
        intensity * dark.green * multiplier,
        intensity * dark.blue * multiplier,
    ];

    let effect = luma_image
        .linear(&scale, &offset)
        .map_err(|e| anyhow::anyhow!("Failed to apply linear filter: {e:?}"))?;

    let image = image
        .linear(&[1.0 - intensity; 3], &[0.0; 3])
        .map_err(|e| anyhow::anyhow!("Failed to apply linear filter: {e:?}"))?;

    image
        .add(effect)
        .map_err(|e| anyhow::anyhow!("Failed to apply addition: {e:?}"))
}

fn get_luma_image(image: VipsImage) -> Result<VipsImage> {
    let matrix = VipsImage::new_matrix(3, 1, &LUMA)
        .map_err(|e| anyhow::anyhow!("Failed to build luma matrix: {e:?}"))?;

    image
        .recomb(matrix)
        .map_err(|e| anyhow::anyhow!("Failed to create luma image from matrix: {e:?}"))
}

fn get_lut(tone: Background, bit_depth: u8) -> Result<VipsImage> {
    let multiplier = get_bit_depth_multiplier(bit_depth);
    let mut lut_data = Vec::with_capacity((multiplier + 1.0) as usize * 3);
    let tone = tone.to_rgb_with_bit_depth(bit_depth);

    for value in 0..=(multiplier as u8) {
        let value = value as f64;

        let mapped = if value <= LUT_PIVOT {
            // black → selected color
            let t = value / LUT_PIVOT;
            [tone.0 * t, tone.1 * t, tone.2 * t]
        } else {
            // selected color → white
            let t = (value - LUT_PIVOT) / (multiplier - LUT_PIVOT);
            [
                tone.0 + t * (multiplier - tone.0),
                tone.1 + t * (multiplier - tone.1),
                tone.2 + t * (multiplier - tone.2),
            ]
        };

        for channel in mapped {
            lut_data.push(channel.round().clamp(0.0, multiplier) as u8);
        }
    }

    VipsImage::new_from_memory(&lut_data, (multiplier + 1.0) as i32, 1, 3)
        .map_err(|e| anyhow::anyhow!("Failed to create LUT image: {e:?}"))
}
