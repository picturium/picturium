use std::str::FromStr;
use crate::params::colors::{Color, ColorParseError};

pub struct OklabColor {
    l:     f64,  // [0.0, 1.0]   perceived lightness
    a:     f64,  // [-0.5, 0.5]  green↔red axis
    b:     f64,  // [-0.5, 0.5]  blue↔yellow axis
    alpha: f64,  // [0.0, 1.0]
}

pub type OklabColorParseError = ColorParseError;

/// Parses L: a plain number in [0, 1] or a percentage in [0%, 100%].
fn parse_l(raw: &str) -> Result<f64, OklabColorParseError> {
    let err = || ColorParseError(format!("invalid L value: '{raw}'"));
    if let Some(pct) = raw.strip_suffix('%') {
        let v: f64 = pct.trim().parse().map_err(|_| err())?;
        if !(0.0..=100.0).contains(&v) {
            return Err(ColorParseError(format!("L percentage out of range: '{raw}'")));
        }
        Ok(v / 100.0)
    } else {
        let v: f64 = raw.trim().parse().map_err(|_| err())?;
        if !(0.0..=1.0).contains(&v) {
            return Err(ColorParseError(format!("L value out of range: '{raw}'")));
        }
        Ok(v)
    }
}

/// Parses a or b: a plain number in [-0.5, 0.5] or a percentage in [-100%, 100%].
/// Percentages are mapped so that ±100% = ±0.4 (CSS spec mapping).
fn parse_ab(raw: &str) -> Result<f64, OklabColorParseError> {
    let err = || ColorParseError(format!("invalid a/b value: '{raw}'"));
    if let Some(pct) = raw.strip_suffix('%') {
        let v: f64 = pct.trim().parse().map_err(|_| err())?;
        if !(-100.0..=100.0).contains(&v) {
            return Err(ColorParseError(format!("a/b percentage out of range: '{raw}'")));
        }
        Ok(v / 100.0 * 0.4)
    } else {
        let v: f64 = raw.trim().parse().map_err(|_| err())?;
        if !(-0.5..=0.5).contains(&v) {
            return Err(ColorParseError(format!("a/b value out of range: '{raw}'")));
        }
        Ok(v)
    }
}

/// Parses an alpha value: a plain number in [0, 1] or a percentage in [0%, 100%].
fn parse_alpha(raw: &str) -> Result<f64, OklabColorParseError> {
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

impl FromStr for OklabColor {
    type Err = OklabColorParseError;

    /// Accepted formats:
    /// - `"0.5 0.1 -0.1"`           plain numbers
    /// - `"50% 25% -25%"`           percentages
    /// - `"0.5 0.1 -0.1 / 0.5"`    with slash alpha in [0, 1]
    /// - `"0.5 0.1 -0.1 / 50%"`    with slash alpha as percentage
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(format!("invalid Oklab color: '{s}'"));

        // --- slash-separated alpha branch: "L a b / alpha" ---
        if let Some((color_part, alpha_part)) = s.split_once('/') {
            let channels: Vec<&str> = color_part.split_whitespace().collect();
            let alpha_raw = alpha_part.trim();

            return match channels.as_slice() {
                [l, a, b] => Ok(Self {
                    l:     parse_l(l)?,
                    a:     parse_ab(a)?,
                    b:     parse_ab(b)?,
                    alpha: parse_alpha(alpha_raw)?,
                }),
                _ => Err(err()),
            };
        }

        // --- plain space-separated branch: "L a b" ---
        let parts: Vec<&str> = s.split_whitespace().collect();
        match parts.as_slice() {
            [l, a, b] => Ok(Self {
                l:     parse_l(l)?,
                a:     parse_ab(a)?,
                b:     parse_ab(b)?,
                alpha: 1.0,
            }),
            _ => Err(err()),
        }
    }
}

impl OklabColor {
    /// Converts Oklab → linear sRGB → gamma-compressed sRGB, all channels in [0.0, 1.0].
    ///
    /// Pipeline (Björn Ottosson's Oklab spec):
    /// 1. Oklab → LMS (cube): l̂ = (L + 0.3963377774·a + 0.2158037573·b)³  etc.
    /// 2. LMS → linear sRGB via the standard 3×3 matrix.
    /// 3. Apply sRGB gamma (piecewise): linear → display.
    /// 4. Clamp to [0, 1].
    fn oklab_to_rgb(&self) -> (f64, f64, f64) {
        let l = self.l;
        let a = self.a;
        let b = self.b;

        // Step 1 — Oklab → LMS (cube roots of the intermediate values)
        let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
        let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
        let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);

        // Step 2 — LMS → linear sRGB
        let r_lin =  4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_;
        let g_lin = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_;
        let b_lin = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_;

        // Step 3 — linear sRGB → gamma-compressed sRGB
        let gamma = |c: f64| -> f64 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        };

        // Step 4 — clamp to [0, 1]
        let clamp = |c: f64| c.clamp(0.0, 1.0);

        (clamp(gamma(r_lin)), clamp(gamma(g_lin)), clamp(gamma(b_lin)))
    }
}

impl Color for OklabColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        let (r, g, b) = self.oklab_to_rgb();
        (r, g, b, self.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01  // within 1% on the [0, 1] scale
    }

    fn assert_rgba(s: &str, r: f64, g: f64, b: f64, a: f64) {
        let c = OklabColor::from_str(s).unwrap_or_else(|e| panic!("parse failed for '{s}': {e}"));
        let (cr, cg, cb, ca) = c.to_rgb();
        assert!(approx_eq(cr, r), "red mismatch for '{s}': {cr} != {r}");
        assert!(approx_eq(cg, g), "green mismatch for '{s}': {cg} != {g}");
        assert!(approx_eq(cb, b), "blue mismatch for '{s}': {cb} != {b}");
        assert!(approx_eq(ca, a), "alpha mismatch for '{s}': {ca} != {a}");
    }

    // oklab(1 0 0) → white
    #[test]
    fn test_white() {
        assert_rgba("1 0 0", 1.0, 1.0, 1.0, 1.0);
    }

    // oklab(0 0 0) → black
    #[test]
    fn test_black() {
        assert_rgba("0 0 0", 0.0, 0.0, 0.0, 1.0);
    }

    // oklab(100% 0% 0%) → white (percentage form)
    #[test]
    fn test_white_pct() {
        assert_rgba("100% 0% 0%", 1.0, 1.0, 1.0, 1.0);
    }

    // oklab(0% 0% 0%) → black (percentage form)
    #[test]
    fn test_black_pct() {
        assert_rgba("0% 0% 0%", 0.0, 0.0, 0.0, 1.0);
    }

    // oklab(0.5 0 0) → mid grey
    #[test]
    fn test_grey() {
        let c = OklabColor::from_str("0.5 0 0").unwrap();
        let (r, g, b, _) = c.to_rgb();
        // All channels should be equal for a neutral grey
        assert!(approx_eq(r, g) && approx_eq(g, b), "expected grey, got ({r}, {g}, {b})");
    }

    // Slash alpha — plain number
    #[test]
    fn test_slash_alpha() {
        let c = OklabColor::from_str("1 0 0 / 0.5").unwrap();
        let (_, _, _, a) = c.to_rgb();
        assert!(approx_eq(a, 0.5), "alpha mismatch: {a} != 0.5");
    }

    // Slash alpha — percentage
    #[test]
    fn test_slash_alpha_pct() {
        let c = OklabColor::from_str("1 0 0 / 50%").unwrap();
        let (_, _, _, a) = c.to_rgb();
        assert!(approx_eq(a, 0.5), "alpha mismatch: {a} != 0.5");
    }

    // oklab(0.5 0.1 -0.1) — mixed a/b, just check it parses and stays in range
    #[test]
    fn test_mixed_ab() {
        let c = OklabColor::from_str("0.5 0.1 -0.1").unwrap();
        let (r, g, b, a) = c.to_rgb();
        assert!((0.0..=1.0).contains(&r), "r out of range: {r}");
        assert!((0.0..=1.0).contains(&g), "g out of range: {g}");
        assert!((0.0..=1.0).contains(&b), "b out of range: {b}");
        assert!(approx_eq(a, 1.0));
    }

    #[test]
    fn test_invalid_l_range() {
        assert!(OklabColor::from_str("1.5 0 0").is_err());
    }

    #[test]
    fn test_invalid_ab_range() {
        assert!(OklabColor::from_str("0.5 0.6 0").is_err());
    }

    #[test]
    fn test_invalid_alpha_range() {
        assert!(OklabColor::from_str("0.5 0 0 / 2.0").is_err());
    }

    #[test]
    fn test_invalid_format() {
        assert!(OklabColor::from_str("not a color").is_err());
    }
}
