use std::fmt;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum OutputQuality {
    Auto,
    Low,
    #[default]
    Medium,
    High,
    Maximum,
    Value(u8)
}

impl<'de> Deserialize<'de> for OutputQuality {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = OutputQuality;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "auto, low, medium, high, maximum, or a number 0..100")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "auto" => Ok(OutputQuality::Auto),
                    "low" => Ok(OutputQuality::Low),
                    "medium" => Ok(OutputQuality::Medium),
                    "high" => Ok(OutputQuality::High),
                    "maximum" => Ok(OutputQuality::Maximum),
                    other => match other.parse::<u8>() {
                        Ok(value) if value >= 1 && value <= 100 => Ok(OutputQuality::Value(value)),
                        _ => Err(de::Error::custom("invalid value for output quality parameter")),
                    },
                }
            }
        }

        d.deserialize_str(V)
    }
}
