use anyhow::{Result, anyhow};
use picturium_libvips::{VipsAnimations, VipsCrop, VipsImage, arrayjoin};

pub(super) fn page_height(image: &VipsImage) -> i32 {
    image.get_page_height().clamp(1, image.get_height().max(1))
}

pub(super) fn page_count(image: &VipsImage) -> i32 {
    (image.get_height() / page_height(image)).max(1)
}

pub(super) fn per_page(
    image: VipsImage,
    operation: impl Fn(VipsImage) -> Result<VipsImage>,
) -> Result<VipsImage> {
    let pages = page_count(&image);

    if pages <= 1 {
        return operation(image);
    }

    let (width, height) = (image.get_width(), page_height(&image));
    let mut frames = Vec::with_capacity(pages as usize);

    for index in 0..pages {
        let frame = image
            .clone()
            .extract_area(0, index * height, width, height)
            .map_err(|error| anyhow!("failed to split animation frame {index}: {error}"))?;

        frames.push(operation(frame)?);
    }

    let frame_height = frames[0].get_height();

    arrayjoin(frames, 1)
        .map_err(|error| anyhow!("failed to restack animation frames: {error}"))?
        .set_page_height(frame_height)
        .map_err(|error| anyhow!("failed to tag animation frame height: {error}"))
}

pub(super) fn select(image: VipsImage, indices: &[i32]) -> Result<VipsImage> {
    let (width, height) = (image.get_width(), page_height(&image));
    let mut frames = Vec::with_capacity(indices.len());

    for index in indices {
        frames.push(
            image
                .clone()
                .extract_area(0, index * height, width, height)
                .map_err(|error| anyhow!("failed to select animation frame {index}: {error}"))?,
        );
    }

    arrayjoin(frames, 1)
        .map_err(|error| anyhow!("failed to restack selected animation frames: {error}"))?
        .set_page_height(height)
        .map_err(|error| anyhow!("failed to tag animation frame height: {error}"))
}

pub(super) fn flatten(image: VipsImage) -> Result<VipsImage> {
    if page_count(&image) <= 1 {
        return Ok(image);
    }

    let (width, height) = (image.get_width(), page_height(&image));

    image
        .extract_area(0, 0, width, height)
        .map_err(|error| anyhow!("failed to take the first animation frame: {error}"))?
        .set_page_height(height)
        .map_err(|error| anyhow!("failed to tag the flattened frame height: {error}"))
}
