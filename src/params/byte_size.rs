use serde::de::Visitor;
use serde::{de, Deserializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ByteSize(pub usize);

#[derive(Debug)]
pub struct ByteSizeParseError(String);

impl fmt::Display for ByteSizeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ByteSizeParseError {}

impl FromStr for ByteSize {
    type Err = ByteSizeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let amount = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let unit = s[amount.len()..].to_ascii_lowercase();

        let multiplier: usize = match unit.as_str() {
            "" | "b" => 1,
            "k" | "kb" | "kib" => 1024,
            "m" | "mb" | "mib" => 1024 * 1024,
            "g" | "gb" | "gib" => 1024 * 1024 * 1024,
            _ => return Err(ByteSizeParseError(format!("Unknown size unit: '{unit}'"))),
        };

        let amount: f64 = amount
            .trim()
            .parse()
            .map_err(|_| ByteSizeParseError(format!("Invalid size value: '{s}'")))?;

        if !amount.is_finite() || amount < 0.0 {
            return Err(ByteSizeParseError(format!("Invalid size value: '{s}'")));
        }

        Ok(ByteSize((amount * multiplier as f64) as usize))
    }
}

impl<'de> serde::Deserialize<'de> for ByteSize {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = ByteSize;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a byte count, plain or with a unit suffix such as 500K or 2M")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(ByteSize(v as usize))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                if v < 0 {
                    return Err(de::Error::custom(format!("Invalid size value: '{v}'")));
                }

                Ok(ByteSize(v as usize))
            }
        }

        d.deserialize_any(V)
    }
}

pub fn deserialize_usize<'de, D: Deserializer<'de>>(d: D) -> Result<usize, D::Error> {
    use serde::Deserialize;
    Ok(ByteSize::deserialize(d)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> usize {
        s.parse::<ByteSize>().expect(s).0
    }

    #[test]
    fn a_bare_number_is_bytes() {
        assert_eq!(parse("0"), 0);
        assert_eq!(parse("120000"), 120000);
        assert_eq!(parse("512B"), 512);
    }

    #[test]
    fn suffixes_are_binary_and_case_insensitive() {
        assert_eq!(parse("500K"), 500 * 1024);
        assert_eq!(parse("500k"), 500 * 1024);
        assert_eq!(parse("500kb"), 500 * 1024);
        assert_eq!(parse("500KiB"), 500 * 1024);
        assert_eq!(parse("2M"), 2 * 1024 * 1024);
        assert_eq!(parse("1G"), 1024 * 1024 * 1024);
    }

    #[test]
    fn accepts_a_fractional_amount() {
        assert_eq!(parse("1.5M"), 1024 * 1024 * 3 / 2);
        assert_eq!(parse("0.5K"), 512);
    }

    #[test]
    fn rejects_junk() {
        for input in ["", "M", "abc", "-5", "5X", "5 M B", "1.5.5K"] {
            assert!(input.parse::<ByteSize>().is_err(), "accepted '{input}'");
        }
    }
}
