use std::path::PathBuf;

use libvips::{ops, VipsImage};
use libvips::ops::{ForeignHeifCompression, ForeignHeifEncoder, ForeignKeep, ForeignSubsample, ForeignWebpPreset, HeifsaveOptions, JpegsaveOptions, PngsaveOptions, WebpsaveOptions};
use log::{debug, error};

use crate::cache;
use crate::parameters::{Quality, UrlParameters};
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::formats::OutputFormat;
use crate::services::vips::get_error_message;

pub(crate) async fn run(image: VipsImage, url_parameters: &UrlParameters<'_>, output_format: &OutputFormat) -> PipelineResult<PathBuf> {
    match output_format {
        OutputFormat::Avif => finalize_avif(image, url_parameters),
        OutputFormat::Webp => finalize_webp(image, url_parameters),
        OutputFormat::Png => finalize_png(image, url_parameters),
        _ => finalize_jpg(image, url_parameters)
    }
}

fn finalize_avif(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<PathBuf> {

    let cache_path = cache::get_path_from_url_parameters(url_parameters, &OutputFormat::Avif);

    if ops::heifsave_with_opts(&image, &cache_path, &HeifsaveOptions {
        q: match url_parameters.quality {
            Quality::Custom(quality) => quality as i32,
            Quality::Default => avif_default_quality(&image),
        },
        bitdepth: 8,
        compression: ForeignHeifCompression::Hevc,
        effort: 0,
        subsample_mode: ForeignSubsample::Off,
        encoder: ForeignHeifEncoder::Aom,
        keep: ForeignKeep::None,
        background: match &url_parameters.background {
            Some(background) => Vec::from(background)[0..3].to_vec(),
            None => Vec::new()
        },
        ..HeifsaveOptions::default()
    }).is_err() {
        error!("Failed to save AVIF image {}: {}", url_parameters.path.to_string_lossy(), get_error_message());
        return Err(PipelineError("Failed to save image".to_string()));
    }

    Ok(cache_path.into())

}

fn finalize_webp(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<PathBuf> {

    let cache_path = cache::get_path_from_url_parameters(url_parameters, &OutputFormat::Webp);

    if ops::webpsave_with_opts(&image, &cache_path, &WebpsaveOptions {
        q: match url_parameters.quality {
            Quality::Custom(quality) => quality as i32,
            Quality::Default => webp_default_quality(&image),
        },
        preset: ForeignWebpPreset::Last,
        smart_subsample: true,
        keep: ForeignKeep::None,
        background: match &url_parameters.background {
            Some(background) => Vec::from(background)[0..3].to_vec(),
            None => Vec::new()
        },
        alpha_q: 50,
        ..WebpsaveOptions::default()
    }).is_err() {
        error!("Failed to save WEBP image {}: {}", url_parameters.path.to_string_lossy(), get_error_message());
        return Err(PipelineError("Failed to save image".to_string()));
    }

    Ok(cache_path.into())

}

fn finalize_jpg(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<PathBuf> {

    let cache_path = cache::get_path_from_url_parameters(url_parameters, &OutputFormat::Jpg);

    if ops::jpegsave_with_opts(&image, &cache_path, &JpegsaveOptions {
        q: match url_parameters.quality {
            Quality::Custom(quality) => quality as i32,
            Quality::Default => jpg_default_quality(&image),
        },
        optimize_coding: true,
        keep: ForeignKeep::None,
        background: match &url_parameters.background {
            Some(background) => Vec::from(background)[0..3].to_vec(),
            None => Vec::new()
        },
        ..JpegsaveOptions::default()
    }).is_err() {
        error!("Failed to save JPG image {}: {}", url_parameters.path.to_string_lossy(), get_error_message());
        return Err(PipelineError("Failed to save image".to_string()));
    }

    Ok(cache_path.into())

}

fn finalize_png(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<PathBuf> {

    let cache_path = cache::get_path_from_url_parameters(url_parameters, &OutputFormat::Png);
    let quality = match url_parameters.quality {
        Quality::Custom(quality) => quality as i32,
        Quality::Default => 78,
    };

    if ops::pngsave_with_opts(&image, &cache_path, &PngsaveOptions {
        keep: ForeignKeep::None,
        palette: true,
        q: quality,
        dither: if quality < 90 { 0.8 } else { 1.0 },
        background: match &url_parameters.background {
            Some(background) => Vec::from(background)[0..3].to_vec(),
            None => Vec::new()
        },
        ..PngsaveOptions::default()
    }).is_err() {
        error!("Failed to save PNG image {}: {}", url_parameters.path.to_string_lossy(), get_error_message());
        return Err(PipelineError("Failed to save image".to_string()));
    }

    Ok(cache_path.into())

}

fn avif_default_quality(image: &VipsImage) -> i32 {

    let width = image.get_width() as f64;
    let height = image.get_height() as f64;
    let area = width * height / 1000000.0;

    // Dynamic AVIF quality based on image area, min. 40, max. 59
    let quality = (8.0 - area).clamp(0.0, 8.0 - 0.25) * (59.0 - 40.0) / (8.0 - 0.25) + 40.0;
    debug!("Serving image with quality: {}%, {area}MPix", quality as i32);

    quality as i32

}

fn webp_default_quality(image: &VipsImage) -> i32 {

    let width = image.get_width() as f64;
    let height = image.get_height() as f64;
    let area = width * height / 1000000.0;

    // Dynamic WebP quality based on image area, min. 16, max. 78
    let quality = (8.0 - area).clamp(0.0, 8.0 - 0.25) * (78.0 - 16.0) / (8.0 - 0.25) + 16.0;
    debug!("Serving image with quality: {}%, {area}MPix", quality as i32);

    quality as i32

}

fn jpg_default_quality(image: &VipsImage) -> i32 {

    let width = image.get_width() as f64;
    let height = image.get_height() as f64;
    let area = width * height / 1000000.0;

    // Dynamic JPEG quality based on image area, min. 40, max. 75
    let quality = (8.0 - area).clamp(0.0, 8.0 - 0.25) * (75.0 - 40.0) / (8.0 - 0.25) + 40.0;
    debug!("Serving image with quality: {}%, {area}MPix", quality as i32);

    quality as i32

}
