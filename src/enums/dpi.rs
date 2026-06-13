use std::fmt;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dpi {
    #[default]
    Auto,
    Value(u16)
}

impl<'de> Deserialize<'de> for Dpi {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Dpi;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "auto or a positive integer between 1 and 3000")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "auto" => Ok(Dpi::Auto),
                    _ => match v.parse::<u16>() {
                        Ok(value) if value > 0 && value <= 3000 => Ok(Dpi::Value(value)),
                        _ => Err(de::Error::custom("invalid value for dpi parameter")),
                    },
                }
            }
        }

        d.deserialize_str(V)
    }
}
