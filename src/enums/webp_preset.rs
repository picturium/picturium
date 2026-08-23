use picturium_libvips::VipsWebpPreset;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString)]
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
