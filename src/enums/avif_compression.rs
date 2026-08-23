use picturium_libvips::VipsHeifCompression;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum AvifCompression {
    #[default]
    Av1,
    Hevc,
    Avc,
    Jpeg,
}

impl Into<VipsHeifCompression> for AvifCompression {
    fn into(self) -> VipsHeifCompression {
        match self {
            AvifCompression::Av1 => VipsHeifCompression::AV1,
            AvifCompression::Hevc => VipsHeifCompression::HEVC,
            AvifCompression::Avc => VipsHeifCompression::AVC,
            AvifCompression::Jpeg => VipsHeifCompression::JPEG,
        }
    }
}
