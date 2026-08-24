use picturium_libvips::VipsWebpPreset;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum WebpPreset {
    #[default]
    Default,
    Picture,
    Photo,
    Drawing,
    Icon,
    Text,
}

impl Into<VipsWebpPreset> for WebpPreset {
    fn into(self) -> VipsWebpPreset {
        match self {
            WebpPreset::Default => VipsWebpPreset::Default,
            WebpPreset::Picture => VipsWebpPreset::Picture,
            WebpPreset::Photo => VipsWebpPreset::Photo,
            WebpPreset::Drawing => VipsWebpPreset::Drawing,
            WebpPreset::Icon => VipsWebpPreset::Icon,
            WebpPreset::Text => VipsWebpPreset::Text,
        }
    }
}
