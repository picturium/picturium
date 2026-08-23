use crate::config::{ConfigFromEnv, parse_env_or};
use crate::enums::avif_compression::AvifCompression;
use crate::enums::avif_encoder::AvifEncoder;
use crate::enums::chroma_subsample::ChromaSubsample;
use crate::enums::output_effort::OutputEffort;
use crate::enums::webp_preset::WebpPreset;
use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffortLevels {
    pub low: i32,
    pub medium: i32,
    pub high: i32,
}

impl EffortLevels {
    pub fn get(&self, level: OutputEffort) -> i32 {
        match level {
            OutputEffort::Low => self.low,
            OutputEffort::Medium => self.medium,
            OutputEffort::High => self.high,
        }
    }

    fn parse_env(&self, format: &str) -> Result<Self> {
        Ok(Self {
            low: parse_env_or(&format!("OUTPUT_EFFORT_{format}_LOW"), self.low)?,
            medium: parse_env_or(&format!("OUTPUT_EFFORT_{format}_MEDIUM"), self.medium)?,
            high: parse_env_or(&format!("OUTPUT_EFFORT_{format}_HIGH"), self.high)?,
        })
    }

    fn validate(&self, format: &str, range: std::ops::RangeInclusive<i32>) -> Result<()> {
        for (name, value) in [
            ("LOW", self.low),
            ("MEDIUM", self.medium),
            ("HIGH", self.high),
        ] {
            if !range.contains(&value) {
                bail!(
                    "OUTPUT_EFFORT_{format}_{name} must be between {} and {}, got {value}",
                    range.start(),
                    range.end()
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct JpegEncoderConfig {
    pub optimize_coding: bool,
}

#[derive(Debug, Clone)]
pub struct WebpEncoderConfig {
    pub effort: EffortLevels,
    pub text_preset_area: f64,
    pub preset_small: WebpPreset,
    pub preset_large: WebpPreset,
    pub min_alpha_quality: i32,
    pub smart_subsample: bool,
}

#[derive(Debug, Clone)]
pub struct PngEncoderConfig {
    pub effort: EffortLevels,
    pub lossless_quality: i32,
    pub compression: i32,
    pub lossless_compression: i32,
    pub dither: f64,
    pub lossless_dither: f64,
}

#[derive(Debug, Clone)]
pub struct AvifEncoderConfig {
    pub effort: EffortLevels,
    pub bitdepth: i32,
    pub compression: AvifCompression,
    pub encoder: AvifEncoder,
    pub subsample: ChromaSubsample,
}

#[derive(Debug, Clone)]
pub struct JxlEncoderConfig {
    pub effort: EffortLevels,
    pub tier: i32,
    pub lossless: bool,
    pub distance_per_quality: f64,
}

#[derive(Debug, Clone)]
pub struct GifEncoderConfig {
    pub effort: EffortLevels,
}

#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub jpeg: JpegEncoderConfig,
    pub webp: WebpEncoderConfig,
    pub png: PngEncoderConfig,
    pub avif: AvifEncoderConfig,
    pub jxl: JxlEncoderConfig,
    pub gif: GifEncoderConfig,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            jpeg: JpegEncoderConfig {
                optimize_coding: true,
            },
            webp: WebpEncoderConfig {
                effort: EffortLevels {
                    low: 0,
                    medium: 2,
                    high: 4,
                },
                text_preset_area: 0.25,
                preset_small: WebpPreset::Text,
                preset_large: WebpPreset::Default,
                min_alpha_quality: 75,
                smart_subsample: true,
            },
            png: PngEncoderConfig {
                effort: EffortLevels {
                    low: 1,
                    medium: 2,
                    high: 5,
                },
                lossless_quality: 100,
                compression: 2,
                lossless_compression: 3,
                dither: 0.25,
                lossless_dither: 0.0,
            },
            avif: AvifEncoderConfig {
                effort: EffortLevels {
                    low: 0,
                    medium: 1,
                    high: 4,
                },
                bitdepth: 8,
                compression: AvifCompression::Av1,
                encoder: AvifEncoder::Aom,
                subsample: ChromaSubsample::On,
            },
            jxl: JxlEncoderConfig {
                effort: EffortLevels {
                    low: 1,
                    medium: 3,
                    high: 6,
                },
                tier: 0,
                lossless: false,
                distance_per_quality: 0.15,
            },
            gif: GifEncoderConfig {
                effort: EffortLevels {
                    low: 1,
                    medium: 7,
                    high: 9,
                },
            },
        }
    }
}

impl ConfigFromEnv for EncoderConfig {
    fn from_env() -> Result<Self> {
        let default = Self::default();

        let config = Self {
            jpeg: JpegEncoderConfig {
                optimize_coding: parse_env_or(
                    "OUTPUT_JPEG_OPTIMIZE_CODING",
                    default.jpeg.optimize_coding,
                )?,
            },
            webp: WebpEncoderConfig {
                effort: default.webp.effort.parse_env("WEBP")?,
                text_preset_area: parse_env_or(
                    "OUTPUT_WEBP_TEXT_PRESET_AREA",
                    default.webp.text_preset_area,
                )?,
                preset_small: parse_env_or("OUTPUT_WEBP_PRESET_SMALL", default.webp.preset_small)?,
                preset_large: parse_env_or("OUTPUT_WEBP_PRESET_LARGE", default.webp.preset_large)?,
                min_alpha_quality: parse_env_or(
                    "OUTPUT_WEBP_MIN_ALPHA_QUALITY",
                    default.webp.min_alpha_quality,
                )?,
                smart_subsample: parse_env_or(
                    "OUTPUT_WEBP_SMART_SUBSAMPLE",
                    default.webp.smart_subsample,
                )?,
            },
            png: PngEncoderConfig {
                effort: default.png.effort.parse_env("PNG")?,
                lossless_quality: parse_env_or(
                    "OUTPUT_PNG_LOSSLESS_QUALITY",
                    default.png.lossless_quality,
                )?,
                compression: parse_env_or("OUTPUT_PNG_COMPRESSION", default.png.compression)?,
                lossless_compression: parse_env_or(
                    "OUTPUT_PNG_LOSSLESS_COMPRESSION",
                    default.png.lossless_compression,
                )?,
                dither: parse_env_or("OUTPUT_PNG_DITHER", default.png.dither)?,
                lossless_dither: parse_env_or(
                    "OUTPUT_PNG_LOSSLESS_DITHER",
                    default.png.lossless_dither,
                )?,
            },
            avif: AvifEncoderConfig {
                effort: default.avif.effort.parse_env("AVIF")?,
                bitdepth: parse_env_or("OUTPUT_AVIF_BITDEPTH", default.avif.bitdepth)?,
                compression: parse_env_or("OUTPUT_AVIF_COMPRESSION", default.avif.compression)?,
                encoder: parse_env_or("OUTPUT_AVIF_ENCODER", default.avif.encoder)?,
                subsample: parse_env_or("OUTPUT_AVIF_SUBSAMPLE", default.avif.subsample)?,
            },
            jxl: JxlEncoderConfig {
                effort: default.jxl.effort.parse_env("JXL")?,
                tier: parse_env_or("OUTPUT_JXL_TIER", default.jxl.tier)?,
                lossless: parse_env_or("OUTPUT_JXL_LOSSLESS", default.jxl.lossless)?,
                distance_per_quality: parse_env_or(
                    "OUTPUT_JXL_DISTANCE_PER_QUALITY",
                    default.jxl.distance_per_quality,
                )?,
            },
            gif: GifEncoderConfig {
                effort: default.gif.effort.parse_env("GIF")?,
            },
        };

        config.validate()?;
        Ok(config)
    }
}

impl EncoderConfig {
    fn validate(&self) -> Result<()> {
        self.webp.effort.validate("WEBP", 0..=6)?;
        self.png.effort.validate("PNG", 1..=10)?;
        self.avif.effort.validate("AVIF", 0..=9)?;
        self.jxl.effort.validate("JXL", 1..=9)?;
        self.gif.effort.validate("GIF", 1..=10)?;

        if !(0..=100).contains(&self.webp.min_alpha_quality) {
            bail!(
                "OUTPUT_WEBP_MIN_ALPHA_QUALITY must be between 0 and 100, got {}",
                self.webp.min_alpha_quality
            );
        }

        if self.webp.text_preset_area < 0.0 {
            bail!(
                "OUTPUT_WEBP_TEXT_PRESET_AREA must not be negative, got {}",
                self.webp.text_preset_area
            );
        }

        if !(1..=100).contains(&self.png.lossless_quality) {
            bail!(
                "OUTPUT_PNG_LOSSLESS_QUALITY must be between 1 and 100, got {}",
                self.png.lossless_quality
            );
        }

        for (name, value) in [
            ("OUTPUT_PNG_COMPRESSION", self.png.compression),
            (
                "OUTPUT_PNG_LOSSLESS_COMPRESSION",
                self.png.lossless_compression,
            ),
        ] {
            if !(0..=9).contains(&value) {
                bail!("{name} must be between 0 and 9, got {value}");
            }
        }

        for (name, value) in [
            ("OUTPUT_PNG_DITHER", self.png.dither),
            ("OUTPUT_PNG_LOSSLESS_DITHER", self.png.lossless_dither),
        ] {
            if !(0.0..=1.0).contains(&value) {
                bail!("{name} must be between 0 and 1, got {value}");
            }
        }

        if !matches!(self.avif.bitdepth, 8 | 10 | 12) {
            bail!(
                "OUTPUT_AVIF_BITDEPTH must be 8, 10 or 12, got {}",
                self.avif.bitdepth
            );
        }

        if !(0..=4).contains(&self.jxl.tier) {
            bail!("OUTPUT_JXL_TIER must be between 0 and 4, got {}", self.jxl.tier);
        }

        if self.jxl.distance_per_quality <= 0.0 {
            bail!(
                "OUTPUT_JXL_DISTANCE_PER_QUALITY must be positive, got {}",
                self.jxl.distance_per_quality
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_effort_matches_what_the_encoders_used_before() {
        let config = EncoderConfig::default();

        assert_eq!(config.webp.effort.get(OutputEffort::Medium), 2);
        assert_eq!(config.png.effort.get(OutputEffort::Medium), 2);
        assert_eq!(config.avif.effort.get(OutputEffort::Medium), 1);
        assert_eq!(config.jxl.effort.get(OutputEffort::Medium), 3);
        assert_eq!(config.gif.effort.get(OutputEffort::Medium), 7);
    }

    #[test]
    fn selects_the_configured_level() {
        let levels = EffortLevels {
            low: 1,
            medium: 5,
            high: 9,
        };

        assert_eq!(levels.get(OutputEffort::Low), 1);
        assert_eq!(levels.get(OutputEffort::Medium), 5);
        assert_eq!(levels.get(OutputEffort::High), 9);
    }

    #[test]
    fn defaults_match_the_shipped_encoder_options() {
        let config = EncoderConfig::default();

        assert!(config.jpeg.optimize_coding);
        assert_eq!(config.webp.text_preset_area, 0.25);
        assert_eq!(config.webp.min_alpha_quality, 75);
        assert_eq!(config.png.compression, 2);
        assert_eq!(config.png.lossless_compression, 3);
        assert_eq!(config.png.dither, 0.25);
        assert_eq!(config.png.lossless_dither, 0.0);
        assert_eq!(config.avif.bitdepth, 8);
        assert_eq!(config.jxl.tier, 0);
        assert!(!config.jxl.lossless);
    }

    #[test]
    fn the_shipped_defaults_validate() {
        assert!(EncoderConfig::default().validate().is_ok());
    }

    #[test]
    fn rejects_an_effort_outside_the_codec_range() {
        // webp tops out at 6.
        let mut config = EncoderConfig::default();
        config.webp.effort.high = 7;
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_out_of_range_encoder_options() {
        let invalid: [fn(&mut EncoderConfig); 5] = [
            |c| c.avif.bitdepth = 7,
            |c| c.jxl.tier = 5,
            |c| c.png.compression = 10,
            |c| c.png.dither = 1.5,
            |c| c.webp.min_alpha_quality = 101,
        ];

        for (index, apply) in invalid.iter().enumerate() {
            let mut config = EncoderConfig::default();
            apply(&mut config);
            assert!(config.validate().is_err(), "case {index} was accepted");
        }
    }
}
