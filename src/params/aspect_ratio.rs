use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum AspectRatio {
    #[default]
    Auto,
    Value(f32), // width / height
}

#[derive(Debug)]
pub struct AspectRatioParseError(String);

impl fmt::Display for AspectRatioParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid aspect ratio: '{}'", self.0)
    }
}

impl std::error::Error for AspectRatioParseError {}

impl FromStr for AspectRatio {
    type Err = AspectRatioParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "auto" => Ok(Self::Auto),
            "video" => Ok(Self::Value(16.0 / 9.0)),
            "square" => Ok(Self::Value(1.0)),
            custom => {
                let parts: Vec<&str> = custom.split('/').collect();

                if parts.len() < 2 {
                    return Err(AspectRatioParseError(s.to_string()));
                }

                let width: f32 = parts[0]
                    .parse()
                    .map_err(|_| AspectRatioParseError(s.to_string()))?;

                let height: f32 = parts[1]
                    .parse()
                    .map_err(|_| AspectRatioParseError(s.to_string()))?;

                if width <= 0.0 || height <= 0.0 {
                    return Err(AspectRatioParseError(s.to_string()));
                }

                Ok(Self::Value(width / height))
            }
        }
    }
}

impl<'de> Deserialize<'de> for AspectRatio {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = AspectRatio;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "auto, video, square, or a ratio like 16/9, 4/3, ...")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}