pub mod aspect_ratio;
pub mod padding;
pub mod rotate;
pub mod background;
pub mod colors;
pub mod crop;
pub mod dpr;
pub mod filter;
pub mod metadata;
pub mod scale;
pub mod style;
pub mod limits;
pub mod thumbnail;
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
use crate::params::background::Background;
use crate::params::crop::Crop;
use crate::params::dpr::deserialize_dpr;
use crate::params::filter::Filter;
use crate::params::limits::Limits;
use crate::params::metadata::{deserialize_metadata, Metadata};
use crate::params::padding::Padding;
use crate::params::rotate::Rotate;
use crate::params::scale::deserialize_scale;
use crate::params::style::deserialize_style;
use crate::params::thumbnail::Thumbnail;
use aspect_ratio::AspectRatio;
use serde::Deserialize;
use crate::enums::force::Force;
use crate::params::watermark::Watermark;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RequestParams {
    pub force: Option<Force>,
    #[serde(alias = "w")]
    pub width: Option<u16>,
    #[serde(alias = "h")]
    pub height: Option<u16>,
    #[serde(alias = "ar")]
    pub aspect_ratio: Option<AspectRatio>,
    #[serde(alias = "g")]
    pub gravity: Option<ImageGravity>,
    #[serde(default, deserialize_with = "deserialize_dpr")]
    pub dpr: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_scale")]
    pub scale: Option<f32>,
    pub upsize: Option<Upsize>,
    pub extend: Option<ImageExtend>,
    pub resample: Option<ImageResample>,
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
    pub download: Option<Download>,
    pub original: Option<Original>,
    #[serde(alias = "q")]
    pub quality: Option<OutputQuality>,
    #[serde(alias = "f")]
    pub format: Option<OutputFormat>,
    pub dpi: Option<Dpi>,
    #[serde(default, deserialize_with = "deserialize_style")]
    pub style: Option<String>,
    #[serde(default, deserialize_with = "deserialize_metadata", alias = "meta")]
    pub metadata: Option<Metadata>,
    pub fallback: Option<String>,
    pub limits: Option<Limits>,
    #[serde(alias = "thumb")]
    pub thumbnail: Option<Thumbnail>,
    pub watermark: Option<Watermark>,
}


