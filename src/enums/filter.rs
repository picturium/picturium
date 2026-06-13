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
    Bw(bool),
    Palette(Vec<Background>),
    Invert(bool),
    Sepia(bool),
    Blur(f32),
    Sharpen(f32),
    Pixelate(f32),
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
            "brightness" => FilterValue::Brightness(value.parse().map_err(|_| FilterParseError(format!("Invalid brightness value: '{value}'")))?),
            "contrast" => FilterValue::Contrast(value.parse().map_err(|_| FilterParseError(format!("Invalid contrast value: '{value}'")))?),
            "saturate" => FilterValue::Saturate(value.parse().map_err(|_| FilterParseError(format!("Invalid saturate value: '{value}'")))?),
            "hue" => FilterValue::Hue(value.parse().map_err(|_| FilterParseError(format!("Invalid hue value: '{value}'")))?),
            "bw" => FilterValue::Bw(value.parse().map_err(|_| FilterParseError(format!("Invalid bw value: '{value}'")))?),
            "palette" => FilterValue::Palette(value.split(',').map(|s| s.parse().map_err(|_| FilterParseError(format!("Invalid palette color: '{s}'")))).collect::<Result<Vec<_>, _>>()?),
            "invert" => FilterValue::Invert(value.parse().map_err(|_| FilterParseError(format!("Invalid invert value: '{value}'")))?),
            "sepia" => FilterValue::Sepia(value.parse().map_err(|_| FilterParseError(format!("Invalid sepia value: '{value}'")))?),
            "blur" => FilterValue::Blur(value.parse().map_err(|_| FilterParseError(format!("Invalid blur value: '{value}'")))?),
            "sharpen" => FilterValue::Sharpen(value.parse().map_err(|_| FilterParseError(format!("Invalid sharpen value: '{value}'")))?),
            "pixelate" => FilterValue::Pixelate(value.parse().map_err(|_| FilterParseError(format!("Invalid pixelate value: '{value}'")))?),
            _ => return Err(FilterParseError(format!("Unknown filter key: '{s}'"))),
        })
    }
}
