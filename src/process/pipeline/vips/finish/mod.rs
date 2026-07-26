mod avif;
mod gif;
mod jpeg;
mod jxl;
mod png;
mod webp;

use crate::enums::output_format::OutputFormat;
use crate::enums::output_metadata::OutputMetadata;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use picturium_libvips::{VipsImage, VipsKeep};

pub fn finish_image(request: &PipelineRequest, image: VipsImage) -> Result<Vec<u8>> {
    let keep = metadata_to_keep(&request.parameters.metadata);

    match request.output_format {
        OutputFormat::Jpeg => jpeg::finish_image(request, image, keep),
        OutputFormat::Webp => webp::finish_image(request, image, keep),
        OutputFormat::Avif => avif::finish_image(request, image, keep),
        OutputFormat::Jxl => jxl::finish_image(request, image, keep),
        OutputFormat::Png => png::finish_image(request, image, keep),
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

fn calculate_area(image: &VipsImage) -> f64 {
    let width = image.get_width() as f64;
    let height = image.get_height() as f64;

    width * height / 1000000.0
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
