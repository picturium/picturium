use crate::params::background::Background;
use serde::Deserialize;
use std::str::FromStr;
use crate::params::filter::FilterParseError;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterValue {
    Brightness(f64),
    Contrast(f64),
    Saturate(f64),
    Hue(f32),
    Grayscale(f64),
    Sepia(f64),
    Invert(f64),
    Blur(u16),
    Sharpen(f64),
    Pixelate(u16),
    Palette((f64, Vec<Background>)),
}

impl From<&str> for FilterValue {
    fn from(s: &str) -> Self {
        s.parse().unwrap()
    }
}

impl FromStr for FilterValue {
    type Err = FilterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s.split_once(':').ok_or_else(|| FilterParseError(format!("Missing ':' in filter segment '{s}'")))?;

        Ok(match key.trim().to_ascii_lowercase().as_str() {
            "brightness" => FilterValue::Brightness(parse_brightness(value)?),
            "contrast" => FilterValue::Contrast(parse_contrast(value)?),
            "saturate" => FilterValue::Saturate(parse_saturate(value)?),
            "hue" => FilterValue::Hue(parse_hue(value)?),
            "grayscale" => FilterValue::Grayscale(parse_grayscale(value)?),
            "sepia" => FilterValue::Sepia(parse_sepia(value)?),
            "invert" => FilterValue::Invert(parse_invert(value)?),
            "blur" => FilterValue::Blur(parse_blur(value)?),
            "sharpen" => FilterValue::Sharpen(parse_sharpen(value)?),
            "pixelate" => FilterValue::Pixelate(parse_pixelate(value)?),
            "palette" => FilterValue::Palette(parse_palette(value)?),
            _ => return Err(FilterParseError(format!("Unknown filter key: '{s}'"))),
        })
    }
}

fn parse_brightness(value: &str) -> Result<f64, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid brightness value: '{value}'")))?;

    if value < 0.0 {
        return Err(FilterParseError(format!("Brightness value must be a number greater than 0: '{value}'")));
    }

    Ok(value)
}

fn parse_contrast(value: &str) -> Result<f64, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid contrast value: '{value}'")))?;

    if value < 0.0 {
        return Err(FilterParseError(format!("Contrast value must be a number greater than 0: '{value}'")));
    }

    Ok(value)
}

fn parse_saturate(value: &str) -> Result<f64, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid saturate value: '{value}'")))?;

    if value < 0.0 {
        return Err(FilterParseError(format!("Saturate value must be a number greater than 0: '{value}'")));
    }

    Ok(value)
}

fn parse_hue(value: &str) -> Result<f32, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid hue value: '{value}'")))?;

    if value < 0.0 || value > 360.0 {
        return Err(FilterParseError(format!("Hue value must be a number between 0 and 360: '{value}'")));
    }

    Ok(value)
}

fn parse_grayscale(value: &str) -> Result<f64, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid grayscale value: '{value}'")))?;

    if value < 0.0 || value > 1.0 {
        return Err(FilterParseError(format!("Grayscale value must be a number between 0 and 1: '{value}'")));
    }

    Ok(value)
}

fn parse_sepia(value: &str) -> Result<f64, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid sepia value: '{value}'")))?;

    if value < 0.0 || value > 1.0 {
        return Err(FilterParseError(format!("Sepia value must be a number between 0 and 1: '{value}'")));
    }

    Ok(value)
}

fn parse_invert(value: &str) -> Result<f64, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid invert value: '{value}'")))?;

    if value < 0.0 || value > 1.0 {
        return Err(FilterParseError(format!("Invert value must be a number between 0 and 1: '{value}'")));
    }

    Ok(value)
}

fn parse_blur(value: &str) -> Result<u16, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid blur value: '{value}'")))?;

    Ok(value)
}

fn parse_sharpen(value: &str) -> Result<f64, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid sharpen value: '{value}'")))?;

    if value < 0.0 || value > 10.0 {
        return Err(FilterParseError(format!("Sharpen value must be a number between 0 and 10: '{value}'")));
    }

    Ok(value)
}

fn parse_pixelate(value: &str) -> Result<u16, FilterParseError> {
    let value = value.parse().map_err(|_| FilterParseError(format!("Invalid sharpen value: '{value}'")))?;

    if value < 1 {
        return Err(FilterParseError(format!("Pixelate value must be a number greater than 0: '{value}'")));
    }

    Ok(value)
}

fn parse_palette(value: &str) -> Result<(f64, Vec<Background>), FilterParseError> {
    let parts: Vec<&str> = value.split(',').collect();
    let with_intensity = parts.last().is_some() && parts.last().unwrap().parse::<f64>().is_ok();

    if (with_intensity && parts.len() > 3) || (!with_intensity && parts.len() > 2) {
        return Err(FilterParseError(format!("Palette can have at most 2 colors (monotone or duotone): '{value}'")));
    }

    let intensity: f64 = if with_intensity {
        parts.last().unwrap().parse()
            .map_err(|_| FilterParseError(format!("Invalid palette intensity: '{}'", parts.last().unwrap())))?
    } else {
        1.0
    };

    let intensity = intensity.max(0.0);

    let colors = if with_intensity {
        parts[..parts.len() - 1].to_vec()
    } else {
        parts.to_vec()
    };

    let colors = colors
        .into_iter()
        .map(|s| s.parse().map_err(|_| FilterParseError(format!("Invalid palette color: '{s}'"))))
        .collect::<Result<Vec<Background>, FilterParseError>>()?;

    Ok((intensity, colors))
}