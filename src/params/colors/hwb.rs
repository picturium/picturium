use std::str::FromStr;
use crate::params::colors::{Color, ColorParseError};

pub struct HwbColor {
    hue:       f64,  // [0, 360)
    whiteness: f64,  // [0.0, 1.0]
    blackness: f64,  // [0.0, 1.0]
    alpha:     f64,  // [0.0, 1.0]
}

pub type HwbColorParseError = ColorParseError;

/// Parses the hue [0, 360), stripping an optional "deg" suffix
fn parse_hue(input: &str) -> Result<f64, HwbColorParseError> {
    let degrees = input.strip_suffix("deg")
        .unwrap_or(input)
        .trim();

    let hue_degrees: f64 = degrees.parse()
        .map_err(|_| ColorParseError(format!("invalid hue value: '{input}'")))?;

    Ok(hue_degrees.rem_euclid(360.0))
}

/// Parses whiteness or blackness — the `%` suffix is required on input
fn parse_wb(input: &str) -> Result<f64, HwbColorParseError> {
    let percentage_text = input.strip_suffix('%')
        .ok_or_else(|| ColorParseError(format!("invalid whiteness/blackness value: '{input}'")))?
        .trim();

    let percentage = percentage_text.parse::<f64>()
        .map_err(|_| ColorParseError(format!("invalid whiteness/blackness value: '{input}'")))?
        .clamp(0.0, 100.0);

    Ok(percentage / 100.0)
}

/// Parses an alpha value, either as a percentage ("20%") or a plain number in [0, 1].
fn parse_alpha(input: &str) -> Result<f64, HwbColorParseError> {
    if let Some(percentage_text) = input.strip_suffix('%') {
        let percentage = percentage_text.trim()
            .parse::<f64>()
            .map_err(|_| ColorParseError(format!("invalid alpha value: '{input}'")))?
            .clamp(0.0, 100.0);

        return Ok(percentage / 100.0);
    }

    let alpha = input.trim()
        .parse::<f64>()
        .map_err(|_| ColorParseError(format!("invalid alpha value: '{input}'")))?
        .clamp(0.0, 1.0);

    Ok(alpha)
}

impl FromStr for HwbColor {
    type Err = HwbColorParseError;

    /// Accepted formats (whiteness and blackness require the `%` suffix):
    /// - `"120deg 75% 25%"`        deg suffix
    /// - `"120 75% 25%"`           plain hue
    /// - `"120 75% 25% / 0.2"`     with slash alpha in [0, 1]
    /// - `"120 75% 25% / 20%"`     with slash alpha as percentage
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // --- slash-separated alpha: "h w b / a[%]" ---
        if let Some((color_text, alpha_text)) = input.split_once('/') {
            let channels: Vec<&str> = color_text.split_whitespace().collect();
            let alpha = alpha_text.trim();

            return match channels.as_slice() {
                [hue, whiteness, blackness] => Ok(Self {
                    hue: parse_hue(hue)?,
                    whiteness: parse_wb(whiteness)?,
                    blackness: parse_wb(blackness)?,
                    alpha: parse_alpha(alpha)?,
                }),
                _ => Err(ColorParseError(format!("invalid HWB color: '{input}'"))),
            };
        }

        // --- plain space-separated branch: "h w b" ---
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            [hue, whiteness, blackness] => Ok(Self {
                hue: parse_hue(hue)?,
                whiteness: parse_wb(whiteness)?,
                blackness: parse_wb(blackness)?,
                alpha: 1.0,
            }),
            _ => Err(ColorParseError(format!("invalid HWB color: '{input}'"))),
        }
    }
}

impl HwbColor {
    /// Converts HWB to RGB, returning each channel in [0.0, 255.0].
    ///
    /// Algorithm (CSS Color 4 §10.1):
    /// 1. If `w + b >= 1`, normalise them so they sum to 1 (achromatic).
    /// 2. Derive the hue-based RGB the same way as HSL with s=1, l=0.5.
    /// 3. Mix: `channel = channel * (1 - w - b) + w`.
    fn hwb_to_rgb(&self) -> (f64, f64, f64) {
        let mut whiteness = self.whiteness;
        let mut blackness = self.blackness;

        // Normalise if whiteness + blackness exceed 1
        let total = whiteness + blackness;

        if total > 1.0 {
            whiteness /= total;
            blackness /= total;
        }

        // Pure-hue RGB (equivalent to HSL with s=1, l=0.5)
        let hue_sector = self.hue / 60.0;
        let sector = hue_sector.floor() as u32 % 6;
        let sector_fraction = hue_sector - hue_sector.floor();

        let (red, green, blue) = match sector {
            0 => (1.0, sector_fraction, 0.0),
            1 => (1.0 - sector_fraction, 1.0, 0.0),
            2 => (0.0, 1.0, sector_fraction),
            3 => (0.0, 1.0 - sector_fraction, 1.0),
            4 => (sector_fraction, 0.0, 1.0),
            _ => (1.0, 0.0, 1.0 - sector_fraction),
        };

        let mix = |channel: f64| channel * (1.0 - whiteness - blackness) + whiteness;

        (mix(red), mix(green), mix(blue))
    }
}

impl Color for HwbColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        let (red, green, blue) = self.hwb_to_rgb();
        (red, green, blue, self.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn approx_eq(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() < 1.0 / 255.0 // within 1 unit on the 0-255 scale
    }

    /// Expected values are given on the 0-255 scale; parsed channels are in [0.0, 1.0].
    fn assert_rgba(
        input: &str,
        expected_red: f64,
        expected_green: f64,
        expected_blue: f64,
        expected_alpha: f64,
    ) {
        let color = HwbColor::from_str(input)
            .unwrap_or_else(|error| panic!("parse failed for '{input}': {error}"));
        let (actual_red, actual_green, actual_blue, actual_alpha) = color.to_rgb();
        let (expected_red, expected_green, expected_blue, expected_alpha) = (
            expected_red / 255.0,
            expected_green / 255.0,
            expected_blue / 255.0,
            expected_alpha / 255.0,
        );
        assert!(
            approx_eq(actual_red, expected_red),
            "red mismatch for '{input}': {actual_red} != {expected_red}"
        );
        assert!(
            approx_eq(actual_green, expected_green),
            "green mismatch for '{input}': {actual_green} != {expected_green}"
        );
        assert!(
            approx_eq(actual_blue, expected_blue),
            "blue mismatch for '{input}': {actual_blue} != {expected_blue}"
        );
        assert!(
            approx_eq(actual_alpha, expected_alpha),
            "alpha mismatch for '{input}': {actual_alpha} != {expected_alpha}"
        );
    }

    // hwb(120, 75%, 25%) => rgb(191, 255, 191)  [w=0.75, b=0.25 → sum=1 → achromatic-ish]
    // Actually w+b=1 so every channel = w = 191. Let's use a non-degenerate case:
    // hwb(120, 10%, 20%) => pure green sector: r=0, g=1, b=0
    //   mix: r=(0*(1-0.1-0.2)+0.1)*255 = 25.5≈26, g=(1*0.7+0.1)*255=204, b=26
    #[test]
    fn test_deg_pct() {
        assert_rgba("120deg 10% 20%", 26.0, 204.0, 26.0, 255.0);
    }

    #[test]
    fn test_plain_hue() {
        assert_rgba("120 10% 20%", 26.0, 204.0, 26.0, 255.0);
    }

    #[test]
    fn test_slash_alpha() {
        assert_rgba("120 10% 20% / 0.2", 26.0, 204.0, 26.0, 51.0);
    }

    #[test]
    fn test_slash_alpha_pct() {
        assert_rgba("120 10% 20% / 20%", 26.0, 204.0, 26.0, 51.0);
    }

    #[test]
    fn test_hue_wrapping() {
        // 480deg == 120deg
        assert_rgba("480 10% 20%", 26.0, 204.0, 26.0, 255.0);
    }

    #[test]
    fn test_normalisation() {
        // w=60% + b=60% > 100% → normalised to w=0.5, b=0.5 → all channels = 128
        assert_rgba("120 60% 60%", 128.0, 128.0, 128.0, 255.0);
    }

    #[test]
    fn test_full_white() {
        // w=100%, b=0% → pure white
        assert_rgba("0 100% 0%", 255.0, 255.0, 255.0, 255.0);
    }

    #[test]
    fn test_full_black() {
        // w=0%, b=100% → pure black
        assert_rgba("0 0% 100%", 0.0, 0.0, 0.0, 255.0);
    }

    #[test]
    fn test_missing_pct_suffix() {
        // whiteness/blackness without '%' must be rejected
        assert!(HwbColor::from_str("120 10 20").is_err());
    }

    #[test]
    fn test_invalid_wb_range() {
        assert!(HwbColor::from_str("120 150% 20%").is_err());
    }

    #[test]
    fn test_invalid_alpha_range() {
        assert!(HwbColor::from_str("120 10% 20% / 2.0").is_err());
    }

    #[test]
    fn test_invalid_format() {
        assert!(HwbColor::from_str("not a color").is_err());
    }
}
