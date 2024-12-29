use std::path::PathBuf;

use log::{debug, error};
use picturium_libvips::{HeifSaveOptions, JpegSaveOptions, PngSaveOptions, VipsHeifCompression, VipsHeifEncoder, VipsImage, VipsKeep, VipsSaving, VipsSubsample, VipsWebpPreset, WebpSaveOptions};
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

    if let Err(e) = image.save_heif(&cache_path, HeifSaveOptions {
        q: match url_parameters.quality {
            Quality::Custom(quality) => quality as i32,
            Quality::Default => avif_default_quality(&image),
        },
        bitdepth: 8,
        compression: VipsHeifCompression::HEVC,
        effort: 1,
        subsample_mode: VipsSubsample::Off,
        encoder: VipsHeifEncoder::AOM,
        keep: VipsKeep::None,
        ..HeifSaveOptions::default()
    }.into()) {
        error!("Failed to save AVIF image {}: {}", url_parameters.path.to_string_lossy(), e);
        return Err(PipelineError("Failed to save image".to_string()));
    }

    Ok(cache_path.into())

}

fn finalize_webp(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<PathBuf> {

    let cache_path = cache::get_path_from_url_parameters(url_parameters, &OutputFormat::Webp);

    if let Err(e) = image.save_webp(&cache_path, WebpSaveOptions {
        q: match url_parameters.quality {
            Quality::Custom(quality) => quality as i32,
            Quality::Default => webp_default_quality(&image),
        },
        preset: VipsWebpPreset::Default,
        smart_subsample: true,
        keep: VipsKeep::None,
        alpha_q: 50,
        ..WebpSaveOptions::default()
    }.into()) {
        error!("Failed to save WEBP image {}: {}", url_parameters.path.to_string_lossy(), e);
        return Err(PipelineError("Failed to save image".to_string()));
    }

    Ok(cache_path.into())

}

fn finalize_jpg(image: VipsImage, url_parameters: &UrlParameters<'_>) -> PipelineResult<PathBuf> {

    let cache_path = cache::get_path_from_url_parameters(url_parameters, &OutputFormat::Jpg);

    if let Err(e) = image.save_jpeg(&cache_path, JpegSaveOptions {
        q: match url_parameters.quality {
            Quality::Custom(quality) => quality as i32,
            Quality::Default => jpg_default_quality(&image),
        },
        optimize_coding: true,
        keep: VipsKeep::None,
        ..JpegSaveOptions::default()
    }.into()) {
        error!("Failed to save JPEG image {}: {}", url_parameters.path.to_string_lossy(), e);
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

    if let Err(e) = image.save_png(&cache_path, PngSaveOptions {
        keep: VipsKeep::None,
        palette: true,
        q: quality,
        dither: if quality < 90 { 0.8 } else { 1.0 },
        ..PngSaveOptions::default()
    }.into()) {
        error!("Failed to save PNG image {}: {}", url_parameters.path.to_string_lossy(), e);
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
