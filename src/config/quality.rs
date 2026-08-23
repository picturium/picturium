use crate::config::{ConfigFromEnv, parse_env_or};
use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityCurve {
    pub min: f64,
    pub max: f64,
    pub low: f64,
    pub high: f64,
    pub maximum: f64,
}

impl QualityCurve {
    const JPEG: Self = Self { min: 32.0, max: 80.0, low: -12.0, high: 10.0, maximum: 17.0 };
    const WEBP: Self = Self { min: 40.0, max: 78.0, low: -10.0, high: 8.0, maximum: 15.0 };
    const AVIF: Self = Self { min: 38.0, max: 67.0, low: -10.0, high: 10.0, maximum: 16.0 };
    const PNG: Self = Self { min: 32.0, max: 99.0, low: -30.0, high: 0.0, maximum: 1.0 };
    const JXL: Self = Self { min: 66.667, max: 85.333, low: -10.0, high: 4.467, maximum: 11.667 };

    fn parse_env(&self, format: &str) -> Result<Self> {
        let curve = Self {
            min: parse_env_or(&format!("OUTPUT_QUALITY_{format}_MIN"), self.min)?,
            max: parse_env_or(&format!("OUTPUT_QUALITY_{format}_MAX"), self.max)?,
            low: parse_env_or(&format!("OUTPUT_QUALITY_{format}_LOW"), self.low)?,
            high: parse_env_or(&format!("OUTPUT_QUALITY_{format}_HIGH"), self.high)?,
            maximum: parse_env_or(&format!("OUTPUT_QUALITY_{format}_MAXIMUM"), self.maximum)?,
        };

        curve.validate(format)?;
        Ok(curve)
    }

    fn validate(&self, format: &str) -> Result<()> {
        for (name, value) in [("MIN", self.min), ("MAX", self.max)] {
            if !(1.0..=100.0).contains(&value) {
                bail!("OUTPUT_QUALITY_{format}_{name} must be between 1 and 100, got {value}");
            }
        }

        for (name, value) in [
            ("LOW", self.low),
            ("HIGH", self.high),
            ("MAXIMUM", self.maximum),
        ] {
            if !(-100.0..=100.0).contains(&value) {
                bail!("OUTPUT_QUALITY_{format}_{name} must be between -100 and 100, got {value}");
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct QualityConfig {
    pub jpeg: QualityCurve,
    pub webp: QualityCurve,
    pub avif: QualityCurve,
    pub png: QualityCurve,
    pub jxl: QualityCurve,
    pub min_area: f64,
    pub max_area: f64,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            jpeg: QualityCurve::JPEG,
            webp: QualityCurve::WEBP,
            avif: QualityCurve::AVIF,
            png: QualityCurve::PNG,
            jxl: QualityCurve::JXL,
            min_area: 8.0,
            max_area: 0.25,
        }
    }
}

impl ConfigFromEnv for QualityConfig {
    fn from_env() -> Result<Self> {
        let default = Self::default();

        let config = Self {
            jpeg: default.jpeg.parse_env("JPEG")?,
            webp: default.webp.parse_env("WEBP")?,
            avif: default.avif.parse_env("AVIF")?,
            png: default.png.parse_env("PNG")?,
            jxl: default.jxl.parse_env("JXL")?,
            min_area: parse_env_or("OUTPUT_QUALITY_MIN_AREA", default.min_area)?,
            max_area: parse_env_or("OUTPUT_QUALITY_MAX_AREA", default.max_area)?,
        };

        if !(config.max_area > 0.0) || !(config.min_area > config.max_area) {
            bail!(
                "OUTPUT_QUALITY_MIN_AREA ({}) must be greater than OUTPUT_QUALITY_MAX_AREA ({}), which must be above 0",
                config.min_area,
                config.max_area
            );
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_shipped_curves() {
        let config = QualityConfig::default();

        assert_eq!(config.jpeg.min, 32.0);
        assert_eq!(config.jpeg.max, 80.0);
        assert_eq!(config.webp.low, -10.0);
        assert_eq!(config.png.maximum, 1.0);
        assert_eq!(config.min_area, 8.0);
        assert_eq!(config.max_area, 0.25);
    }

    #[test]
    fn the_jxl_curve_inverts_to_its_tuned_distances() {
        let jxl = QualityConfig::default().jxl;
        let distance = |quality: f64| 0.15 * (100.0 - quality);

        assert!((distance(jxl.min) - 5.0).abs() < 0.01);
        assert!((distance(jxl.max) - 2.2).abs() < 0.01);
    }

    #[test]
    fn rejects_a_quality_outside_the_shared_domain() {
        let curve = QualityCurve { max: 120.0, ..QualityCurve::JPEG };
        assert!(curve.validate("JPEG").is_err());

        let curve = QualityCurve { min: 0.0, ..QualityCurve::JPEG };
        assert!(curve.validate("JPEG").is_err());
    }

    #[test]
    fn rejects_an_out_of_range_modifier() {
        let curve = QualityCurve { low: -400.0, ..QualityCurve::JPEG };
        assert!(curve.validate("JPEG").is_err());
    }

    #[test]
    fn accepts_every_shipped_curve() {
        let config = QualityConfig::default();

        for (name, curve) in [
            ("JPEG", config.jpeg),
            ("WEBP", config.webp),
            ("AVIF", config.avif),
            ("PNG", config.png),
            ("JXL", config.jxl),
        ] {
            assert!(curve.validate(name).is_ok(), "{name} curve rejected");
        }
    }
}
