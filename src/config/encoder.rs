use crate::enums::avif_compression::AvifCompression;
use crate::enums::avif_encoder::AvifEncoder;
use crate::enums::chroma_subsample::ChromaSubsample;
use crate::enums::webp_preset::WebpPreset;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

fn validate_effort(format: &str, value: i32, range: std::ops::RangeInclusive<i32>) -> Result<()> {
    if !range.contains(&value) {
        bail!(
            "output.encoder.{format}.effort must be between {} and {}, got {value}",
            range.start(),
            range.end()
        );
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct JpegEncoderConfig {
    pub optimize_coding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebpEncoderConfig {
    pub effort: i32,
    pub text_preset_area: f64,
    pub preset_small: WebpPreset,
    pub preset_large: WebpPreset,
    pub min_alpha_quality: i32,
    pub smart_subsample: bool,
    pub animation_min_size: bool,
    pub animation_kmin: i32,
    pub animation_kmax: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PngEncoderConfig {
    pub effort: i32,
    pub lossless_quality: i32,
    pub compression: i32,
    pub lossless_compression: i32,
    pub dither: f64,
    pub lossless_dither: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AvifEncoderConfig {
    pub effort: i32,
    pub bitdepth: i32,
    pub compression: AvifCompression,
    pub encoder: AvifEncoder,
    pub subsample: ChromaSubsample,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct JxlEncoderConfig {
    pub effort: i32,
    pub tier: i32,
    pub lossless: bool,
    pub distance_per_quality: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GifEncoderConfig {
    pub effort: i32,
    pub bitdepth: i32,
    pub min_bitdepth: i32,
    pub dither: f64,
    pub min_dither: f64,
    pub interframe_maxerror: f64,
    pub interpalette_maxerror: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EncoderConfig {
    pub jpeg: JpegEncoderConfig,
    pub webp: WebpEncoderConfig,
    pub png: PngEncoderConfig,
    pub avif: AvifEncoderConfig,
    pub jxl: JxlEncoderConfig,
    pub gif: GifEncoderConfig,
}

impl Default for JpegEncoderConfig {
    fn default() -> Self {
        Self { optimize_coding: true }
    }
}

impl Default for WebpEncoderConfig {
    fn default() -> Self {
        Self {
            effort: 2,
            text_preset_area: 0.25,
            preset_small: WebpPreset::Text,
            preset_large: WebpPreset::Default,
            min_alpha_quality: 75,
            smart_subsample: true,
            animation_min_size: false,
            animation_kmin: 0,
            animation_kmax: 0,
        }
    }
}

impl Default for PngEncoderConfig {
    fn default() -> Self {
        Self {
            effort: 2,
            lossless_quality: 100,
            compression: 2,
            lossless_compression: 3,
            dither: 0.25,
            lossless_dither: 0.0,
        }
    }
}

impl Default for AvifEncoderConfig {
    fn default() -> Self {
        Self {
            effort: 1,
            bitdepth: 8,
            compression: AvifCompression::Av1,
            encoder: AvifEncoder::Aom,
            subsample: ChromaSubsample::On,
        }
    }
}

impl Default for JxlEncoderConfig {
    fn default() -> Self {
        Self {
            effort: 3,
            tier: 0,
            lossless: false,
            distance_per_quality: 0.15,
        }
    }
}

impl Default for GifEncoderConfig {
    fn default() -> Self {
        Self {
            effort: 7,
            bitdepth: 8,
            min_bitdepth: 5,
            dither: 1.0,
            min_dither: 0.6,
            interframe_maxerror: 8.0,
            interpalette_maxerror: 20.0,
        }
    }
}

impl EncoderConfig {
    pub(super) fn validate(&self) -> Result<()> {
        validate_effort("webp", self.webp.effort, 0..=6)?;
        validate_effort("png", self.png.effort, 1..=10)?;
        validate_effort("avif", self.avif.effort, 0..=9)?;
        validate_effort("jxl", self.jxl.effort, 1..=9)?;
        validate_effort("gif", self.gif.effort, 1..=10)?;

        if !(0..=100).contains(&self.webp.min_alpha_quality) {
            bail!(
                "output.encoder.webp.min_alpha_quality must be between 0 and 100, got {}",
                self.webp.min_alpha_quality
            );
        }

        if self.webp.text_preset_area < 0.0 {
            bail!(
                "output.encoder.webp.text_preset_area must not be negative, got {}",
                self.webp.text_preset_area
            );
        }

        if self.webp.animation_kmin < 0 || self.webp.animation_kmax < self.webp.animation_kmin {
            bail!(
                "output.encoder.webp.animation_kmin must not be negative and must be no larger than animation_kmax, got {} and {}",
                self.webp.animation_kmin,
                self.webp.animation_kmax
            );
        }

        if !(1..=100).contains(&self.png.lossless_quality) {
            bail!(
                "output.encoder.png.lossless_quality must be between 1 and 100, got {}",
                self.png.lossless_quality
            );
        }

        for (name, value) in [
            ("output.encoder.png.compression", self.png.compression),
            (
                "output.encoder.png.lossless_compression",
                self.png.lossless_compression,
            ),
        ] {
            if !(0..=9).contains(&value) {
                bail!("{name} must be between 0 and 9, got {value}");
            }
        }

        for (name, value) in [
            ("output.encoder.png.dither", self.png.dither),
            ("output.encoder.png.lossless_dither", self.png.lossless_dither),
        ] {
            if !(0.0..=1.0).contains(&value) {
                bail!("{name} must be between 0 and 1, got {value}");
            }
        }

        if !matches!(self.avif.bitdepth, 8 | 10 | 12) {
            bail!(
                "output.encoder.avif.bitdepth must be 8, 10 or 12, got {}",
                self.avif.bitdepth
            );
        }

        for (name, value) in [
            ("output.encoder.gif.bitdepth", self.gif.bitdepth),
            ("output.encoder.gif.min_bitdepth", self.gif.min_bitdepth),
        ] {
            if !(1..=8).contains(&value) {
                bail!("{name} must be between 1 and 8, got {value}");
            }
        }

        if self.gif.min_bitdepth > self.gif.bitdepth {
            bail!(
                "output.encoder.gif.min_bitdepth ({}) must not exceed output.encoder.gif.bitdepth ({})",
                self.gif.min_bitdepth,
                self.gif.bitdepth
            );
        }

        for (name, value) in [
            ("output.encoder.gif.dither", self.gif.dither),
            ("output.encoder.gif.min_dither", self.gif.min_dither),
        ] {
            if !(0.0..=1.0).contains(&value) {
                bail!("{name} must be between 0 and 1, got {value}");
            }
        }

        if self.gif.min_dither > self.gif.dither {
            bail!(
                "output.encoder.gif.min_dither ({}) must not exceed output.encoder.gif.dither ({})",
                self.gif.min_dither,
                self.gif.dither
            );
        }

        if !(0.0..=32.0).contains(&self.gif.interframe_maxerror) {
            bail!(
                "output.encoder.gif.interframe_maxerror must be between 0 and 32, got {}",
                self.gif.interframe_maxerror
            );
        }

        if !(0.0..=256.0).contains(&self.gif.interpalette_maxerror) {
            bail!(
                "output.encoder.gif.interpalette_maxerror must be between 0 and 256, got {}",
                self.gif.interpalette_maxerror
            );
        }

        if !(0..=4).contains(&self.jxl.tier) {
            bail!("output.encoder.jxl.tier must be between 0 and 4, got {}", self.jxl.tier);
        }

        if self.jxl.distance_per_quality <= 0.0 {
            bail!(
                "output.encoder.jxl.distance_per_quality must be positive, got {}",
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

        assert_eq!(config.webp.effort, 2);
        assert_eq!(config.png.effort, 2);
        assert_eq!(config.avif.effort, 1);
        assert_eq!(config.jxl.effort, 3);
        assert_eq!(config.gif.effort, 7);
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
        assert_eq!(config.gif.bitdepth, 8);
        assert_eq!(config.gif.min_bitdepth, 5);
        assert_eq!(config.gif.dither, 1.0);
        assert_eq!(config.gif.min_dither, 0.6);
        assert_eq!(config.gif.interframe_maxerror, 8.0);
        assert_eq!(config.gif.interpalette_maxerror, 20.0);
    }

    #[test]
    fn the_shipped_defaults_validate() {
        assert!(EncoderConfig::default().validate().is_ok());
    }

    #[test]
    fn rejects_an_effort_outside_the_codec_range() {
        // webp tops out at 6.
        let mut config = EncoderConfig::default();
        config.webp.effort = 7;
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_out_of_range_encoder_options() {
        let invalid: [fn(&mut EncoderConfig); 12] = [
            |c| c.avif.bitdepth = 7,
            |c| c.jxl.tier = 5,
            |c| c.png.compression = 10,
            |c| c.png.dither = 1.5,
            |c| c.webp.min_alpha_quality = 101,
            |c| c.gif.bitdepth = 9,
            |c| c.gif.min_bitdepth = 0,
            |c| c.gif.bitdepth = 4, // below the default min_bitdepth of 5
            |c| c.gif.dither = 0.5, // below the default min_dither of 0.6
            |c| c.gif.min_dither = 1.5,
            |c| c.gif.interframe_maxerror = 40.0,
            |c| c.gif.interpalette_maxerror = 300.0,
        ];

        for (index, apply) in invalid.iter().enumerate() {
            let mut config = EncoderConfig::default();
            apply(&mut config);
            assert!(config.validate().is_err(), "case {index} was accepted");
        }
    }
}
