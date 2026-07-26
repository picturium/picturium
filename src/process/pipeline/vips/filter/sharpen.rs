use anyhow::Result;
use picturium_libvips::{SharpenOptions, VipsFilters, VipsImage};

pub fn apply(image: VipsImage, sigma: f64) -> Result<VipsImage> {
    if sigma == 0.0 {
        return Ok(image);
    }

    image
        .sharpen(Some(SharpenOptions {
            sigma,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to apply sharpen filter: {:?}", e))
}
