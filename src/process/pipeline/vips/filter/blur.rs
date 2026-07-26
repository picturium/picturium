use anyhow::Result;
use picturium_libvips::{VipsFilters, VipsImage, VipsOperations};

const PREMULTIPLICATION_THRESHOLD: u16 = 50;

pub fn apply(image: VipsImage, sigma: u16) -> Result<VipsImage> {
    if sigma == 0 {
        return Ok(image);
    }

    let mut image = image;

    if PREMULTIPLICATION_THRESHOLD >= sigma {
        image = image
            .premultiply()
            .map_err(|e| anyhow::anyhow!("Failed to premultiply image before blur: {:?}", e))?;
    }

    image = image
        .blur(sigma as f64, None)
        .map_err(|e| anyhow::anyhow!("Failed to apply blur filter: {:?}", e))?;

    if PREMULTIPLICATION_THRESHOLD >= sigma {
        image = image
            .unpremultiply()
            .map_err(|e| anyhow::anyhow!("Failed to unpremultiply image after blur: {:?}", e))?;
    }

    Ok(image)
}
