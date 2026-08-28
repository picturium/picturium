use crate::config::watermark::WatermarkConfig;
use crate::enums::watermark_position::WatermarkPosition;
use crate::params::color::Color;
use crate::params::padding::Padding;
use crate::params::scale::parse_scale;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, de};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Default)]
pub struct Watermark {
    pub enabled: bool,
    pub anchor: Option<WatermarkPosition>,
    pub opacity: Option<u8>,
    pub padding: Option<Padding>,
    pub rotate: Option<f64>,
    pub image: Option<String>,
    pub scale: Option<f32>,
    pub text: Option<String>,
    pub font: Option<String>,
    pub size: Option<u16>,
    pub color: Option<Color>,
}

#[derive(Debug, Clone)]
pub enum WatermarkSource {
    Image {
        path: String,
        from_request: bool,
        scale: f32,
    },
    Text {
        text: String,
        font: String,
        size: u32,
        color: Color,
    },
}

#[derive(Debug, Clone)]
pub struct ResolvedWatermark {
    pub source: WatermarkSource,
    pub anchor: WatermarkPosition,
    pub opacity: u8,
    pub padding: Padding,
    pub rotate: f64,
    pub max_scale: f32,
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
                "rot" | "rotate" => {
                    let v: f64 = value.parse().map_err(|_| WatermarkParseError(format!("Invalid rot value: '{value}'")))?;

                    if !v.is_finite() {
                        return Err(WatermarkParseError(format!("Invalid rot value: '{value}'")));
                    }

                    watermark.rotate = Some(v);
                },
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

    pub fn resolve(request: Option<Self>, config: &WatermarkConfig) -> Option<ResolvedWatermark> {
        let request = match request {
            Some(request) if !request.enabled => return None,
            Some(request) => request,
            None if config.enabled => Self::default(),
            None => return None,
        };

        Some(ResolvedWatermark {
            source: request.source(config)?,
            anchor: request.anchor.unwrap_or(config.position),
            opacity: request.opacity.unwrap_or(config.opacity),
            padding: request.padding.unwrap_or(config.padding),
            rotate: request.rotate.unwrap_or(config.rotate),
            max_scale: config.max_scale,
        })
    }

    fn source(&self, config: &WatermarkConfig) -> Option<WatermarkSource> {
        if let Some(path) = &self.image {
            return Some(WatermarkSource::Image {
                path: path.clone(),
                from_request: true,
                scale: self.scale.unwrap_or(config.image.scale),
            });
        }

        if let Some(text) = &self.text {
            return Some(self.text_source(text.clone(), config));
        }

        if !config.image.path.is_empty() {
            return Some(WatermarkSource::Image {
                path: config.image.path.clone(),
                from_request: false,
                scale: self.scale.unwrap_or(config.image.scale),
            });
        }

        if !config.text.text.is_empty() {
            return Some(self.text_source(config.text.text.clone(), config));
        }

        None
    }

    fn text_source(&self, text: String, config: &WatermarkConfig) -> WatermarkSource {
        WatermarkSource::Text {
            text,
            font: self.font.clone().unwrap_or_else(|| config.text.font_family.clone()),
            size: self.size.map(u32::from).unwrap_or(config.text.font_size),
            color: self.color.or_else(|| config.text.font_color.parse().ok()).unwrap_or(Color { red: 0.0, green: 0.0, blue: 0.0, alpha: 1.0 }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> WatermarkConfig {
        WatermarkConfig::default()
    }

    #[test]
    fn every_supported_key_is_parsed() {
        let watermark: Watermark =
            "anchor:top-left|opacity:40|pad:5|rot:12.5|image:logo.png|scale:0.5|font:Arial|size:24|color:black"
                .parse()
                .unwrap();

        assert!(watermark.enabled);
        assert_eq!(watermark.anchor, Some(WatermarkPosition::TopLeft));
        assert_eq!(watermark.opacity, Some(40));
        assert_eq!(watermark.padding, Some(Padding { top: 5, right: 5, bottom: 5, left: 5 }));
        assert_eq!(watermark.rotate, Some(12.5));
        assert_eq!(watermark.image.as_deref(), Some("logo.png"));
        assert_eq!(watermark.scale, Some(0.5));
        assert_eq!(watermark.font.as_deref(), Some("Arial"));
        assert_eq!(watermark.size, Some(24));
    }

    #[test]
    fn hostile_values_are_rejected() {
        assert!("opacity:101".parse::<Watermark>().is_err());
        assert!("scale:0".parse::<Watermark>().is_err());
        assert!("rot:sideways".parse::<Watermark>().is_err());
        assert!("rot:NaN".parse::<Watermark>().is_err());
        assert!("nonsense:1".parse::<Watermark>().is_err());
        assert!("image".parse::<Watermark>().is_err());
    }

    #[test]
    fn an_absent_parameter_follows_the_configuration() {
        let mut enabled = config();
        enabled.enabled = true;
        enabled.image.path = "logo.png".into();

        assert!(Watermark::resolve(None, &config()).is_none());
        assert!(Watermark::resolve(None, &enabled).is_some());
    }

    #[test]
    fn watermark_false_wins_over_an_enabled_configuration() {
        let mut enabled = config();
        enabled.enabled = true;
        enabled.image.path = "logo.png".into();

        let request = "false".parse::<Watermark>().unwrap();

        assert!(Watermark::resolve(Some(request), &enabled).is_none());
    }

    #[test]
    fn nothing_to_draw_resolves_to_no_watermark() {
        let request = "opacity:50".parse::<Watermark>().unwrap();

        assert!(Watermark::resolve(Some(request), &config()).is_none());
    }

    #[test]
    fn an_image_beats_text_and_the_request_beats_the_configuration() {
        let mut config = config();
        config.image.path = "configured.png".into();

        let request = "image:requested.png|text:Hello".parse::<Watermark>().unwrap();
        let resolved = Watermark::resolve(Some(request), &config).unwrap();

        assert!(matches!(
            resolved.source,
            WatermarkSource::Image { ref path, from_request: true, .. } if path == "requested.png"
        ));

        let request = "text:Hello".parse::<Watermark>().unwrap();
        let resolved = Watermark::resolve(Some(request), &config).unwrap();

        assert!(matches!(resolved.source, WatermarkSource::Text { ref text, .. } if text == "Hello"));
    }

    #[test]
    fn a_configured_image_path_is_not_confined_to_the_data_directory() {
        let mut config = config();
        config.enabled = true;
        config.image.path = "/etc/picturium/logo.png".into();

        let resolved = Watermark::resolve(None, &config).unwrap();

        assert!(matches!(resolved.source, WatermarkSource::Image { from_request: false, .. }));
    }

    #[test]
    fn unset_keys_fall_back_to_the_configuration() {
        let mut config = config();
        config.opacity = 25;
        config.rotate = 30.0;
        config.position = WatermarkPosition::Center;

        let request = "image:logo.png".parse::<Watermark>().unwrap();
        let resolved = Watermark::resolve(Some(request), &config).unwrap();

        assert_eq!(resolved.opacity, 25);
        assert_eq!(resolved.rotate, 30.0);
        assert_eq!(resolved.anchor, WatermarkPosition::Center);
        assert_eq!(resolved.padding, config.padding);
    }
}
