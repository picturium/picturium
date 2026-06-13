use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;
use picturium_libvips::VipsAngle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rotate {
    #[default]
    No = 0,
    Left = 90,
    Right = 270,
    BottomUp = 180
}

#[derive(Debug)]
pub struct RotateParseError(String);

impl fmt::Display for RotateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid rotation value: '{}'", self.0)
    }
}

impl std::error::Error for RotateParseError {}

impl FromStr for Rotate {
    type Err = RotateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "0" | "no" => Ok(Self::No),
            "90" | "left" | "anticlockwise" => Ok(Self::Left),
            "180" | "bottom-up" => Ok(Self::BottomUp),
            "270" | "right" | "clockwise" => Ok(Self::Right),
            _ => Err(RotateParseError(s.to_string())),
        }
    }
}

impl<'de> Deserialize<'de> for Rotate {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Rotate;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "0, 90, 180, 270, no, left, right, bottom-up, clockwise, or anticlockwise")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}

impl Into<VipsAngle> for Rotate {
    fn into(self) -> VipsAngle {
        match self {
            Rotate::No => VipsAngle::None,
            Rotate::Left => VipsAngle::Left,
            Rotate::BottomUp => VipsAngle::UpsideDown,
            Rotate::Right => VipsAngle::Right,
        }
    }
}
