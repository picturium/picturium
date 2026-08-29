pub mod animate;
pub mod aspect_ratio;
pub mod byte_size;
pub mod padding;
pub mod rotate;
pub mod background;
pub mod colors;
pub mod crop;
pub mod dpr;
pub mod filter;
pub mod metadata;
pub mod pages;
pub mod scale;
pub mod style;
pub mod limits;
pub mod thumbnail;
pub mod time;
pub mod watermark;
pub mod color;
pub mod parsed;

use crate::enums::autorot::Autorot;
use crate::enums::download::Download;
use crate::enums::dpi::Dpi;
use crate::enums::image_extend::ImageExtend;
use crate::enums::image_fit::ImageFit;
use crate::enums::image_gravity::ImageGravity;
use crate::enums::image_resample::ImageResample;
use crate::enums::original::Original;
use crate::enums::output_format::OutputFormat;
use crate::enums::output_quality::OutputQuality;
use crate::enums::upsize::Upsize;
use crate::params::animate::Animate;
use crate::params::background::Background;
use crate::params::crop::Crop;
use crate::params::dpr::deserialize_dpr;
use crate::params::filter::Filter;
use crate::params::limits::Limits;
use crate::params::metadata::{deserialize_metadata, Metadata};
use crate::params::padding::Padding;
use crate::params::pages::Pages;
use crate::params::rotate::Rotate;
use crate::params::scale::deserialize_scale;
use crate::params::style::deserialize_style;
use crate::params::thumbnail::Thumbnail;
use crate::params::time::Time;
use aspect_ratio::AspectRatio;
use serde::{Deserialize, Deserializer};
use std::fmt::Display;
use std::str::FromStr;
use crate::enums::force::Force;
use crate::params::watermark::Watermark;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RequestParams {
    pub force: Option<Force>,
    #[serde(alias = "w", default, deserialize_with = "deserialize_dimension")]
    pub width: Option<u16>,
    #[serde(alias = "h", default, deserialize_with = "deserialize_dimension")]
    pub height: Option<u16>,
    #[serde(alias = "ar")]
    pub aspect_ratio: Option<AspectRatio>,
    #[serde(alias = "g", default, deserialize_with = "deserialize_enum")]
    pub gravity: Option<ImageGravity>,
    #[serde(default, deserialize_with = "deserialize_dpr")]
    pub dpr: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_scale")]
    pub scale: Option<f32>,
    pub upsize: Option<Upsize>,
    #[serde(default, deserialize_with = "deserialize_enum")]
    pub extend: Option<ImageExtend>,
    #[serde(default, deserialize_with = "deserialize_enum")]
    pub resample: Option<ImageResample>,
    #[serde(default, deserialize_with = "deserialize_enum")]
    pub fit: Option<ImageFit>,
    #[serde(alias = "pad")]
    pub padding: Option<Padding>,
    #[serde(alias = "autorot")]
    pub auto_rotate: Option<Autorot>,
    #[serde(alias = "rot")]
    pub rotate: Option<Rotate>,
    #[serde(alias = "bg")]
    pub background: Option<Background>,
    pub crop: Option<Crop>,
    pub filter: Option<Filter>,
    pub cache: Option<String>,
    pub download: Option<Download>,
    pub original: Option<Original>,
    #[serde(alias = "q")]
    pub quality: Option<OutputQuality>,
    #[serde(alias = "f", default, deserialize_with = "deserialize_enum")]
    pub format: Option<OutputFormat>,
    pub dpi: Option<Dpi>,
    #[serde(default, deserialize_with = "deserialize_style")]
    pub style: Option<String>,
    #[serde(default, deserialize_with = "deserialize_metadata", alias = "meta")]
    pub metadata: Option<Metadata>,
    pub fallback: Option<String>,
    #[serde(alias = "limit")]
    pub limits: Option<Limits>,
    #[serde(alias = "page")]
    pub pages: Option<Pages>,
    #[serde(alias = "thumb")]
    pub thumbnail: Option<Thumbnail>,
    #[serde(alias = "anim")]
    pub animate: Option<Animate>,
    #[serde(alias = "t")]
    pub time: Option<Time>,
    pub watermark: Option<Watermark>,
}

pub(crate) fn parse_dimension(value: &str) -> Result<Option<u16>, std::num::ParseIntError> {
    let value = value.trim();

    if value.is_empty() {
        return Ok(None);
    }

    Ok(Some(value.parse::<u16>()?).filter(|value| *value > 0))
}

fn deserialize_dimension<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u16>, D::Error> {
    parse_dimension(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
}

fn deserialize_enum<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    String::deserialize(deserializer)?
        .parse()
        .map(Some)
        .map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::value::{Error, StrDeserializer};

    fn parse<T: FromStr>(value: &str) -> Option<T>
    where
        T::Err: Display,
    {
        deserialize_enum(StrDeserializer::<Error>::new(value)).ok().flatten()
    }

    #[test]
    fn url_enum_values_are_case_insensitive() {
        assert_eq!(parse::<ImageGravity>("center"), Some(ImageGravity::Center));
        assert_eq!(parse::<ImageGravity>("Center"), Some(ImageGravity::Center));
        assert_eq!(parse::<ImageGravity>("TOP-LEFT"), Some(ImageGravity::TopLeft));
        assert_eq!(parse::<ImageFit>("Contain"), Some(ImageFit::Contain));
        assert_eq!(parse::<OutputFormat>("JPG"), Some(OutputFormat::Jpeg));
        assert_eq!(parse::<OutputFormat>("jpeg"), Some(OutputFormat::Jpeg));
    }

    #[test]
    fn a_zero_or_empty_dimension_means_the_parameter_was_not_requested() {
        assert_eq!(parse_dimension("800").unwrap(), Some(800));
        assert_eq!(parse_dimension("0").unwrap(), None);
        assert_eq!(parse_dimension("").unwrap(), None);
        assert_eq!(parse_dimension(" ").unwrap(), None);
        assert!(parse_dimension("-5").is_err());
        assert!(parse_dimension("70000").is_err());
    }

    #[test]
    fn an_unknown_url_enum_value_is_rejected() {
        assert_eq!(parse::<ImageGravity>("middle"), None);
        assert_eq!(parse::<ImageGravity>(""), None);
    }
}
