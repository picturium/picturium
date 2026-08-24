use crate::enums::watermark_position::WatermarkPosition;
use crate::params::padding::Padding;
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
    pub rotate: i32,
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
            rotate: 0,
            image: WatermarkImageConfig::default(),
            text: WatermarkTextConfig::default(),
        }
    }
}
