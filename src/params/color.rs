use crate::params::colors::hex::HexColor;
use crate::params::colors::hsl::HslColor;
use crate::params::colors::hwb::HwbColor;
use crate::params::colors::named::NamedColor;
use crate::params::colors::oklab::OklabColor;
use crate::params::colors::oklch::OklchColor;
use crate::params::colors::rgb::RgbColor;
use anyhow::Result;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

const BIT_DEPTH_MULTIPLIER_8: f64 = 255.0;
const BIT_DEPTH_MULTIPLIER_10: f64 = 1023.0;
const BIT_DEPTH_MULTIPLIER_12: f64 = 4095.0;
const BIT_DEPTH_MULTIPLIER_16: f64 = 65535.0;

pub fn get_bit_depth_multiplier(bit_depth: u8) -> f64 {
    match bit_depth {
        8 => BIT_DEPTH_MULTIPLIER_8,
        10 => BIT_DEPTH_MULTIPLIER_10,
        12 => BIT_DEPTH_MULTIPLIER_12,
        16 => BIT_DEPTH_MULTIPLIER_16,
        _ => panic!("Unsupported bit depth: {}", bit_depth),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub red: f64, // [0.0, 1.0]
    pub green: f64, // [0.0, 1.0]
    pub blue: f64, // [0.0, 1.0]
    pub alpha: f64, // [0.0, 1.0]
}

impl Color {
    fn to_rgb(&self, bit_depth_multiplier: f64) -> (f64, f64, f64, f64) {
        (
            self.red * bit_depth_multiplier,
            self.green * bit_depth_multiplier,
            self.blue * bit_depth_multiplier,
            self.alpha * bit_depth_multiplier,
        )
    }

    pub fn to_rgb_with_bit_depth(&self, bit_depth: u8) -> (f64, f64, f64, f64) {
        self.to_rgb(get_bit_depth_multiplier(bit_depth))
    }

    pub fn to_rgb_vec_with_bit_depth(&self, bit_depth: u8) -> Vec<f64> {
        let (r, g, b, a) = self.to_rgb_with_bit_depth(bit_depth);
        vec![r, g, b, a]
    }
}

#[derive(Debug)]
pub struct ColorParseError(String);

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid color value: '{}'", self.0)
    }
}

impl std::error::Error for ColorParseError {}

impl From<crate::params::colors::ColorParseError> for ColorParseError {
    fn from(e: crate::params::colors::ColorParseError) -> Self {
        ColorParseError(e.to_string())
    }
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();

        let color: Box<dyn crate::params::colors::Color> = match s.as_str() {
            // RGB
            s if s.starts_with("rgb(") || s.starts_with("rgba(") => Box::new(RgbColor::from_str(
                &s.replace("rgb(", "").replace("rgba(", "").replace(")", ""),
            )?),
            // HSL
            s if s.starts_with("hsl(") || s.starts_with("hsla(") => Box::new(HslColor::from_str(
                &s.replace("hsl(", "").replace("hsla(", "").replace(")", ""),
            )?),
            // HWB
            s if s.starts_with("hwb(") => Box::new(HwbColor::from_str(
                &s.replace("hwb(", "").replace(")", ""),
            )?),
            // Oklab
            s if s.starts_with("oklab(") => Box::new(OklabColor::from_str(
                &s.replace("oklab(", "").replace(")", ""),
            )?),
            // Oklch
            s if s.starts_with("oklch(") => Box::new(OklchColor::from_str(
                &s.replace("oklch(", "").replace(")", ""),
            )?),

            // Deprecated RGB syntax from v0.1
            s if s.contains(',') || s.contains(' ') => Box::new(RgbColor::from_str(s)?),

            // CSS named colors with HEX fallback
            s => if let Ok(color) = NamedColor::from_str(s) {
                Box::new(color)
            } else {
                Box::new(HexColor::from_str(s)?)
            },
        };

        let (red, green, blue, alpha) = color.to_rgb();
        Ok(Self { red, green, blue, alpha })
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Color;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "color value in CSS format. Supported formats are hex, hsl, hwb, oklab, oklch, rgb")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}
