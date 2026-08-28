use crate::enums::watermark_position::WatermarkPosition;
use crate::params::color::Color;
use crate::params::padding::Padding;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WatermarkImageConfig {
    pub path: String,
    pub scale: f32,
}

impl Default for WatermarkImageConfig {
    fn default() -> Self {
        Self { path: String::new(), scale: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WatermarkTextConfig {
    pub text: String,
    pub font_family: String,
    pub font_size: u32,
    pub font_color: String,
}

impl Default for WatermarkTextConfig {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_family: "Arial".into(),
            font_size: 12,
            font_color: "black".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WatermarkConfig {
    pub enabled: bool,
    pub position: WatermarkPosition,
    pub opacity: u8,
    pub padding: Padding,
    pub rotate: f64,
    pub max_scale: f32,
    pub image: WatermarkImageConfig,
    pub text: WatermarkTextConfig,
}

impl Default for WatermarkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: WatermarkPosition::BottomRight,
            opacity: 50,
            padding: Padding { top: 10, right: 10, bottom: 10, left: 10 },
            rotate: 0.0,
            max_scale: 1.0,
            image: WatermarkImageConfig::default(),
            text: WatermarkTextConfig::default(),
        }
    }
}

impl WatermarkConfig {
    pub(super) fn validate(&self) -> Result<()> {
        if self.opacity > 100 {
            bail!(
                "watermark.opacity must be between 0 and 100, got {}",
                self.opacity
            );
        }

        if self.image.scale <= 0.0 {
            bail!(
                "watermark.image.scale must be greater than 0, got {}",
                self.image.scale
            );
        }

        if !(self.max_scale > 0.0 && self.max_scale <= 1.0) {
            bail!(
                "watermark.max_scale must be greater than 0 and at most 1, got {}",
                self.max_scale
            );
        }

        if self.text.font_color.parse::<Color>().is_err() {
            bail!(
                "watermark.text.font_color is not a valid color: {}",
                self.text.font_color
            );
        }

        if self.enabled && self.image.path.is_empty() && self.text.text.is_empty() {
            bail!("watermark.enabled is on, but neither watermark.image.path nor watermark.text.text is set");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_scale_outside_zero_to_one_is_refused() {
        let mut config = WatermarkConfig::default();
        assert!(config.validate().is_ok());

        config.max_scale = 1.5;
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("watermark.max_scale"), "{error}");

        config.max_scale = 0.0;
        assert!(config.validate().is_err());
    }
}
