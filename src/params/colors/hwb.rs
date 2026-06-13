use std::str::FromStr;
use crate::params::colors::{Color, ColorParseError};

pub struct HwbColor {
    hue:       f64,  // [0, 360)
    whiteness: f64,  // [0.0, 1.0]
    blackness: f64,  // [0.0, 1.0]
    alpha:     f64,  // [0.0, 1.0]
}

pub type HwbColorParseError = ColorParseError;

/// Parses the hue, stripping an optional "deg" suffix.
/// Hue is kept in degrees [0, 360) via wrapping modulo.
fn parse_hue(raw: &str) -> Result<f64, HwbColorParseError> {
    let err = || ColorParseError(format!("invalid hue value: '{raw}'"));
    let stripped = raw.strip_suffix("deg").unwrap_or(raw).trim();
    let v: f64 = stripped.parse().map_err(|_| err())?;
    Ok(v.rem_euclid(360.0))
}

/// Parses whiteness or blackness — the `%` suffix is required.
/// Value is normalised to [0.0, 1.0] and must be in [0, 100].
fn parse_wb(raw: &str) -> Result<f64, HwbColorParseError> {
    let err = || ColorParseError(format!("invalid whiteness/blackness value: '{raw}'"));
    let pct = raw
        .strip_suffix('%')
        .ok_or_else(err)?
        .trim();
    let v: f64 = pct.parse().map_err(|_| err())?;
    if !(0.0..=100.0).contains(&v) {
        return Err(ColorParseError(format!("whiteness/blackness out of range: '{raw}'")));
    }
    Ok(v / 100.0)
}

/// Parses an alpha value, either as a percentage ("20%") or a plain number in [0, 1].
fn parse_alpha(raw: &str) -> Result<f64, HwbColorParseError> {
    let err = || ColorParseError(format!("invalid alpha value: '{raw}'"));
    if let Some(pct) = raw.strip_suffix('%') {
        let v: f64 = pct.trim().parse().map_err(|_| err())?;
        if !(0.0..=100.0).contains(&v) {
            return Err(ColorParseError(format!("alpha percentage out of range: '{raw}'")));
        }
        Ok(v / 100.0)
    } else {
        let v: f64 = raw.trim().parse().map_err(|_| err())?;
        if !(0.0..=1.0).contains(&v) {
            return Err(ColorParseError(format!("alpha value out of range: '{raw}'")));
        }
        Ok(v)
    }
}

impl FromStr for HwbColor {
    type Err = HwbColorParseError;

    /// Accepted formats (whiteness and blackness require the `%` suffix):
    /// - `"120deg 75% 25%"`        deg suffix
    /// - `"120 75% 25%"`           plain hue
    /// - `"120 75% 25% / 0.2"`     with slash alpha in [0, 1]
    /// - `"120 75% 25% / 20%"`     with slash alpha as percentage
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(format!("invalid HWB color: '{s}'"));

        // --- slash-separated alpha branch: "h w b / a[%]" ---
        if let Some((color_part, alpha_part)) = s.split_once('/') {
            let channels: Vec<&str> = color_part.split_whitespace().collect();
            let alpha_raw = alpha_part.trim();

            return match channels.as_slice() {
                [h, w, b] => Ok(Self {
                    hue:       parse_hue(h)?,
                    whiteness: parse_wb(w)?,
                    blackness: parse_wb(b)?,
                    alpha:     parse_alpha(alpha_raw)?,
                }),
                _ => Err(err()),
            };
        }

        // --- plain space-separated branch: "h w b" ---
        let parts: Vec<&str> = s.split_whitespace().collect();
        match parts.as_slice() {
            [h, w, b] => Ok(Self {
                hue:       parse_hue(h)?,
                whiteness: parse_wb(w)?,
                blackness: parse_wb(b)?,
                alpha:     1.0,
            }),
            _ => Err(err()),
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
        let mut w = self.whiteness;
        let mut bk = self.blackness;

        // Normalise if whiteness + blackness exceed 1
        let sum = w + bk;
        if sum > 1.0 {
            w /= sum;
            bk /= sum;
        }

        // Pure-hue RGB (equivalent to HSL with s=1, l=0.5)
        let h = self.hue / 60.0;
        let sector = h.floor() as u32 % 6;
        let f = h - h.floor();

        let (r, g, b) = match sector {
            0 => (1.0, f,   0.0),
            1 => (1.0 - f, 1.0, 0.0),
            2 => (0.0, 1.0, f),
            3 => (0.0, 1.0 - f, 1.0),
            4 => (f,   0.0, 1.0),
            _ => (1.0, 0.0, 1.0 - f),
        };

        let mix = |c: f64| c * (1.0 - w - bk) + w;
        (mix(r), mix(g), mix(b))
    }
}

impl Color for HwbColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        let (r, g, b) = self.hwb_to_rgb();
        (r, g, b, self.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1.0  // within 1 unit on the 0-255 scale
    }

    fn assert_rgba(s: &str, r: f64, g: f64, b: f64, a: f64) {
        let c = HwbColor::from_str(s).unwrap_or_else(|e| panic!("parse failed for '{s}': {e}"));
        let (cr, cg, cb, ca) = c.to_rgb();
        assert!(approx_eq(cr, r), "red mismatch for '{s}': {cr} != {r}");
        assert!(approx_eq(cg, g), "green mismatch for '{s}': {cg} != {g}");
        assert!(approx_eq(cb, b), "blue mismatch for '{s}': {cb} != {b}");
        assert!(approx_eq(ca, a), "alpha mismatch for '{s}': {ca} != {a}");
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
