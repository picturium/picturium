use std::fmt;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
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

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                match u8::try_from(v) {
                    Ok(value) if (1..=100).contains(&value) => Ok(OutputQuality::Value(value)),
                    _ => Err(de::Error::custom("invalid value for output quality parameter")),
                }
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                match u8::try_from(v) {
                    Ok(value) if (1..=100).contains(&value) => Ok(OutputQuality::Value(value)),
                    _ => Err(de::Error::custom("invalid value for output quality parameter")),
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v.to_ascii_lowercase().as_str() {
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

        d.deserialize_any(V)
    }
}

impl Serialize for OutputQuality {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            OutputQuality::Auto => s.serialize_str("auto"),
            OutputQuality::Low => s.serialize_str("low"),
            OutputQuality::Medium => s.serialize_str("medium"),
            OutputQuality::High => s.serialize_str("high"),
            OutputQuality::Maximum => s.serialize_str("maximum"),
            OutputQuality::Value(value) => s.serialize_str(&value.to_string()),
        }
    }
}
