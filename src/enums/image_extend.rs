use serde::Deserialize;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum ImageExtend {
    #[default]
    Bg,
    Copy,
    Repeat,
    Mirror,
}

impl From<ImageExtend> for picturium_libvips::VipsExtend {
    fn from(value: ImageExtend) -> Self {
        match value {
            ImageExtend::Bg => Self::Background,
            ImageExtend::Copy => Self::Copy,
            ImageExtend::Repeat => Self::Repeat,
            ImageExtend::Mirror => Self::Mirror,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_url_extend_modes_to_libvips() {
        assert!(matches!(
            picturium_libvips::VipsExtend::from(ImageExtend::Bg),
            picturium_libvips::VipsExtend::Background
        ));
        assert!(matches!(
            picturium_libvips::VipsExtend::from(ImageExtend::Copy),
            picturium_libvips::VipsExtend::Copy
        ));
        assert!(matches!(
            picturium_libvips::VipsExtend::from(ImageExtend::Repeat),
            picturium_libvips::VipsExtend::Repeat
        ));
        assert!(matches!(
            picturium_libvips::VipsExtend::from(ImageExtend::Mirror),
            picturium_libvips::VipsExtend::Mirror
        ));
    }
}
