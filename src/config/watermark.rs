use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};
use crate::enums::watermark_position::WatermarkPosition;
use crate::params::padding::Padding;

const DEFAULT_WATERMARK_IMAGE: &str = "";
const DEFAULT_WATERMARK_IMAGE_SCALE: &str = "1";
const DEFAULT_WATERMARK_TEXT: &str = "";
const DEFAULT_WATERMARK_FONT_FAMILY: &str = "Arial";
const DEFAULT_WATERMARK_FONT_SIZE: &str = "12";
const DEFAULT_WATERMARK_FONT_COLOR: &str = "black";
const DEFAULT_WATERMARK_ENABLED: &str = "false";
const DEFAULT_WATERMARK_POSITION: &str = "bottom-right";
const DEFAULT_WATERMARK_OPACITY: &str = "50";
const DEFAULT_WATERMARK_PADDING: &str = "10";
const DEFAULT_WATERMARK_ROTATE: &str = "0";

#[derive(Debug, Clone)]
pub struct WatermarkImageConfig {
    pub path: String,
    pub scale: f32,
}

impl ConfigFromEnv for WatermarkImageConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            path: parse_env("WATERMARK_IMAGE", DEFAULT_WATERMARK_IMAGE)?,
            scale: parse_env("WATERMARK_IMAGE_SCALE", DEFAULT_WATERMARK_IMAGE_SCALE)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WatermarkTextConfig {
    pub text: String,
    pub font_family: String,
    pub font_size: u32,
    pub font_color: String,
}

impl ConfigFromEnv for WatermarkTextConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            text: parse_env("WATERMARK_TEXT", DEFAULT_WATERMARK_TEXT)?,
            font_family: parse_env("WATERMARK_FONT_FAMILY", DEFAULT_WATERMARK_FONT_FAMILY)?,
            font_size: parse_env("WATERMARK_FONT_SIZE", DEFAULT_WATERMARK_FONT_SIZE)?,
            font_color: parse_env("WATERMARK_FONT_COLOR", DEFAULT_WATERMARK_FONT_COLOR)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WatermarkConfig {
    pub enabled: bool,
    pub position: WatermarkPosition,
    pub opacity: u8,
    pub padding: Padding,
    pub rotate: i32,
    pub image: WatermarkImageConfig,
    pub text: WatermarkTextConfig,
}

impl ConfigFromEnv for WatermarkConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            enabled: parse_env("WATERMARK_ENABLED", DEFAULT_WATERMARK_ENABLED)?,
            position: parse_env("WATERMARK_POSITION", DEFAULT_WATERMARK_POSITION)?,
            opacity: parse_env("WATERMARK_OPACITY", DEFAULT_WATERMARK_OPACITY)?,
            padding: parse_env("WATERMARK_PADDING", DEFAULT_WATERMARK_PADDING)?,
            rotate: parse_env("WATERMARK_ROTATE", DEFAULT_WATERMARK_ROTATE)?,
            image: WatermarkImageConfig::from_env()?,
            text: WatermarkTextConfig::from_env()?,
        })
    }
}