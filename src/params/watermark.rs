use std::fmt;
use std::str::FromStr;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;
use crate::enums::watermark_position::WatermarkPosition;
use crate::params::color::Color;
use crate::params::padding::Padding;
use crate::params::rotate::Rotate;
use crate::params::scale::parse_scale;

#[derive(Debug, Clone, Default)]
pub struct Watermark {
    pub enabled: bool,
    pub anchor: Option<WatermarkPosition>,
    pub opacity: Option<u8>,
    pub padding: Option<Padding>,
    pub rotate: Option<Rotate>,
    pub image: Option<String>,
    pub scale: Option<f32>,
    pub text: Option<String>,
    pub font: Option<String>,
    pub size: Option<u16>,
    pub color: Option<Color>,
}

#[derive(Debug)]
pub struct WatermarkParseError(String);

impl fmt::Display for WatermarkParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WatermarkParseError {}

impl FromStr for Watermark {
    type Err = WatermarkParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim() == "false" {
            return Ok(Watermark {
                enabled: false,
                ..Watermark::default()
            });
        }

        let parts: Vec<&str> = s.split('|')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut watermark = Watermark {
            enabled: true,
            ..Watermark::default()
        };

        for part in parts {
            let (key, value) = part.split_once(':').ok_or_else(|| WatermarkParseError(format!("Missing ':' in watermark segment '{part}'")))?;

            match key {
                "anchor" => watermark.anchor = Some(value.parse().map_err(|_| WatermarkParseError(format!("Invalid anchor value: '{value}'")))?),
                "opacity" => {
                    let v: u8 = value.parse().map_err(|_| WatermarkParseError(format!("Invalid opacity value: '{value}'")))?;

                    if v > 100 {
                        return Err(WatermarkParseError(format!("Opacity value must be between 0 and 100, got '{v}'")));
                    }

                    watermark.opacity = Some(v);
                },
                "pad" | "padding" => watermark.padding = Some(value.parse().map_err(|_| WatermarkParseError(format!("Invalid padding value: '{value}'")))?),
                "rot" | "rotate" => watermark.rotate = Some(value.parse().map_err(|_| WatermarkParseError(format!("Invalid rot value: '{value}'")))?),
                "image" => watermark.image = Some(value.to_string()),
                "scale" => {
                    let v: f32 = value.parse().map_err(|_| WatermarkParseError(format!("Invalid scale value: '{value}'")))?;
                    watermark.scale = Some(parse_scale(v).map_err(|e| WatermarkParseError(e.to_string()))?);
                },
                "text" => watermark.text = Some(value.to_string()),
                "font" => watermark.font = Some(value.to_string()),
                "size" => watermark.size = Some(value.parse().map_err(|_| WatermarkParseError(format!("Invalid size value: '{value}'")))?),
                "color" => watermark.color = Some(value.parse().map_err(|_| WatermarkParseError(format!("Invalid color value: '{value}'")))?),
                _ => return Err(WatermarkParseError(format!("Unknown watermark key: '{key}'"))),
            }
        }

        Ok(watermark)
    }
}

impl<'de> Deserialize<'de> for Watermark {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Watermark;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "watermark parameters in format anchor:top-left|opacity:50|pad:10|rot:45|image:image.png|scale:0.5|text:Hello|font:Arial|size:24|color:#000000")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}

impl Watermark {
    pub fn apply_dpr(&self, dpr: f32) -> Self {
        Self {
            scale: self.scale.map(|s| s * dpr),
            padding: self.padding.map(|p| p.apply_dpr(dpr)),
            size: self.size.map(|s| (s as f32 * dpr) as u16),
            ..self.clone()
        }
    }
}
