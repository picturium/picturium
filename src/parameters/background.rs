use log::error;
use serde::Serialize;

#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct Background(pub f64, pub f64, pub f64, pub f64);

impl Background {
    pub fn is_invisible(&self) -> bool {
        self.3 == 0.0
    }

    fn parse_hex(value: &str) -> Option<Self> {
        if value.len() != 6 && value.len() != 8 {
            error!("Invalid hex color format");
            return None;
        }

        let r = match u8::from_str_radix(&value[0..2], 16) {
            Ok(r) => (r as f64).clamp(0.0, 255.0),
            Err(_) => return None,
        };

        let g = match u8::from_str_radix(&value[2..4], 16) {
            Ok(g) => (g as f64).clamp(0.0, 255.0),
            Err(_) => return None,
        };

        let b = match u8::from_str_radix(&value[4..6], 16) {
            Ok(b) => (b as f64).clamp(0.0, 255.0),
            Err(_) => return None,
        };

        let a = match value.len() {
            8 => match u8::from_str_radix(&value[6..8], 16) {
                Ok(a) => (a as f64).clamp(0.0, 255.0),
                Err(_) => return None,
            },
            _ => 255.0,
        };

        Some(Background(r, g, b, a))
    }

    fn parse_rgb(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split(",").collect();

        if parts.len() != 3 && parts.len() != 4 {
            return None;
        }

        let r = match parts[0].parse::<f64>() {
            Ok(r) => r.clamp(0.0, 255.0),
            Err(_) => return None,
        };

        let g = match parts[1].parse::<f64>() {
            Ok(g) => g.clamp(0.0, 255.0),
            Err(_) => return None,
        };

        let b = match parts[2].parse::<f64>() {
            Ok(b) => b.clamp(0.0, 255.0),
            Err(_) => return None,
        };

        let a = match parts.len() {
            4 => match parts[3].parse::<f64>() {
                Ok(a) => (a / 100.0 * 255.0).clamp(0.0, 255.0),
                Err(_) => return None,
            },
            _ => 255.0,
        };

        Some(Background(r, g, b, a))
    }

    fn parse_hsl(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split(",").collect();

        if parts.len() != 3 && parts.len() != 4 {
            return None;
        }

        let h = match parts[0].parse::<f64>() {
            Ok(h) => h.clamp(0.0, 360.0) / 360.0,
            Err(_) => return None,
        };

        let s = match parts[1].parse::<f64>() {
            Ok(s) => (s / 100.0).clamp(0.0, 1.0),
            Err(_) => return None,
        };

        let l = match parts[2].parse::<f64>() {
            Ok(l) => (l / 100.0).clamp(0.0, 1.0),
            Err(_) => return None,
        };

        let a = match parts.len() {
            4 => match parts[3].parse::<f64>() {
                Ok(a) => (a / 100.0 * 255.0).clamp(0.0, 255.0),
                Err(_) => return None,
            },
            _ => 255.0,
        };

        if s == 0.0 {
            let l = l * 255.0;
            return Some(Background(l, l, l, a));
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };

        let p = l + l - q;
        let r = Self::hue_to_rgb(p, q, h + (1.0 / 3.0));
        let g = Self::hue_to_rgb(p, q, h);
        let b = Self::hue_to_rgb(p, q, h - (1.0 / 3.0));

        Some(Background(r * 255.0, g * 255.0, b * 255.0, a))
    }

    fn hue_to_rgb(p: f64, q: f64, mut h: f64) -> f64 {
        if h < 0.0 {
            h += 1.0;
        }

        if h > 1.0 {
            h -= 1.0;
        }

        if h < (1.0 / 6.0) {
            p + (q - p) * 6.0 * h
        } else if h < 0.5 {
            q
        } else if h < (2.0 / 3.0) {
            p + (q - p) * ((2.0 / 3.0) - h) * 6.0
        } else {
            p
        }
    }

    fn match_predefined(value: &str) -> Option<Self> {
        match value {
            "transparent" => Self::parse_hex("00000000"),
            "black" => Self::parse_hex("000000"),
            "silver" => Self::parse_hex("C0C0C0"),
            "gray" => Self::parse_hex("808080"),
            "white" => Self::parse_hex("FFFFFF"),
            "maroon" => Self::parse_hex("800000"),
            "red" => Self::parse_hex("FF0000"),
            "purple" => Self::parse_hex("800080"),
            "fuchsia" => Self::parse_hex("FF00FF"),
            "green" => Self::parse_hex("008000"),
            "lime" => Self::parse_hex("00FF00"),
            "olive" => Self::parse_hex("808000"),
            "yellow" => Self::parse_hex("FFFF00"),
            "navy" => Self::parse_hex("000080"),
            "blue" => Self::parse_hex("0000FF"),
            "teal" => Self::parse_hex("008080"),
            "aqua" => Self::parse_hex("00FFFF"),
            _ => {
                error!("Undefined background color: {value}");
                None
            }
        }
    }

    pub fn from(value: &Option<String>) -> Option<Self> {
        if value.is_none() {
            return None;
        }

        let value = value.as_ref().unwrap().to_lowercase();
        let value = value.replace("%", "");
        let parts = value.split(':').collect::<Vec<&str>>();

        if parts.len() != 2 {
            return Self::match_predefined(&value);
        }

        let prefix = parts[0].trim();
        let value = parts[1].trim();

        match prefix {
            "hex" => Self::parse_hex(value),
            "rgb" => Self::parse_rgb(value),
            "hsl" => Self::parse_hsl(value),
            _ => {
                error!("Invalid background color format: {value}");
                None
            }
        }
    }
}

impl From<&Background> for Vec<f64> {
    fn from(value: &Background) -> Self {
        vec![value.0, value.1, value.2, value.3]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_background_from_predefined() {
        assert_eq!(
            Background::from(&Some("transparent".to_string())),
            Some(Background(0.0, 0.0, 0.0, 0.0))
        );
        assert_eq!(
            Background::from(&Some("red".to_string())),
            Some(Background(255.0, 0.0, 0.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("maroon".to_string())),
            Some(Background(128.0, 0.0, 0.0, 255.0))
        );
        assert_eq!(Background::from(&Some("cyan".to_string())), None);
    }

    #[test]
    fn test_background_from_hex() {
        assert_eq!(
            Background::from(&Some("hex:ff0000".to_string())),
            Some(Background(255.0, 0.0, 0.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("hex:00ff00".to_string())),
            Some(Background(0.0, 255.0, 0.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("hex:0000ff".to_string())),
            Some(Background(0.0, 0.0, 255.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("hex:497dbcb0".to_string())),
            Some(Background(73.0, 125.0, 188.0, 176.0))
        );
    }

    #[test]
    fn test_background_from_rgb() {
        assert_eq!(
            Background::from(&Some("rgb:255,0,0".to_string())),
            Some(Background(255.0, 0.0, 0.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("rgb:0,255,0".to_string())),
            Some(Background(0.0, 255.0, 0.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("rgb:0,0,255".to_string())),
            Some(Background(0.0, 0.0, 255.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("rgb:73,125,188,69%".to_string())),
            Some(Background(73.0, 125.0, 188.0, 175.95))
        );
    }

    #[test]
    fn test_background_from_hsl() {
        assert_eq!(
            Background::from(&Some("hsl:0,100%,50%".to_string())),
            Some(Background(255.0, 0.0, 0.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("hsl:120,100%,50%".to_string())),
            Some(Background(0.0, 255.0, 0.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("hsl:240,100%,50%".to_string())),
            Some(Background(0.0, 0.0, 255.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("hsl:222,100%,40%".to_string())),
            Some(Background(0.0, 61.199999999999925, 204.0, 255.0))
        );
        assert_eq!(
            Background::from(&Some("hsl:240,100%,50%,50%".to_string())),
            Some(Background(0.0, 0.0, 255.0, 127.5))
        );
    }

    #[test]
    fn it_checks_background_visibility() {
        assert_eq!(
            Background::from(&Some("black".to_string()))
                .unwrap()
                .is_invisible(),
            false
        );
        assert_eq!(
            Background::from(&Some("transparent".to_string()))
                .unwrap()
                .is_invisible(),
            true
        );
        assert_eq!(
            Background::from(&Some("hex:45ef00aa".to_string()))
                .unwrap()
                .is_invisible(),
            false
        );
        assert_eq!(
            Background::from(&Some("hex:45ef0000".to_string()))
                .unwrap()
                .is_invisible(),
            true
        );
    }
}
