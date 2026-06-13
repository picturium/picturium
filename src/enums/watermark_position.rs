use serde::Deserialize;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum WatermarkPosition {
    Center,
    Repeat,
    Top,
    Right,
    Bottom,
    Left,
    #[strum(serialize = "top-left")]
    #[serde(rename = "top-left")]
    TopLeft,
    #[strum(serialize = "top-right")]
    #[serde(rename = "top-right")]
    TopRight,
    #[strum(serialize = "bottom-left")]
    #[serde(rename = "bottom-left")]
    BottomLeft,
    #[default]
    #[strum(serialize = "bottom-right")]
    #[serde(rename = "bottom-right")]
    BottomRight,
}
