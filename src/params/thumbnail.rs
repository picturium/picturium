use std::fmt;
use std::str::FromStr;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;
use crate::params::pages::parse_pages;

#[derive(Debug, Clone, Default)]
pub struct Thumbnail {
    pub pages: Option<Vec<u32>>,
}

#[derive(Debug)]
pub struct ThumbnailParseError(String);

impl fmt::Display for ThumbnailParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ThumbnailParseError {}

impl FromStr for Thumbnail {
    type Err = ThumbnailParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('|')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut thumb = Thumbnail::default();

        for part in parts {
            let (key, value) = part.split_once(':').ok_or_else(|| ThumbnailParseError(format!("Missing ':' in thumb segment '{part}'")))?;

            match key {
                // Deprecated, in favor of top-level `page`|`pages` parameter
                "p" | "page" | "pages" => thumb.pages = Some(
                    parse_pages(value).map_err(|e| ThumbnailParseError(e.to_string()))?
                ),
                _ => return Err(ThumbnailParseError(format!("Unknown thumb key: '{key}'"))),
            }
        }

        Ok(thumb)
    }
}

impl<'de> Deserialize<'de> for Thumbnail {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Thumbnail;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "thumb parameters in format thumb=page:1,2")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}
