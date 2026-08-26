use serde::de::Visitor;
use serde::{Deserialize, Deserializer, de};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Time(pub String);

#[derive(Debug)]
pub struct TimeParseError(String);

impl fmt::Display for TimeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TimeParseError {}

impl FromStr for Time {
    type Err = TimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let is_valid = s.starts_with(|c: char| c.is_ascii_digit())
            && s.chars().all(|c| c.is_ascii_digit() || c == ':' || c == '.');

        match is_valid {
            true => Ok(Self(s.to_owned())),
            false => Err(TimeParseError(format!(
                "Invalid time value: '{s}', expected seconds or a timecode, eg. 5, 5.25 or 00:01:30"
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Time;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a time in seconds or as a timecode, eg. t=5 or t=00:01:30")
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
    fn accepts_seconds_and_timecodes() {
        assert_eq!("5".parse::<Time>().unwrap().0, "5");
        assert_eq!("5.25".parse::<Time>().unwrap().0, "5.25");
        assert_eq!("00:01:30.5".parse::<Time>().unwrap().0, "00:01:30.5");
    }

    #[test]
    fn rejects_anything_ffmpeg_could_read_as_a_flag_or_garbage() {
        assert!("-5".parse::<Time>().is_err());
        assert!("".parse::<Time>().is_err());
        assert!("abc".parse::<Time>().is_err());
        assert!("5s".parse::<Time>().is_err());
        assert!(":30".parse::<Time>().is_err());
    }
}
