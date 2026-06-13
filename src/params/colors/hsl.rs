use crate::params::colors::{Color, ColorParseError};
use std::str::FromStr;

pub struct HslColor {
    hue: f64,        // [0, 360)
    saturation: f64, // [0.0, 1.0]
    lightness: f64,  // [0.0, 1.0]
    alpha: f64,      // [0.0, 1.0]
}

/// Parses the hue, stripping an optional "deg" suffix.
/// Hue is kept in degrees [0, 360) via wrapping modulo.
fn parse_hue(raw: &str) -> Result<f64, ColorParseError> {
    let stripped = raw.strip_suffix("deg").unwrap_or(raw).trim();
    let value: f64 = stripped.parse().map_err(|_| ColorParseError(format!("invalid hue value: '{raw}'")))?;

    Ok(value.rem_euclid(360.0))
}

/// Parses saturation or lightness, accepting either a plain number or a percentage.
/// Both forms are normalized to [0.0, 1.0] and clamped to [0, 100].
fn parse_hue_modifier(raw: &str) -> Result<f64, ColorParseError> {
    let stripped = raw.strip_suffix('%').unwrap_or(raw).trim();
    let value: f64 = stripped.parse().map_err(|_| ColorParseError(format!("invalid saturation/lightness value: '{raw}'")))?;

    if !(0.0..=100.0).contains(&value) {
        return Err(ColorParseError(format!(
            "saturation/lightness out of range: '{raw}'"
        )));
    }

    Ok(value / 100.0)
}

/// Parses an alpha value, either as a percentage ("20%") or a plain number in [0, 1].
fn parse_alpha(raw: &str) -> Result<f64, ColorParseError> {
    if let Some(percentage) = raw.strip_suffix('%') {
        let value: f64 = percentage.trim().parse().map_err(|_| ColorParseError(format!("invalid alpha value: '{raw}'")))?;

        if !(0.0..=100.0).contains(&value) {
            return Err(ColorParseError(format!(
                "alpha percentage out of range: '{raw}'"
            )));
        }

        return Ok(value / 100.0);
    }

    let value: f64 = raw.trim().parse().map_err(|_| ColorParseError(format!("invalid alpha value: '{raw}'")))?;

    if !(0.0..=1.0).contains(&value) {
        return Err(ColorParseError(format!(
            "alpha value out of range: '{raw}'"
        )));
    }

    Ok(value)
}

impl FromStr for HslColor {
    type Err = ColorParseError;

    /// Accepted formats (saturation and lightness accept both plain numbers and `%`):
    /// - `"120deg 75% 25%"`        deg suffix, percentage S/L
    /// - `"120 75% 25%"`           plain hue, percentage S/L
    /// - `"120deg 75 25"`          deg suffix, plain S/L
    /// - `"120 75 25"`             plain hue, plain S/L
    /// - `"120 75 25 / 0.2"`       with slash alpha in [0, 1]
    /// - `"120 75 25 / 20%"`       with slash alpha as percentage
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // --- slash-separated alpha branch: "h s l / a[%]" ---
        if let Some((color_part, alpha_part)) = s.split_once('/') {
            let channels: Vec<&str> = color_part.split_whitespace().collect();
            let alpha_raw = alpha_part.trim();

            return match channels.as_slice() {
                [h, s, l] => Ok(Self {
                    hue: parse_hue(h)?,
                    saturation: parse_hue_modifier(s)?,
                    lightness: parse_hue_modifier(l)?,
                    alpha: parse_alpha(alpha_raw)?,
                }),
                _ => Err(ColorParseError(format!("invalid HSL color: '{s}'"))),
            };
        }

        // --- plain space-separated branch: "h s l" ---
        let parts: Vec<&str> = s.split_whitespace().collect();
        match parts.as_slice() {
            [h, s, l] => Ok(Self {
                hue: parse_hue(h)?,
                saturation: parse_hue_modifier(s)?,
                lightness: parse_hue_modifier(l)?,
                alpha: 1.0,
            }),
            _ => Err(ColorParseError(format!("invalid HSL color: '{s}'"))),
        }
    }
}

impl HslColor {
    /// Converts HSL to RGB, returning each channel in [0.0, 1.0].
    fn hsl_to_rgb(&self) -> (f64, f64, f64) {
        let hue = self.hue / 360.0;
        let saturation = self.saturation;
        let lightness = self.lightness;

        if saturation == 0.0 {
            return (lightness, lightness, lightness);
        }

        let upper_bound = if lightness < 0.5 {
            lightness * (1.0 + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };

        let lower_bound = 2.0 * lightness - upper_bound;

        let hue_to_rgb = |mut hue_offset: f64| -> f64 {
            if hue_offset < 0.0 {
                hue_offset += 1.0;
            }
            if hue_offset > 1.0 {
                hue_offset -= 1.0;
            }

            if hue_offset < 1.0 / 6.0 {
                lower_bound + (upper_bound - lower_bound) * 6.0 * hue_offset
            } else if hue_offset < 1.0 / 2.0 {
                upper_bound
            } else if hue_offset < 2.0 / 3.0 {
                lower_bound + (upper_bound - lower_bound) * (2.0 / 3.0 - hue_offset) * 6.0
            } else {
                lower_bound
            }
        };

        (
            hue_to_rgb(hue + 1.0 / 3.0),
            hue_to_rgb(hue),
            hue_to_rgb(hue - 1.0 / 3.0),
        )
    }
}

impl Color for HslColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        let (red, green, blue) = self.hsl_to_rgb();
        (red, green, blue, self.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.001
    }

    fn assert_rgba(s: &str, r: f64, g: f64, b: f64, a: f64) {
        let c = HslColor::from_str(s).unwrap_or_else(|e| panic!("parse failed for '{s}': {e}"));
        let (cr, cg, cb, ca) = c.to_rgb();
        assert!(approx_eq(cr, r), "red mismatch for '{s}': {cr} != {r}");
        assert!(approx_eq(cg, g), "green mismatch for '{s}': {cg} != {g}");
        assert!(approx_eq(cb, b), "blue mismatch for '{s}': {cb} != {b}");
        assert!(approx_eq(ca, a), "alpha mismatch for '{s}': {ca} != {a}");
    }

    // hsl(120, 75%, 25%) => (0.0625, 0.4375, 0.0625)
    #[test]
    fn test_deg_pct() {
        assert_rgba("120deg 75% 25%", 0.0625, 0.4375, 0.0625, 1.0);
    }

    #[test]
    fn test_plain_pct() {
        assert_rgba("120 75% 25%", 0.0625, 0.4375, 0.0625, 1.0);
    }

    #[test]
    fn test_deg_plain_sl() {
        assert_rgba("120deg 75 25", 0.0625, 0.4375, 0.0625, 1.0);
    }

    #[test]
    fn test_plain_sl() {
        assert_rgba("120 75 25", 0.0625, 0.4375, 0.0625, 1.0);
    }

    #[test]
    fn test_slash_alpha() {
        assert_rgba("120 75 25 / 0.2", 0.0625, 0.4375, 0.0625, 0.2);
    }

    #[test]
    fn test_slash_alpha_pct() {
        assert_rgba("120 75 25 / 20%", 0.0625, 0.4375, 0.0625, 0.2);
    }

    #[test]
    fn test_hue_wrapping() {
        // 480deg == 120deg
        assert_rgba("480 75 25", 0.0625, 0.4375, 0.0625, 1.0);
    }

    #[test]
    fn test_achromatic() {
        // hsl(0, 0%, 50%) => rgb(128, 128, 128) => (0.5, 0.5, 0.5)
        assert_rgba("0 0% 50%", 0.5, 0.5, 0.5, 1.0);
    }

    #[test]
    fn test_invalid_sl_range() {
        assert!(HslColor::from_str("120 150 25").is_err());
    }

    #[test]
    fn test_invalid_alpha_range() {
        assert!(HslColor::from_str("120 75 25 / 2.0").is_err());
    }

    #[test]
    fn test_invalid_format() {
        assert!(HslColor::from_str("not a color").is_err());
    }
}
