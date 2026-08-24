use picturium_libvips::VipsSubsample;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
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
