use std::str::FromStr;
use crate::params::colors::{Color, ColorParseError};

pub struct RgbColor {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

pub type RgbColorParseError = ColorParseError;

/// Parses a single channel value, either as a percentage ("75%") or a plain number.
/// Percentages are normalized to [0.0, 1.0]; plain values are divided by 255.
/// Returns an error if the value is outside the valid range.
fn parse_channel(raw: &str) -> Result<f64, RgbColorParseError> {
    let err = || ColorParseError(format!("invalid channel value: '{raw}'"));

    if let Some(pct) = raw.strip_suffix('%') {
        let v: f64 = pct.trim().parse().map_err(|_| err())?;
        if !(0.0..=100.0).contains(&v) {
            return Err(ColorParseError(format!("channel percentage out of range: '{raw}'")));
        }
        Ok(v / 100.0)
    } else {
        let v: f64 = raw.trim().parse().map_err(|_| err())?;
        if !(0.0..=255.0).contains(&v) {
            return Err(ColorParseError(format!("channel value out of range: '{raw}'")));
        }
        Ok(v / 255.0)
    }
}

/// Parses an alpha value, either as a percentage ("75%") or a plain number in [0, 1].
fn parse_alpha(raw: &str) -> Result<f64, RgbColorParseError> {
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

impl FromStr for RgbColor {
    type Err = RgbColorParseError;

    /// Accepted formats:
    /// - `"r g b"`            plain space-separated
    /// - `"r g b a"`          space-separated with alpha in [0, 1]
    /// - `"r g b / a"`        slash-separated alpha in [0, 1]
    /// - `"r g b / a%"`       slash-separated alpha as percentage
    /// - `"r% g% b%"`         percentage channels
    /// - `"r% g% b% / a"`     percentage channels with slash alpha
    /// - `"r% g% b% / a%"`    percentage channels with slash alpha as percentage
    /// - `"r,g,b"`            comma-separated
    /// - `"r,g,b,a"`          comma-separated with alpha in [0, 1]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(format!("invalid RGB color: '{s}'"));

        // --- comma-separated branch: "r,g,b" or "r,g,b,a" ---
        if s.contains(',') {
            let parts: Vec<&str> = s.split(',').map(str::trim).collect();
            return match parts.as_slice() {
                [r, g, b] => Ok(Self {
                    red:   parse_channel(r)?,
                    green: parse_channel(g)?,
                    blue:  parse_channel(b)?,
                    alpha: 1.0,
                }),
                [r, g, b, a] => Ok(Self {
                    red:   parse_channel(r)?,
                    green: parse_channel(g)?,
                    blue:  parse_channel(b)?,
                    alpha: parse_alpha(a)?,
                }),
                _ => Err(err()),
            };
        }

        // --- slash-separated alpha branch: "r g b / a[%]" ---
        if let Some((color_part, alpha_part)) = s.split_once('/') {
            let channels: Vec<&str> = color_part.split_whitespace().collect();
            let alpha_raw = alpha_part.trim();

            return match channels.as_slice() {
                [r, g, b] => Ok(Self {
                    red:   parse_channel(r)?,
                    green: parse_channel(g)?,
                    blue:  parse_channel(b)?,
                    alpha: parse_alpha(alpha_raw)?,
                }),
                _ => Err(err()),
            };
        }

        // --- plain space-separated branch: "r g b" or "r g b a" ---
        let parts: Vec<&str> = s.split_whitespace().collect();
        match parts.as_slice() {
            [r, g, b] => Ok(Self {
                red:   parse_channel(r)?,
                green: parse_channel(g)?,
                blue:  parse_channel(b)?,
                alpha: 1.0,
            }),
            [r, g, b, a] => Ok(Self {
                red:   parse_channel(r)?,
                green: parse_channel(g)?,
                blue:  parse_channel(b)?,
                alpha: parse_alpha(a)?,
            }),
            _ => Err(err()),
        }
    }
}

impl Color for RgbColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        (self.red, self.green, self.blue, self.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn assert_rgba(s: &str, r: f64, g: f64, b: f64, a: f64) {
        let c = RgbColor::from_str(s).unwrap_or_else(|e| panic!("parse failed for '{s}': {e}"));
        let (cr, cg, cb, ca) = c.to_rgb();
        assert!(approx_eq(cr, r), "red mismatch for '{s}': {cr} != {r}");
        assert!(approx_eq(cg, g), "green mismatch for '{s}': {cg} != {g}");
        assert!(approx_eq(cb, b), "blue mismatch for '{s}': {cb} != {b}");
        assert!(approx_eq(ca, a), "alpha mismatch for '{s}': {ca} != {a}");
    }

    #[test]
    fn test_space_rgb() {
        assert_rgba("255 128 0", 1.0, 128.0 / 255.0, 0.0, 1.0);
    }

    #[test]
    fn test_space_rgba() {
        assert_rgba("255 128 0 0.5", 1.0, 128.0 / 255.0, 0.0, 0.5);
    }

    #[test]
    fn test_slash_alpha() {
        assert_rgba("255 128 0 / 0.5", 1.0, 128.0 / 255.0, 0.0, 0.5);
    }

    #[test]
    fn test_slash_alpha_pct() {
        assert_rgba("255 128 0 / 50%", 1.0, 128.0 / 255.0, 0.0, 0.5);
    }

    #[test]
    fn test_pct_channels() {
        assert_rgba("100% 50% 0%", 1.0, 0.5, 0.0, 1.0);
    }

    #[test]
    fn test_pct_channels_slash_alpha() {
        assert_rgba("100% 50% 0% / 0.5", 1.0, 0.5, 0.0, 0.5);
    }

    #[test]
    fn test_pct_channels_slash_alpha_pct() {
        assert_rgba("100% 50% 0% / 50%", 1.0, 0.5, 0.0, 0.5);
    }

    #[test]
    fn test_comma_rgb() {
        assert_rgba("255,128,0", 1.0, 128.0 / 255.0, 0.0, 1.0);
    }

    #[test]
    fn test_comma_rgba() {
        assert_rgba("255,128,0,0.5", 1.0, 128.0 / 255.0, 0.0, 0.5);
    }

    #[test]
    fn test_invalid_channel_range() {
        assert!(RgbColor::from_str("300 0 0").is_err());
    }

    #[test]
    fn test_invalid_alpha_range() {
        assert!(RgbColor::from_str("255 0 0 / 2.0").is_err());
    }

    #[test]
    fn test_invalid_format() {
        assert!(RgbColor::from_str("not a color").is_err());
    }
}
