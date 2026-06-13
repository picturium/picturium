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