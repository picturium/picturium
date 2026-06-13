use std::fmt;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Download {
    Auto,
    Filename(String),
    #[default]
    No,
}

impl<'de> Deserialize<'de> for Download {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Download;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a filename or empty parameter")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "" => Ok(Download::Auto),
                    _ => Ok(Download::Filename(v.to_string())),
                }
            }
        }

        d.deserialize_str(V)
    }
}
