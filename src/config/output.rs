use crate::config::encoder::EncoderConfig;
use crate::config::quality::QualityConfig;
use crate::enums::output_metadata::OutputMetadata;
use crate::enums::output_quality::OutputQuality;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OutputConfig {
    pub format_priority: Vec<String>,
    pub enable_avif: bool,
    pub enable_webp: bool,
    pub enable_jxl: bool,
    pub metadata: Vec<OutputMetadata>,
    pub quality: OutputQuality,
    pub high_bitdepth: bool,
    pub cmyk: bool,
    pub max_width: u32,
    pub max_height: u32,
    #[serde(deserialize_with = "crate::params::byte_size::deserialize_usize")]
    pub max_size: usize,
    pub max_size_threshold: u8,
    pub max_size_attempts: u8,
    pub max_size_min_quality: u8,
    pub quality_curves: QualityConfig,
    pub encoder: EncoderConfig,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format_priority: ["jxl", "avif", "webp", "jpeg", "png"]
                .iter()
                .map(|format| format.to_string())
                .collect(),
            enable_avif: true,
            enable_webp: true,
            enable_jxl: true,
            metadata: vec![OutputMetadata::None],
            quality: OutputQuality::Medium,
            high_bitdepth: false,
            cmyk: false,
            max_width: 5000,
            max_height: 5000,
            max_size: 0,
            max_size_threshold: 10,
            max_size_attempts: 3,
            max_size_min_quality: 10,
            quality_curves: QualityConfig::default(),
            encoder: EncoderConfig::default(),
        }
    }
}

impl OutputConfig {
    pub(super) fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.max_size_min_quality) {
            bail!(
                "output.max_size_min_quality must be between 1 and 100, got {}",
                self.max_size_min_quality
            );
        }

        self.quality_curves.validate()?;
        self.encoder.validate()
    }
}
