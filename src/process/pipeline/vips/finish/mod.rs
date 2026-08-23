mod avif;
mod gif;
mod jpeg;
mod jxl;
mod png;
mod quality;
mod size_limit;
mod webp;

use crate::enums::output_format::OutputFormat;
use crate::enums::output_metadata::OutputMetadata;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use picturium_libvips::{VipsImage, VipsKeep};
use quality::{get_quality_curve, resolve_quality};

pub fn finish_image(request: &PipelineRequest, image: VipsImage) -> Result<Vec<u8>> {
    let config = &request.state.config.output;
    let keep = metadata_to_keep(&request.parameters.metadata);
    let curve = get_quality_curve(&config.quality_curves, &request.output_format);
    let limit = request.parameters.limits.size.filter(|_| curve.is_some());

    let image = match limit {
        Some(_) => image.copy_memory().map_err(|e| anyhow::anyhow!("Failed to materialize image for size limit: {e}"))?,
        None => image,
    };

    let quality = match curve {
        Some(curve) => resolve_quality(
            &config.quality_curves,
            &image,
            request.parameters.quality,
            curve,
        ),
        None => 100,
    };

    let buffer = encode(request, &image, keep, quality)?;

    let Some(limit) = limit else {
        return Ok(buffer);
    };

    if buffer.len() <= limit {
        return Ok(buffer);
    }

    size_limit::shrink_to_limit(
        quality,
        buffer,
        limit,
        config.max_size_threshold as f64 / 100.0,
        config.max_size_attempts,
        config.max_size_min_quality,
        |quality| encode(request, &image, keep, quality),
    )
}

fn encode(
    request: &PipelineRequest,
    image: &VipsImage,
    keep: VipsKeep,
    quality: u8,
) -> Result<Vec<u8>> {
    match request.output_format {
        OutputFormat::Jpeg => jpeg::finish_image(request, image, keep, quality),
        OutputFormat::Webp => webp::finish_image(request, image, keep, quality),
        OutputFormat::Avif => avif::finish_image(request, image, keep, quality),
        OutputFormat::Jxl => jxl::finish_image(request, image, keep, quality),
        OutputFormat::Png => png::finish_image(request, image, keep, quality),
        OutputFormat::Gif => gif::finish_image(request, image, keep),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported output format: {:?}",
                request.output_format
            ));
        }
    }
    .map_err(|e| anyhow::anyhow!("Failed to generate output image: {:?}", e))
}

fn metadata_to_keep(metadata: &[OutputMetadata]) -> VipsKeep {
    metadata.iter().fold(VipsKeep::None, |keep, metadata| {
        keep | match metadata {
            OutputMetadata::None => VipsKeep::None,
            OutputMetadata::Icc => VipsKeep::ICC,
            OutputMetadata::Exif => VipsKeep::Exif,
            OutputMetadata::Xmp => VipsKeep::XMP,
            OutputMetadata::Iptc => VipsKeep::IPTC,
            OutputMetadata::Other => VipsKeep::Other,
            OutputMetadata::Gainmap => VipsKeep::Gainmap,
            OutputMetadata::All => VipsKeep::All,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_metadata_category_to_its_vips_keep_flag() {
        assert_eq!(metadata_to_keep(&[OutputMetadata::None]), VipsKeep::None);
        assert_eq!(metadata_to_keep(&[OutputMetadata::Icc]), VipsKeep::ICC);
        assert_eq!(metadata_to_keep(&[OutputMetadata::Exif]), VipsKeep::Exif);
        assert_eq!(metadata_to_keep(&[OutputMetadata::Xmp]), VipsKeep::XMP);
        assert_eq!(metadata_to_keep(&[OutputMetadata::Iptc]), VipsKeep::IPTC);
        assert_eq!(metadata_to_keep(&[OutputMetadata::Other]), VipsKeep::Other);
        assert_eq!(
            metadata_to_keep(&[OutputMetadata::Gainmap]),
            VipsKeep::Gainmap
        );
        assert_eq!(metadata_to_keep(&[OutputMetadata::All]), VipsKeep::All);
    }

    #[test]
    fn combines_multiple_metadata_categories() {
        assert_eq!(
            metadata_to_keep(&[OutputMetadata::Icc, OutputMetadata::Exif]),
            VipsKeep::ICC | VipsKeep::Exif
        );
    }
}
