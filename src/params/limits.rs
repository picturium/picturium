use crate::params::byte_size::ByteSize;
use std::fmt;
use std::str::FromStr;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dimension {
    pub width: Option<u16>,
    pub height: Option<u16>,
}

impl FromStr for Dimension {
    type Err = LimitsParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse = |v: &str| -> Result<Option<u16>, Self::Err> {
            match v.trim() {
                "" => Ok(None),
                v => v
                    .parse()
                    .map(Some)
                    .map_err(|_| LimitsParseError(format!("Invalid dimension value: '{v}'"))),
            }
        };

        match s.split_once('x') {
            Some((width, height)) => Ok(Dimension {
                width: parse(width)?,
                height: parse(height)?,
            }),
            None => {
                let value = parse(s)?;
                Ok(Dimension {
                    width: value,
                    height: value,
                })
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Limits {
    pub dimension: Option<Dimension>,
    pub size: Option<usize>,
}

#[derive(Debug)]
pub struct LimitsParseError(String);

impl fmt::Display for LimitsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LimitsParseError {}

impl FromStr for Limits {
    type Err = LimitsParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('|')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut limits = Limits::default();

        for part in parts {
            let (key, value) = part.split_once(':').ok_or_else(|| LimitsParseError(format!("Missing ':' in limits segment '{part}'")))?;

            // TODO > Better value parsing with validation
            match key {
                "dimension" => limits.dimension = Some(value.parse()?),
                "size" => limits.size = Some(value.parse::<ByteSize>().map_err(|e| LimitsParseError(e.to_string()))?.0),
                _ => return Err(LimitsParseError(format!("Unknown limits key: '{key}'"))),
            }
        }

        Ok(limits)
    }
}

impl<'de> Deserialize<'de> for Limits {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Limits;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "limits parameters in format limit=dimension:1000|size:500K")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}
