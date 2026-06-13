use picturium_libvips::VipsKernel;
use serde::Deserialize;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum ImageResample {
    Nearest,
    Linear,
    Cubic,
    Lanczos2,
    #[default]
    Lanczos3,
}

impl Into<VipsKernel> for ImageResample {
    fn into(self) -> VipsKernel {
        match self {
            ImageResample::Nearest => VipsKernel::Nearest,
            ImageResample::Linear => VipsKernel::Linear,
            ImageResample::Cubic => VipsKernel::Cubic,
            ImageResample::Lanczos2 => VipsKernel::Lanczos2,
            ImageResample::Lanczos3 => VipsKernel::Lanczos3,
        }
    }
}
