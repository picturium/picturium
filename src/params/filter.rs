use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;
use crate::enums::filter::FilterValue;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Filter(pub(crate) Vec<FilterValue>);

#[derive(Debug)]
pub struct FilterParseError(pub String);

impl fmt::Display for FilterParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for FilterParseError {}

impl FromStr for Filter {
    type Err = FilterParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('|')
            .filter(|s| !s.trim().is_empty())
            .collect();

        Ok(Filter(
            parts.into_iter()
                .map(|s| FilterValue::from_str(s)
                    .map_err(|e| return FilterParseError(format!("Invalid filter segment: '{s}': {e}")))
                ).collect::<Result<Vec<_>, _>>()?
        ))
    }
}

impl<'de> Deserialize<'de> for Filter {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Filter;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "filter parameters in format filter=brightness:0.5|contrast:0.5")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}
