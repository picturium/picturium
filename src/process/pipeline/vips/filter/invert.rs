use crate::process::pipeline::vips::filter::linear;
use picturium_libvips::VipsImage;

pub fn apply(image: VipsImage, scale: f64) -> anyhow::Result<VipsImage> {
    if scale == 0.0 {
        return Ok(image);
    }

    let offset = 255.0 * scale;
    let scale = (scale * -2.0) + 1.0;

    linear::apply(image, scale, offset)
}
