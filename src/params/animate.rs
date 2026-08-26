use std::fmt;
use std::str::FromStr;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;

#[derive(Debug, Clone, Default)]
pub struct Animate {
    pub frames: Option<i16>,
    pub timing: Option<u16>,
}

#[derive(Debug)]
pub struct AnimateParseError(String);

impl fmt::Display for AnimateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AnimateParseError {}

impl FromStr for Animate {
    type Err = AnimateParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('|')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut animate = Animate::default();

        for part in parts {
            let (key, value) = part.split_once(':').ok_or_else(|| AnimateParseError(format!("Missing ':' in animate segment '{part}'")))?;

            match key {
                "frames" => animate.frames = Some(
                    value.parse::<i16>().map_err(|_| AnimateParseError(format!("Invalid frames value: '{value}'")))?,
                ),
                "timing" => {
                    let timing = value.parse::<u16>().map_err(|_| AnimateParseError(format!("Invalid timing value: '{value}'")))?;

                    if !(5..=5000).contains(&timing) {
                        return Err(AnimateParseError(format!("Timing value must be between 5 and 5000 ms, got '{timing}'")));
                    }

                    animate.timing = Some(timing);
                },
                _ => return Err(AnimateParseError(format!("Unknown animate key: '{key}'"))),
            }
        }

        Ok(animate)
    }
}

impl<'de> Deserialize<'de> for Animate {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Animate;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "animate parameters in format anim=frames:10|timing:500")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_keys() {
        let animate: Animate = "frames:10|timing:500".parse().unwrap();
        assert_eq!(animate.frames, Some(10));
        assert_eq!(animate.timing, Some(500));
    }

    #[test]
    fn rejects_timing_outside_the_supported_range() {
        assert!("timing:1".parse::<Animate>().is_err());
        assert!("timing:9999".parse::<Animate>().is_err());
    }

    #[test]
    fn rejects_unknown_keys_and_missing_separators() {
        assert!("bogus:1".parse::<Animate>().is_err());
        assert!("10".parse::<Animate>().is_err());
    }
}
