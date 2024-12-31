use log::debug;
use picturium_libvips::{Vips, VipsImage, VipsOperations};
use crate::pipeline::{PipelineError, PipelineResult};

pub(crate) async fn transform(image: VipsImage) -> PipelineResult<VipsImage> {
    match image.icc_transform("sRGB", None) {
        Ok(image) => Ok(image),
        Err(_) => Err(PipelineError(format!("Failed to transform image to sRGB: {}", Vips::get_error())))
    }
}

pub fn uses_srgb_color_profile(image: &VipsImage) -> bool {
    // Skip ICC transform for images without ICC profile data
    if !image.has_property("icc-profile-data").unwrap_or(false) {
        debug!("Skip ICC has_property");
        return true;
    }

    let data = match image.get_blob("icc-profile-data") {
        Ok(data) => data,
        Err(_) => {
            debug!("Skip ICC get_blob");
            return true // Skip ICC transform for images without ICC profile data
        }
    };

    let icc = match icc_profile::DecodedICCProfile::new(&data) {
        Ok(icc) => icc,
        Err(_) => {
            debug!("Skip ICC decode");
            return false // Perform ICC transform for images with invalid ICC profile data
        }
    };

    // Skip ICC transform for images already using sRGB(-ish) color profiles
    let description = match icc.tags.get("desc") {
        Some(description) => description,
        None => {
            debug!("Skip ICC description");
            return false // Perform ICC transform for images without ICC profile description
        }
    };

    debug!("Color profile description: {:?}", description);
    description.as_string(0).to_lowercase().contains("srgb")
}
