use crate::config::quality::{QualityConfig, QualityCurve};
use crate::enums::output_format::OutputFormat;
use crate::enums::output_quality::OutputQuality;
use picturium_libvips::VipsImage;
use tracing::debug;

pub(crate) fn get_quality_curve<'a>(
    config: &'a QualityConfig,
    format: &OutputFormat,
) -> Option<&'a QualityCurve> {
    match format {
        OutputFormat::Jpeg => Some(&config.jpeg),
        OutputFormat::Webp => Some(&config.webp),
        OutputFormat::Avif => Some(&config.avif),
        OutputFormat::Jxl => Some(&config.jxl),
        OutputFormat::Png => Some(&config.png),
        _ => None,
    }
}

pub(crate) fn resolve_quality(
    config: &QualityConfig,
    image: &VipsImage,
    requested: OutputQuality,
    curve: &QualityCurve,
) -> u8 {
    if let OutputQuality::Value(quality) = requested {
        return quality;
    }

    let area = calculate_area(image);
    let quality = quality_for_area(config, area, requested, curve);
    debug!("Serving image with quality: {quality}%, {area}MPix");

    quality
}

fn quality_for_area(
    config: &QualityConfig,
    area: f64,
    requested: OutputQuality,
    curve: &QualityCurve,
) -> u8 {
    let span = config.min_area - config.max_area;
    let quality = (config.min_area - area).clamp(0.0, span) * (curve.max - curve.min) / span + curve.min;

    let quality = quality + match requested {
        OutputQuality::Low => curve.low,
        OutputQuality::High => curve.high,
        OutputQuality::Maximum => curve.maximum,
        _ => 0.0,
    };

    quality.clamp(1.0, 100.0) as u8
}

pub(crate) fn calculate_area(image: &VipsImage) -> f64 {
    let width = image.get_width() as f64;
    let height = image.get_height() as f64;

    width * height / 1000000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_jpeg_quality_between_the_area_bounds() {
        let config = QualityConfig::default();
        let jpeg = &config.jpeg;

        assert_eq!(quality_for_area(&config, 0.25, OutputQuality::Medium, jpeg), 80);
        assert_eq!(quality_for_area(&config, 8.0, OutputQuality::Medium, jpeg), 32);
        assert_eq!(quality_for_area(&config, 0.25, OutputQuality::Low, jpeg), 68);
        assert_eq!(quality_for_area(&config, 0.25, OutputQuality::Maximum, jpeg), 97);
        assert_eq!(quality_for_area(&config, 8.0, OutputQuality::Low, jpeg), 20);
    }

    #[test]
    fn treats_auto_like_medium() {
        let config = QualityConfig::default();

        assert_eq!(
            quality_for_area(&config, 2.0, OutputQuality::Auto, &config.jpeg),
            quality_for_area(&config, 2.0, OutputQuality::Medium, &config.jpeg)
        );
    }

    #[test]
    fn clamps_png_maximum_to_the_lossless_branch() {
        let config = QualityConfig::default();

        assert_eq!(
            quality_for_area(&config, 0.25, OutputQuality::Maximum, &config.png),
            100
        );
    }

    #[test]
    fn maps_the_jxl_curve_onto_the_shared_quality_domain() {
        let config = QualityConfig::default();

        assert_eq!(
            quality_for_area(&config, 8.0, OutputQuality::Medium, &config.jxl),
            66 // d ~= 5.0
        );
        assert_eq!(
            quality_for_area(&config, 0.25, OutputQuality::Medium, &config.jxl),
            85 // d ~= 2.2
        );
    }

    #[test]
    fn honours_a_configured_interpolation_window() {
        let config = QualityConfig {
            min_area: 4.0,
            max_area: 1.0,
            ..QualityConfig::default()
        };

        assert_eq!(quality_for_area(&config, 1.0, OutputQuality::Medium, &config.jpeg), 80);
        assert_eq!(quality_for_area(&config, 4.0, OutputQuality::Medium, &config.jpeg), 32);
        assert_eq!(quality_for_area(&config, 8.0, OutputQuality::Medium, &config.jpeg), 32);
    }

    #[test]
    fn gif_has_no_quality_curve() {
        let config = QualityConfig::default();

        assert!(get_quality_curve(&config, &OutputFormat::Gif).is_none());
        assert!(get_quality_curve(&config, &OutputFormat::Jpeg).is_some());
    }
}
