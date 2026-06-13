use std::str::FromStr;
use crate::params::colors::{Color, ColorParseError};

pub struct OklchColor {
    l:     f64,  // [0.0, 1.0]    perceived lightness
    c:     f64,  // [0.0, ~0.5]   chroma (0% = 0, 100% = 0.4)
    h:     f64,  // [0, 360)      hue angle in degrees
    alpha: f64,  // [0.0, 1.0]
}

pub type OklchColorParseError = ColorParseError;

/// Parses L: a plain number in [0, 1] or a percentage in [0%, 100%].
fn parse_l(raw: &str) -> Result<f64, OklchColorParseError> {
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

/// Parses C (chroma): a plain number >= 0 or a percentage in [0%, 100%].
/// 100% maps to 0.4 per the CSS spec.
fn parse_c(raw: &str) -> Result<f64, OklchColorParseError> {
    let err = || ColorParseError(format!("invalid C value: '{raw}'"));
    if let Some(pct) = raw.strip_suffix('%') {
        let v: f64 = pct.trim().parse().map_err(|_| err())?;
        if !(0.0..=100.0).contains(&v) {
            return Err(ColorParseError(format!("C percentage out of range: '{raw}'")));
        }
        Ok(v / 100.0 * 0.4)
    } else {
        let v: f64 = raw.trim().parse().map_err(|_| err())?;
        if v < 0.0 {
            return Err(ColorParseError(format!("C value must be >= 0: '{raw}'")));
        }
        Ok(v)
    }
}

/// Parses H (hue): a plain number or an angle with a "deg" suffix.
/// Hue is wrapped into [0, 360) via rem_euclid.
fn parse_h(raw: &str) -> Result<f64, OklchColorParseError> {
    let err = || ColorParseError(format!("invalid H value: '{raw}'"));
    let stripped = raw.strip_suffix("deg").unwrap_or(raw).trim();
    let v: f64 = stripped.parse().map_err(|_| err())?;
    Ok(v.rem_euclid(360.0))
}

/// Parses an alpha value: a plain number in [0, 1] or a percentage in [0%, 100%].
fn parse_alpha(raw: &str) -> Result<f64, OklchColorParseError> {
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

impl FromStr for OklchColor {
    type Err = OklchColorParseError;

    /// Accepted formats:
    /// - `"0.5 0.2 120"`           plain numbers
    /// - `"50% 50% 120deg"`        percentages + deg suffix on hue
    /// - `"0.5 0.2 120 / 0.5"`    with slash alpha in [0, 1]
    /// - `"0.5 0.2 120 / 50%"`    with slash alpha as percentage
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(format!("invalid Oklch color: '{s}'"));

        // --- slash-separated alpha branch: "L C H / alpha" ---
        if let Some((color_part, alpha_part)) = s.split_once('/') {
            let channels: Vec<&str> = color_part.split_whitespace().collect();
            let alpha_raw = alpha_part.trim();

            return match channels.as_slice() {
                [l, c, h] => Ok(Self {
                    l:     parse_l(l)?,
                    c:     parse_c(c)?,
                    h:     parse_h(h)?,
                    alpha: parse_alpha(alpha_raw)?,
                }),
                _ => Err(err()),
            };
        }

        // --- plain space-separated branch: "L C H" ---
        let parts: Vec<&str> = s.split_whitespace().collect();
        match parts.as_slice() {
            [l, c, h] => Ok(Self {
                l:     parse_l(l)?,
                c:     parse_c(c)?,
                h:     parse_h(h)?,
                alpha: 1.0,
            }),
            _ => Err(err()),
        }
    }
}

impl OklchColor {
    /// Converts Oklch → Oklab → linear sRGB → gamma-compressed sRGB, all channels in [0.0, 1.0].
    ///
    /// Pipeline:
    /// 1. Oklch → Oklab: `a = C·cos(H°)`, `b = C·sin(H°)`.
    /// 2. Oklab → LMS (cube the linear combinations).
    /// 3. LMS → linear sRGB via the standard 3×3 matrix.
    /// 4. Apply sRGB gamma (piecewise) and clamp to [0, 1].
    fn oklch_to_rgb(&self) -> (f64, f64, f64) {
        // Step 1 — Oklch → Oklab
        let h_rad = self.h.to_radians();
        let a = self.c * h_rad.cos();
        let b = self.c * h_rad.sin();
        let l = self.l;

        // Step 2 — Oklab → LMS
        let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
        let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
        let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);

        // Step 3 — LMS → linear sRGB
        let r_lin =  4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_;
        let g_lin = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_;
        let b_lin = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_;

        // Step 4 — linear sRGB → gamma-compressed sRGB, clamped to [0, 1]
        let gamma = |c: f64| -> f64 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        };
        let clamp = |c: f64| c.clamp(0.0, 1.0);

        (clamp(gamma(r_lin)), clamp(gamma(g_lin)), clamp(gamma(b_lin)))
    }
}

impl Color for OklchColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        let (r, g, b) = self.oklch_to_rgb();
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
        let c = OklchColor::from_str(s).unwrap_or_else(|e| panic!("parse failed for '{s}': {e}"));
        let (cr, cg, cb, ca) = c.to_rgb();
        assert!(approx_eq(cr, r), "red mismatch for '{s}': {cr} != {r}");
        assert!(approx_eq(cg, g), "green mismatch for '{s}': {cg} != {g}");
        assert!(approx_eq(cb, b), "blue mismatch for '{s}': {cb} != {b}");
        assert!(approx_eq(ca, a), "alpha mismatch for '{s}': {ca} != {a}");
    }

    // oklch(1 0 0) → white (C=0 means achromatic, hue irrelevant)
    #[test]
    fn test_white() {
        assert_rgba("1 0 0", 1.0, 1.0, 1.0, 1.0);
    }

    // oklch(0 0 0) → black
    #[test]
    fn test_black() {
        assert_rgba("0 0 0", 0.0, 0.0, 0.0, 1.0);
    }

    // oklch(100% 0% 0) → white (percentage form)
    #[test]
    fn test_white_pct() {
        assert_rgba("100% 0% 0", 1.0, 1.0, 1.0, 1.0);
    }

    // oklch(0% 0% 0) → black (percentage form)
    #[test]
    fn test_black_pct() {
        assert_rgba("0% 0% 0", 0.0, 0.0, 0.0, 1.0);
    }

    // oklch(0.5 0 0) → mid grey (C=0 → achromatic)
    #[test]
    fn test_grey() {
        let c = OklchColor::from_str("0.5 0 0").unwrap();
        let (r, g, b, _) = c.to_rgb();
        assert!(approx_eq(r, g) && approx_eq(g, b), "expected grey, got ({r}, {g}, {b})");
    }

    // deg suffix on hue
    #[test]
    fn test_deg_suffix() {
        let a = OklchColor::from_str("0.5 0.2 120").unwrap().to_rgb();
        let b = OklchColor::from_str("0.5 0.2 120deg").unwrap().to_rgb();
        assert!(approx_eq(a.0, b.0) && approx_eq(a.1, b.1) && approx_eq(a.2, b.2));
    }

    // hue wrapping: 480deg == 120deg
    #[test]
    fn test_hue_wrapping() {
        let a = OklchColor::from_str("0.5 0.2 120").unwrap().to_rgb();
        let b = OklchColor::from_str("0.5 0.2 480").unwrap().to_rgb();
        assert!(approx_eq(a.0, b.0) && approx_eq(a.1, b.1) && approx_eq(a.2, b.2));
    }

    // Slash alpha — plain number
    #[test]
    fn test_slash_alpha() {
        let c = OklchColor::from_str("1 0 0 / 0.5").unwrap();
        let (_, _, _, a) = c.to_rgb();
        assert!(approx_eq(a, 0.5), "alpha mismatch: {a} != 0.5");
    }

    // Slash alpha — percentage
    #[test]
    fn test_slash_alpha_pct() {
        let c = OklchColor::from_str("1 0 0 / 50%").unwrap();
        let (_, _, _, a) = c.to_rgb();
        assert!(approx_eq(a, 0.5), "alpha mismatch: {a} != 0.5");
    }

    // oklch(0.5 0.2 120) — chromatic colour, channels must stay in [0, 1]
    #[test]
    fn test_chromatic_in_range() {
        let c = OklchColor::from_str("0.5 0.2 120").unwrap();
        let (r, g, b, a) = c.to_rgb();
        assert!((0.0..=1.0).contains(&r), "r out of range: {r}");
        assert!((0.0..=1.0).contains(&g), "g out of range: {g}");
        assert!((0.0..=1.0).contains(&b), "b out of range: {b}");
        assert!(approx_eq(a, 1.0));
    }

    #[test]
    fn test_invalid_l_range() {
        assert!(OklchColor::from_str("1.5 0 0").is_err());
    }

    #[test]
    fn test_invalid_c_negative() {
        assert!(OklchColor::from_str("0.5 -0.1 120").is_err());
    }

    #[test]
    fn test_invalid_alpha_range() {
        assert!(OklchColor::from_str("0.5 0.2 120 / 2.0").is_err());
    }

    #[test]
    fn test_invalid_format() {
        assert!(OklchColor::from_str("not a color").is_err());
    }
}
