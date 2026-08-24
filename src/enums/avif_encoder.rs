use picturium_libvips::VipsHeifEncoder;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum AvifEncoder {
    #[default]
    Aom,
    Auto,
    Rav1e,
    Svt,
    X265,
}

impl Into<VipsHeifEncoder> for AvifEncoder {
    fn into(self) -> VipsHeifEncoder {
        match self {
            AvifEncoder::Aom => VipsHeifEncoder::AOM,
            AvifEncoder::Auto => VipsHeifEncoder::Auto,
            AvifEncoder::Rav1e => VipsHeifEncoder::RAV1E,
            AvifEncoder::Svt => VipsHeifEncoder::SVT,
            AvifEncoder::X265 => VipsHeifEncoder::X265,
        }
    }
}
