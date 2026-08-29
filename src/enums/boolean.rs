use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Boolean {
    True,
    #[default]
    False,
}

impl<'de> Deserialize<'de> for Boolean {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Boolean;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "enable = true or 1 or empty string; disable = false or 0")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(match v {
                    true => Boolean::True,
                    false => Boolean::False,
                })
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v.to_ascii_lowercase().as_str() {
                    "true" | "" | "1" => Ok(Boolean::True),
                    "false" | "0" => Ok(Boolean::False),
                    _ => Err(de::Error::custom("invalid value for boolean parameter")),
                }
            }
        }

        d.deserialize_str(V)
    }
}

impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        match value {
            true => Boolean::True,
            false => Boolean::False,
        }
    }
}

impl PartialEq<bool> for Boolean {
    fn eq(&self, other: &bool) -> bool {
        matches!((self, other), (Boolean::True, true) | (Boolean::False, false))
    }
}