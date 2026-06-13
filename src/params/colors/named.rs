use std::str::FromStr;
use crate::params::colors::{Color, ColorParseError};

pub struct NamedColor {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

pub type NamedColorParseError = ColorParseError;

impl FromStr for NamedColor {
    type Err = NamedColorParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (r, g, b, a): (u8, u8, u8, u8) = match s {
            "aliceblue"            => (0xf0, 0xf8, 0xff, 0xff),
            "antiquewhite"         => (0xfa, 0xeb, 0xd7, 0xff),
            "aqua" | "cyan"        => (0x00, 0xff, 0xff, 0xff),
            "aquamarine"           => (0x7f, 0xff, 0xd4, 0xff),
            "azure"                => (0xf0, 0xff, 0xff, 0xff),
            "beige"                => (0xf5, 0xf5, 0xdc, 0xff),
            "bisque"               => (0xff, 0xe4, 0xc4, 0xff),
            "black"                => (0x00, 0x00, 0x00, 0xff),
            "blanchedalmond"       => (0xff, 0xeb, 0xcd, 0xff),
            "blue"                 => (0x00, 0x00, 0xff, 0xff),
            "blueviolet"           => (0x8a, 0x2b, 0xe2, 0xff),
            "brown"                => (0xa5, 0x2a, 0x2a, 0xff),
            "burlywood"            => (0xde, 0xb8, 0x87, 0xff),
            "cadetblue"            => (0x5f, 0x9e, 0xa0, 0xff),
            "chartreuse"           => (0x7f, 0xff, 0x00, 0xff),
            "chocolate"            => (0xd2, 0x69, 0x1e, 0xff),
            "coral"                => (0xff, 0x7f, 0x50, 0xff),
            "cornflowerblue"       => (0x64, 0x95, 0xed, 0xff),
            "cornsilk"             => (0xff, 0xf8, 0xdc, 0xff),
            "crimson"              => (0xdc, 0x14, 0x3c, 0xff),
            "darkblue"             => (0x00, 0x00, 0x8b, 0xff),
            "darkcyan"             => (0x00, 0x8b, 0x8b, 0xff),
            "darkgoldenrod"        => (0xb8, 0x86, 0x0b, 0xff),
            "darkgray" | "darkgrey"=> (0xa9, 0xa9, 0xa9, 0xff),
            "darkgreen"            => (0x00, 0x64, 0x00, 0xff),
            "darkkhaki"            => (0xbd, 0xb7, 0x6b, 0xff),
            "darkmagenta"          => (0x8b, 0x00, 0x8b, 0xff),
            "darkolivegreen"       => (0x55, 0x6b, 0x2f, 0xff),
            "darkorange"           => (0xff, 0x8c, 0x00, 0xff),
            "darkorchid"           => (0x99, 0x32, 0xcc, 0xff),
            "darkred"              => (0x8b, 0x00, 0x00, 0xff),
            "darksalmon"           => (0xe9, 0x96, 0x7a, 0xff),
            "darkseagreen"         => (0x8f, 0xbc, 0x8f, 0xff),
            "darkslateblue"        => (0x48, 0x3d, 0x8b, 0xff),
            "darkslategray" | "darkslategrey" => (0x2f, 0x4f, 0x4f, 0xff),
            "darkturquoise"        => (0x00, 0xce, 0xd1, 0xff),
            "darkviolet"           => (0x94, 0x00, 0xd3, 0xff),
            "deeppink"             => (0xff, 0x14, 0x93, 0xff),
            "deepskyblue"          => (0x00, 0xbf, 0xff, 0xff),
            "dimgray" | "dimgrey"  => (0x69, 0x69, 0x69, 0xff),
            "dodgerblue"           => (0x1e, 0x90, 0xff, 0xff),
            "firebrick"            => (0xb2, 0x22, 0x22, 0xff),
            "floralwhite"          => (0xff, 0xfa, 0xf0, 0xff),
            "forestgreen"          => (0x22, 0x8b, 0x22, 0xff),
            "fuchsia" | "magenta"  => (0xff, 0x00, 0xff, 0xff),
            "gainsboro"            => (0xdc, 0xdc, 0xdc, 0xff),
            "ghostwhite"           => (0xf8, 0xf8, 0xff, 0xff),
            "gold"                 => (0xff, 0xd7, 0x00, 0xff),
            "goldenrod"            => (0xda, 0xa5, 0x20, 0xff),
            "gray" | "grey"        => (0x80, 0x80, 0x80, 0xff),
            "green"                => (0x00, 0x80, 0x00, 0xff),
            "greenyellow"          => (0xad, 0xff, 0x2f, 0xff),
            "honeydew"             => (0xf0, 0xff, 0xf0, 0xff),
            "hotpink"              => (0xff, 0x69, 0xb4, 0xff),
            "indianred"            => (0xcd, 0x5c, 0x5c, 0xff),
            "indigo"               => (0x4b, 0x00, 0x82, 0xff),
            "ivory"                => (0xff, 0xff, 0xf0, 0xff),
            "khaki"                => (0xf0, 0xe6, 0x8c, 0xff),
            "lavender"             => (0xe6, 0xe6, 0xfa, 0xff),
            "lavenderblush"        => (0xff, 0xf0, 0xf5, 0xff),
            "lawngreen"            => (0x7c, 0xfc, 0x00, 0xff),
            "lemonchiffon"         => (0xff, 0xfa, 0xcd, 0xff),
            "lightblue"            => (0xad, 0xd8, 0xe6, 0xff),
            "lightcoral"           => (0xf0, 0x80, 0x80, 0xff),
            "lightcyan"            => (0xe0, 0xff, 0xff, 0xff),
            "lightgoldenrodyellow" => (0xfa, 0xfa, 0xd2, 0xff),
            "lightgray" | "lightgrey" => (0xd3, 0xd3, 0xd3, 0xff),
            "lightgreen"           => (0x90, 0xee, 0x90, 0xff),
            "lightpink"            => (0xff, 0xb6, 0xc1, 0xff),
            "lightsalmon"          => (0xff, 0xa0, 0x7a, 0xff),
            "lightseagreen"        => (0x20, 0xb2, 0xaa, 0xff),
            "lightskyblue"         => (0x87, 0xce, 0xfa, 0xff),
            "lightslategray" | "lightslategrey" => (0x77, 0x88, 0x99, 0xff),
            "lightsteelblue"       => (0xb0, 0xc4, 0xde, 0xff),
            "lightyellow"          => (0xff, 0xff, 0xe0, 0xff),
            "lime"                 => (0x00, 0xff, 0x00, 0xff),
            "limegreen"            => (0x32, 0xcd, 0x32, 0xff),
            "linen"                => (0xfa, 0xf0, 0xe6, 0xff),
            "maroon"               => (0x80, 0x00, 0x00, 0xff),
            "mediumaquamarine"     => (0x66, 0xcd, 0xaa, 0xff),
            "mediumblue"           => (0x00, 0x00, 0xcd, 0xff),
            "mediumorchid"         => (0xba, 0x55, 0xd3, 0xff),
            "mediumpurple"         => (0x93, 0x70, 0xdb, 0xff),
            "mediumseagreen"       => (0x3c, 0xb3, 0x71, 0xff),
            "mediumslateblue"      => (0x7b, 0x68, 0xee, 0xff),
            "mediumspringgreen"    => (0x00, 0xfa, 0x9a, 0xff),
            "mediumturquoise"      => (0x48, 0xd1, 0xcc, 0xff),
            "mediumvioletred"      => (0xc7, 0x15, 0x85, 0xff),
            "midnightblue"         => (0x19, 0x19, 0x70, 0xff),
            "mintcream"            => (0xf5, 0xff, 0xfa, 0xff),
            "mistyrose"            => (0xff, 0xe4, 0xe1, 0xff),
            "moccasin"             => (0xff, 0xe4, 0xb5, 0xff),
            "navajowhite"          => (0xff, 0xde, 0xad, 0xff),
            "navy"                 => (0x00, 0x00, 0x80, 0xff),
            "oldlace"              => (0xfd, 0xf5, 0xe6, 0xff),
            "olive"                => (0x80, 0x80, 0x00, 0xff),
            "olivedrab"            => (0x6b, 0x8e, 0x23, 0xff),
            "orange"               => (0xff, 0xa5, 0x00, 0xff),
            "orangered"            => (0xff, 0x45, 0x00, 0xff),
            "orchid"               => (0xda, 0x70, 0xd6, 0xff),
            "palegoldenrod"        => (0xee, 0xe8, 0xaa, 0xff),
            "palegreen"            => (0x98, 0xfb, 0x98, 0xff),
            "paleturquoise"        => (0xaf, 0xee, 0xee, 0xff),
            "palevioletred"        => (0xdb, 0x70, 0x93, 0xff),
            "papayawhip"           => (0xff, 0xef, 0xd5, 0xff),
            "peachpuff"            => (0xff, 0xda, 0xb9, 0xff),
            "peru"                 => (0xcd, 0x85, 0x3f, 0xff),
            "pink"                 => (0xff, 0xc0, 0xcb, 0xff),
            "plum"                 => (0xdd, 0xa0, 0xdd, 0xff),
            "powderblue"           => (0xb0, 0xe0, 0xe6, 0xff),
            "purple"               => (0x80, 0x00, 0x80, 0xff),
            "rebeccapurple"        => (0x66, 0x33, 0x99, 0xff),
            "red"                  => (0xff, 0x00, 0x00, 0xff),
            "rosybrown"            => (0xbc, 0x8f, 0x8f, 0xff),
            "royalblue"            => (0x41, 0x69, 0xe1, 0xff),
            "saddlebrown"          => (0x8b, 0x45, 0x13, 0xff),
            "salmon"               => (0xfa, 0x80, 0x72, 0xff),
            "sandybrown"           => (0xf4, 0xa4, 0x60, 0xff),
            "seagreen"             => (0x2e, 0x8b, 0x57, 0xff),
            "seashell"             => (0xff, 0xf5, 0xee, 0xff),
            "sienna"               => (0xa0, 0x52, 0x2d, 0xff),
            "silver"               => (0xc0, 0xc0, 0xc0, 0xff),
            "skyblue"              => (0x87, 0xce, 0xeb, 0xff),
            "slateblue"            => (0x6a, 0x5a, 0xcd, 0xff),
            "slategray" | "slategrey" => (0x70, 0x80, 0x90, 0xff),
            "snow"                 => (0xff, 0xfa, 0xfa, 0xff),
            "springgreen"          => (0x00, 0xff, 0x7f, 0xff),
            "steelblue"            => (0x46, 0x82, 0xb4, 0xff),
            "tan"                  => (0xd2, 0xb4, 0x8c, 0xff),
            "teal"                 => (0x00, 0x80, 0x80, 0xff),
            "thistle"              => (0xd8, 0xbf, 0xd8, 0xff),
            "tomato"               => (0xff, 0x63, 0x47, 0xff),
            "transparent"          => (0x00, 0x00, 0x00, 0x00),
            "turquoise"            => (0x40, 0xe0, 0xd0, 0xff),
            "violet"               => (0xee, 0x82, 0xee, 0xff),
            "wheat"                => (0xf5, 0xde, 0xb3, 0xff),
            "white"                => (0xff, 0xff, 0xff, 0xff),
            "whitesmoke"           => (0xf5, 0xf5, 0xf5, 0xff),
            "yellow"               => (0xff, 0xff, 0x00, 0xff),
            "yellowgreen"          => (0x9a, 0xcd, 0x32, 0xff),
            _ => return Err(ColorParseError(s.to_string())),
        };

        Ok(Self {
            red:   r as f64 / 255.0,
            green: g as f64 / 255.0,
            blue:  b as f64 / 255.0,
            alpha: a as f64 / 255.0,
        })
    }
}

impl Color for NamedColor {
    fn to_rgb(&self) -> (f64, f64, f64, f64) {
        (self.red, self.green, self.blue, self.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn assert_rgba(s: &str, r: f64, g: f64, b: f64, a: f64) {
        let c = NamedColor::from_str(s).unwrap_or_else(|e| panic!("parse failed for '{s}': {e}"));
        let (cr, cg, cb, ca) = c.to_rgb();
        assert_eq!(cr, r, "red mismatch for '{s}'");
        assert_eq!(cg, g, "green mismatch for '{s}'");
        assert_eq!(cb, b, "blue mismatch for '{s}'");
        assert_eq!(ca, a, "alpha mismatch for '{s}'");
    }

    // --- basic named colors ---
    #[test]
    fn test_black() {
        assert_rgba("black", 0.0, 0.0, 0.0, 255.0);
    }

    #[test]
    fn test_white() {
        assert_rgba("white", 255.0, 255.0, 255.0, 255.0);
    }

    #[test]
    fn test_red() {
        assert_rgba("red", 255.0, 0.0, 0.0, 255.0);
    }

    #[test]
    fn test_green() {
        assert_rgba("green", 0.0, 128.0, 0.0, 255.0);
    }

    #[test]
    fn test_blue() {
        assert_rgba("blue", 0.0, 0.0, 255.0, 255.0);
    }

    #[test]
    fn test_lime() {
        assert_rgba("lime", 0.0, 255.0, 0.0, 255.0);
    }

    // --- transparent ---
    #[test]
    fn test_transparent() {
        assert_rgba("transparent", 0.0, 0.0, 0.0, 0.0);
    }

    // --- alias pairs ---
    #[test]
    fn test_aqua_cyan_alias() {
        let a = NamedColor::from_str("aqua").unwrap().to_rgb();
        let b = NamedColor::from_str("cyan").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_fuchsia_magenta_alias() {
        let a = NamedColor::from_str("fuchsia").unwrap().to_rgb();
        let b = NamedColor::from_str("magenta").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_gray_grey_alias() {
        let a = NamedColor::from_str("gray").unwrap().to_rgb();
        let b = NamedColor::from_str("grey").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_darkgray_darkgrey_alias() {
        let a = NamedColor::from_str("darkgray").unwrap().to_rgb();
        let b = NamedColor::from_str("darkgrey").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_darkslategray_darkslategrey_alias() {
        let a = NamedColor::from_str("darkslategray").unwrap().to_rgb();
        let b = NamedColor::from_str("darkslategrey").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_dimgray_dimgrey_alias() {
        let a = NamedColor::from_str("dimgray").unwrap().to_rgb();
        let b = NamedColor::from_str("dimgrey").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_lightgray_lightgrey_alias() {
        let a = NamedColor::from_str("lightgray").unwrap().to_rgb();
        let b = NamedColor::from_str("lightgrey").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_lightslategray_lightslategrey_alias() {
        let a = NamedColor::from_str("lightslategray").unwrap().to_rgb();
        let b = NamedColor::from_str("lightslategrey").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    #[test]
    fn test_slategray_slategrey_alias() {
        let a = NamedColor::from_str("slategray").unwrap().to_rgb();
        let b = NamedColor::from_str("slategrey").unwrap().to_rgb();
        assert_eq!(a, b);
    }

    // --- case insensitivity ---
    #[test]
    fn test_uppercase() {
        assert_rgba("RED", 255.0, 0.0, 0.0, 255.0);
    }

    #[test]
    fn test_mixed_case() {
        assert_rgba("AliceBlue", 240.0, 248.0, 255.0, 255.0);
    }

    // --- spot-check a few specific colors against their CSS values ---
    #[test]
    fn test_aliceblue() {
        assert_rgba("aliceblue", 0xf0 as f64, 0xf8 as f64, 0xff as f64, 255.0);
    }

    #[test]
    fn test_rebeccapurple() {
        assert_rgba("rebeccapurple", 0x66 as f64, 0x33 as f64, 0x99 as f64, 255.0);
    }

    #[test]
    fn test_coral() {
        assert_rgba("coral", 0xff as f64, 0x7f as f64, 0x50 as f64, 255.0);
    }

    // --- invalid names ---
    #[test]
    fn test_invalid_name() {
        assert!(NamedColor::from_str("notacolor").is_err());
    }

    #[test]
    fn test_invalid_empty() {
        assert!(NamedColor::from_str("").is_err());
    }

    #[test]
    fn test_invalid_hex_string() {
        assert!(NamedColor::from_str("#ff0000").is_err());
    }
}
