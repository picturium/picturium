use crate::enums::image_extend::ImageExtend;
use crate::enums::image_fit::ImageFit;
use crate::enums::image_gravity::ImageGravity;
use crate::enums::image_resample::ImageResample;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImageConfig {
    pub auto_rotate: bool,
    pub upsize: bool,
    pub extend: ImageExtend,
    pub fit: ImageFit,
    pub gravity: ImageGravity,
    pub crop_gravity: ImageGravity,
    pub resample: ImageResample,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            auto_rotate: true,
            upsize: false,
            extend: ImageExtend::Bg,
            fit: ImageFit::Cover,
            gravity: ImageGravity::Center,
            crop_gravity: ImageGravity::Center,
            resample: ImageResample::Lanczos3,
        }
    }
}
