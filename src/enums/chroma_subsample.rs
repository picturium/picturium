use picturium_libvips::VipsSubsample;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum ChromaSubsample {
    #[default]
    Auto,
    On,
    Off,
}

impl Into<VipsSubsample> for ChromaSubsample {
    fn into(self) -> VipsSubsample {
        match self {
            ChromaSubsample::Auto => VipsSubsample::Auto,
            ChromaSubsample::On => VipsSubsample::On,
            ChromaSubsample::Off => VipsSubsample::Off,
        }
    }
}
