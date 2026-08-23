use std::str::FromStr;
use crate::config::encoder::EncoderConfig;
use crate::config::quality::QualityConfig;
use crate::config::{parse_env, parse_env_or, ConfigFromEnv};
use crate::params::byte_size::ByteSize;
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
const DEFAULT_OUTPUT_MAX_SIZE_THRESHOLD: &str = "10";
const DEFAULT_OUTPUT_MAX_SIZE_ATTEMPTS: &str = "3";
const DEFAULT_OUTPUT_MAX_SIZE_MIN_QUALITY: u8 = 10;

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
    pub max_size_threshold: u8,
    pub max_size_attempts: u8,
    pub max_size_min_quality: u8,
    pub quality_curves: QualityConfig,
    pub encoder: EncoderConfig,
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
            max_size: parse_env::<ByteSize>("OUTPUT_MAX_SIZE", DEFAULT_OUTPUT_MAX_SIZE)?.0,
            max_size_threshold: parse_env("OUTPUT_MAX_SIZE_THRESHOLD", DEFAULT_OUTPUT_MAX_SIZE_THRESHOLD)?,
            max_size_attempts: parse_env("OUTPUT_MAX_SIZE_ATTEMPTS", DEFAULT_OUTPUT_MAX_SIZE_ATTEMPTS)?,
            max_size_min_quality: {
                let quality = parse_env_or("OUTPUT_MAX_SIZE_MIN_QUALITY", DEFAULT_OUTPUT_MAX_SIZE_MIN_QUALITY)?;

                if !(1..=100).contains(&quality) {
                    anyhow::bail!("OUTPUT_MAX_SIZE_MIN_QUALITY must be between 1 and 100, got {quality}");
                }

                quality
            },
            quality_curves: QualityConfig::from_env()?,
            encoder: EncoderConfig::from_env()?,
        })
    }
}