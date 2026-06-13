use std::str::FromStr;
use crate::config::{parse_env, ConfigFromEnv};
use crate::enums::output_effort::OutputEffort;
use crate::enums::output_metadata::OutputMetadata;
use crate::enums::output_quality::OutputQuality;
use anyhow::Result;

const DEFAULT_OUTPUT_FORMAT_PRIORITY: &str = "jxl,avif,webp,jpeg,png";
const DEFAULT_OUTPUT_ENABLE_AVIF: &str = "true";
const DEFAULT_OUTPUT_ENABLE_WEBP: &str = "true";
const DEFAULT_OUTPUT_ENABLE_JXL: &str = "true";
const DEFAULT_OUTPUT_METADATA: &str = "none";
const DEFAULT_OUTPUT_QUALITY: &str = "medium";
const DEFAULT_OUTPUT_EFFORT: &str = "medium";
const DEFAULT_OUTPUT_HIGH_BITDEPTH: &str = "false";
const DEFAULT_OUTPUT_CMYK: &str = "false";
const DEFAULT_OUTPUT_MAX_WIDTH: &str = "5000";
const DEFAULT_OUTPUT_MAX_HEIGHT: &str = "5000";
const DEFAULT_OUTPUT_MAX_SIZE: &str = "0";

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub format_priority: Vec<String>,
    pub enable_avif: bool,
    pub enable_webp: bool,
    pub enable_jxl: bool,
    pub metadata: Vec<OutputMetadata>,
    pub quality: OutputQuality,
    pub effort: OutputEffort,
    pub high_bitdepth: bool,
    pub cmyk: bool,
    pub max_width: u32,
    pub max_height: u32,
    pub max_size: usize,
}

impl ConfigFromEnv for OutputConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            format_priority: parse_env::<String>("OUTPUT_FORMAT_PRIORITY", DEFAULT_OUTPUT_FORMAT_PRIORITY)?
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            enable_avif: parse_env("OUTPUT_ENABLE_AVIF", DEFAULT_OUTPUT_ENABLE_AVIF)?,
            enable_webp: parse_env("OUTPUT_ENABLE_WEBP", DEFAULT_OUTPUT_ENABLE_WEBP)?,
            enable_jxl: parse_env("OUTPUT_ENABLE_JXL", DEFAULT_OUTPUT_ENABLE_JXL)?,
            metadata: parse_env::<String>("OUTPUT_METADATA", DEFAULT_OUTPUT_METADATA)?
                .split(',')
                .map(|s| s.trim())
                .map(OutputMetadata::from_str)
                .collect::<Result<Vec<_>, _>>()?,
            quality: parse_env("OUTPUT_QUALITY", DEFAULT_OUTPUT_QUALITY)?,
            effort: parse_env("OUTPUT_EFFORT", DEFAULT_OUTPUT_EFFORT)?,
            high_bitdepth: parse_env("OUTPUT_HIGH_BITDEPTH", DEFAULT_OUTPUT_HIGH_BITDEPTH)?,
            cmyk: parse_env("OUTPUT_CMYK", DEFAULT_OUTPUT_CMYK)?,
            max_width: parse_env("OUTPUT_MAX_WIDTH", DEFAULT_OUTPUT_MAX_WIDTH)?,
            max_height: parse_env("OUTPUT_MAX_HEIGHT", DEFAULT_OUTPUT_MAX_HEIGHT)?,
            max_size: parse_env("OUTPUT_MAX_SIZE", DEFAULT_OUTPUT_MAX_SIZE)?,
        })
    }
}