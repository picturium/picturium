use crate::params::colors::{Color, ColorParseError};
use std::str::FromStr;

pub struct HexColor {
    red: f64, // [0.0, 1.0]
    green: f64, // [0.0, 1.0]
    blue: f64, // [0.0, 1.0]
    alpha: f64, // [0.0, 1.0]
}

impl FromStr for HexColor {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse_hex = |digits: &str| {
            u8::from_str_radix(digits, 16)
                .map_err(|_| ColorParseError(digits.to_string()))
        };

        // Expand a single hex digit to a full byte by duplicating it: 'a' -> "aa"
        let expand = |c: char| {
            let mut buf = [0u8; 4];
            let digit = c.encode_utf8(&mut buf);
            let doubled = format!("{digit}{digit}");

            parse_hex(&doubled)
        };

        let chars: Vec<char> = s.chars().collect();

        let (red, green, blue, alpha) = match chars.as_slice() {
            // #RGB
            [r, g, b] => (expand(*r)?, expand(*g)?, expand(*b)?, 255u8),
            // #RGBA
            [r, g, b, a] => (expand(*r)?, expand(*g)?, expand(*b)?, expand(*a)?),
            // #RRGGBB
            [_, _, _, _, _, _] => (
                parse_hex(&s[0..2])?, parse_hex(&s[2..4])?, parse_hex(&s[4..6])?, 255u8
            ),
            // #RRGGBBAA
            [_, _, _, _, _, _, _, _] => (
                parse_hex(&s[0..2])?, parse_hex(&s[2..4])?, parse_hex(&s[4..6])?, parse_hex(&s[6..8])?
            ),
            _ => return Err(ColorParseError(s.to_string())),
        };

        Ok(Self {
            red: red as f64 / 255.0,
            green: green as f64 / 255.0,
            blue: blue as f64 / 255.0,
            alpha: alpha as f64 / 255.0,
        })
    }
}

impl Color for HexColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        (self.red, self.green, self.blue, self.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn assert_rgba(s: &str, r: f64, g: f64, b: f64, a: f64) {
        let c = HexColor::from_str(s).unwrap_or_else(|e| panic!("parse failed for '{s}': {e}"));
        let (cr, cg, cb, ca) = c.to_rgb();
        assert_eq!(cr, r, "red mismatch for '{s}'");
        assert_eq!(cg, g, "green mismatch for '{s}'");
        assert_eq!(cb, b, "blue mismatch for '{s}'");
        assert_eq!(ca, a, "alpha mismatch for '{s}'");
    }

    // --- #RGB ---
    #[test]
    fn test_rgb_short() {
        // f -> ff = 255, 8 -> 88 = 136, 0 -> 00 = 0
        assert_rgba("f80", 255.0 / 255.0, 136.0 / 255.0, 0.0 / 255.0, 255.0 / 255.0);
    }

    #[test]
    fn test_rgb_short_lowercase() {
        assert_rgba("abc", 170.0 / 255.0, 187.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0);
    }

    #[test]
    fn test_rgb_short_uppercase() {
        assert_rgba("ABC", 170.0 / 255.0, 187.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0);
    }

    // --- #RGBA ---
    #[test]
    fn test_rgba_short() {
        // f -> ff = 255, 8 -> 88 = 136, 0 -> 00 = 0, 8 -> 88 = 136
        assert_rgba("f808", 255.0 / 255.0, 136.0 / 255.0, 0.0 / 255.0, 136.0 / 255.0);
    }

    // --- #RRGGBB ---
    #[test]
    fn test_rrggbb() {
        assert_rgba("ff8800", 255.0 / 255.0, 136.0 / 255.0, 0.0 / 255.0, 255.0 / 255.0);
    }

    #[test]
    fn test_rrggbb_black() {
        assert_rgba("000000", 0.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0, 255.0 / 255.0);
    }

    #[test]
    fn test_rrggbb_white() {
        assert_rgba("ffffff", 255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0);
    }

    #[test]
    fn test_rrggbb_uppercase() {
        assert_rgba("FF8800", 255.0 / 255.0, 136.0 / 255.0, 0.0 / 255.0, 255.0 / 255.0);
    }

    // --- #RRGGBBAA ---
    #[test]
    fn test_rrggbbaa() {
        assert_rgba("ff880080", 255.0 / 255.0, 136.0 / 255.0, 0.0 / 255.0, 128.0 / 255.0);
    }

    #[test]
    fn test_rrggbbaa_fully_transparent() {
        assert_rgba("ffffff00", 255.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0, 0.0 / 255.0);
    }

    #[test]
    fn test_rrggbbaa_fully_opaque() {
        assert_rgba("000000ff", 0.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0, 255.0 / 255.0);
    }

    // --- invalid inputs ---
    #[test]
    fn test_invalid_too_short() {
        assert!(HexColor::from_str("ff").is_err());
    }

    #[test]
    fn test_invalid_too_long() {
        assert!(HexColor::from_str("fffffffff").is_err());
    }

    #[test]
    fn test_invalid_non_hex_chars() {
        assert!(HexColor::from_str("zzzzzz").is_err());
    }

    #[test]
    fn test_invalid_five_chars() {
        assert!(HexColor::from_str("fffff").is_err());
    }

    #[test]
    fn test_invalid_seven_chars() {
        assert!(HexColor::from_str("fffffff").is_err());
    }
}