use anyhow::Result;
use picturium_libvips::{VipsFilters, VipsImage};

pub fn apply(image: VipsImage, scale: f64, offset: f64) -> Result<VipsImage> {
    let bands = image.get_bands();

    let mut scale = vec![scale; bands as usize];
    let mut offset = vec![offset; bands as usize];

    if image.is_transparent() {
        scale[bands as usize - 1] = 1.0;
        offset[bands as usize - 1] = 0.0;
    }

    image
        .linear(&scale, &offset)
        .map_err(|e| anyhow::anyhow!("Failed to apply linear filter: {:?}", e))
}
