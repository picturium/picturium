use crate::enums::image_gravity::ImageGravity;
use crate::params::aspect_ratio::AspectRatio;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Crop {
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub aspect_ratio: Option<AspectRatio>,
    pub gravity: Option<ImageGravity>,
    pub x: Option<i16>,
    pub y: Option<i16>,
}

#[derive(Debug)]
pub struct CropParseError(String);

impl fmt::Display for CropParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid crop definition: '{}'", self.0)
    }
}

impl std::error::Error for CropParseError {}

impl FromStr for Crop {
    type Err = CropParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('|')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut crop = Crop::default();

        for part in parts {
            let (key, value) = part.split_once(':')
                .ok_or_else(|| CropParseError(format!("Missing ':' in crop segment '{part}'")))?;

            match key {
                "w" | "width" => crop.width = Some(value.parse().map_err(|_| CropParseError(format!("Invalid crop width: '{value}'")))?),
                "h" | "height" => crop.height = Some(value.parse().map_err(|_| CropParseError(format!("Invalid crop height: '{value}'")))?),
                "ar" | "aspect_ratio" => crop.aspect_ratio = Some(value.parse().map_err(|_| CropParseError(format!("Invalid crop aspect ratio: '{value}'")))?),
                "g" | "gravity" => crop.gravity = Some(value.parse().map_err(|_| CropParseError(format!("Invalid crop gravity: '{value}'")))?),
                "x" => crop.x = Some(value.parse().map_err(|_| CropParseError(format!("Invalid crop x offset: '{value}'")))?),
                "y" => crop.y = Some(value.parse().map_err(|_| CropParseError(format!("Invalid crop y offset: '{value}'")))?),
                _ => {},
            }
        }

        if crop.width.is_none() && crop.height.is_none() && crop.aspect_ratio.is_none() {
            return Err(CropParseError("Please specify at least one of w, h, or ar".to_string()));
        }

        Ok(crop)
    }
}

impl<'de> Deserialize<'de> for Crop {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Crop;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "crop parameters in format crop=ar:auto|w:50|h:50|g:center|x:0|y:0")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}
