use crate::config::{parse_env, ConfigFromEnv};
use crate::enums::image_fit::ImageFit;
use crate::enums::image_gravity::ImageGravity;
use crate::enums::image_resample::ImageResample;
use anyhow::Result;
use crate::enums::image_extend::ImageExtend;

const DEFAULT_IMAGE_EXIF_AUTO_ROTATE: &str = "true";
const DEFAULT_IMAGE_UPSIZE: &str = "false";
const DEFAULT_IMAGE_EXTEND: &str = "bg";
const DEFAULT_IMAGE_FIT: &str = "cover";
const DEFAULT_IMAGE_GRAVITY: &str = "center";
const DEFAULT_IMAGE_CROP_GRAVITY: &str = "center";
const DEFAULT_IMAGE_RESAMPLE: &str = "lanczos3";

#[derive(Debug, Clone)]
pub struct ImageConfig {
    pub auto_rotate: bool,
    pub upsize: bool,
    pub extend: ImageExtend,
    pub fit: ImageFit,
    pub gravity: ImageGravity,
    pub crop_gravity: ImageGravity,
    pub resample: ImageResample,
}

impl ConfigFromEnv for ImageConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            auto_rotate: parse_env("IMAGE_EXIF_AUTO_ROTATE", DEFAULT_IMAGE_EXIF_AUTO_ROTATE)?,
            upsize: parse_env("IMAGE_UPSIZE", DEFAULT_IMAGE_UPSIZE)?,
            extend: parse_env("IMAGE_EXTEND", DEFAULT_IMAGE_EXTEND)?,
            fit: parse_env("IMAGE_FIT", DEFAULT_IMAGE_FIT)?,
            gravity: parse_env("IMAGE_GRAVITY", DEFAULT_IMAGE_GRAVITY)?,
            crop_gravity: parse_env("IMAGE_CROP_GRAVITY", DEFAULT_IMAGE_CROP_GRAVITY)?,
            resample: parse_env("IMAGE_RESAMPLE", DEFAULT_IMAGE_RESAMPLE)?,
        })
    }
}