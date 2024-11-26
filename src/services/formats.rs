use std::env;
use std::fmt::Display;
use std::path::Path;
use actix_web::http::header::HeaderValue;
use libvips::VipsImage;
use log::{error, warn};
use crate::parameters::format::Format;
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};   

const WEBP_MAX_WIDTH: i32 = 16383; // px
const WEBP_MAX_HEIGHT: i32 = 16383; // px
const WEBP_MAX_RESOLUTION: f64 = 170.0; // MPix

const AVIF_MAX_WIDTH: i32 = 16384; // px
const AVIF_MAX_HEIGHT: i32 = 16384; // px

const PNG_MAX_WIDTH: i32 = 16384; // px
const PNG_MAX_HEIGHT: i32 = 16384; // px

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Avif,
    Webp,
    Jpg,
    Png,
    Pdf
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Avif => write!(f, "avif"),
            OutputFormat::Webp => write!(f, "webp"),
            OutputFormat::Jpg => write!(f, "jpg"),
            OutputFormat::Png => write!(f, "png"),
            OutputFormat::Pdf => write!(f, "pdf")
        }
    }
}

pub fn get_extension(path: &Path) -> Result<String, ()> {
    match path.extension() {
        Some(e) => match e.to_str() {
            Some(e) => Ok(e.to_lowercase()),
            None => Err(())
        }
        None => Err(())
    }
}

pub fn check_supported_input_formats(path: &Path) -> Result<(), ()> {

    let extension = get_extension(path)?;

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tif" | "tiff" | "ico" | "svg" | // Raster formats
        "heic" | "heif" | "jp2" | "jpm" | "jpx" | "jpf" | "avif" | "avifs" | // Modern raster formats
        // "arw" | "raw" | // RAW formats
        "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp" | "rtf" | // Document formats
        "pdf" // Document formats
        => Ok(()),
        _ => Err(())
    }

}

pub fn determine_output_format(url_parameters: &UrlParameters, accept: Option<&HeaderValue>) -> OutputFormat {
    
    if url_parameters.format != Format::Auto {
        return match url_parameters.format.as_str() {
            "jpg" => OutputFormat::Jpg,
            "png" => OutputFormat::Png,
            "webp" => OutputFormat::Webp,
            "avif" => OutputFormat::Avif,
            "pdf" => OutputFormat::Pdf,
            _ => OutputFormat::Webp
        }
    }

    let accept = match accept {
        Some(accept) => accept,
        None => return OutputFormat::Webp
    };

    let accept = match accept.to_str() {
        Ok(accept) => accept,
        Err(_) => return OutputFormat::Webp
    };

    if env::var("AVIF_ENABLE").unwrap_or("false".to_string()) == "true" && accept.contains("image/avif") {
        return OutputFormat::Avif;
    }

    if accept.contains("image/webp") {
        return OutputFormat::Webp;
    }

    OutputFormat::Jpg

}

pub fn is_thumbnail_format(path: &Path) -> bool {
    let extension = get_extension(path).unwrap_or_else(|_| String::new());
    matches!(extension.as_str(), "pdf" | "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp" | "rtf")
}

pub fn is_svg(path: &Path) -> bool {
    let extension = get_extension(path).unwrap_or_else(|_| String::new());
    extension == "svg"
}

pub fn is_generated(path: &Path) -> bool {
    let extension = get_extension(path).unwrap_or_else(|_| String::new());
    matches!(extension.as_str(), "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp" | "rtf")
}

pub fn supports_transparency(path: &Path) -> bool {
    let extension = get_extension(path).unwrap_or_else(|_| String::new());
    !matches!(extension.as_str(), "jpg" | "jpeg")
}

pub fn validate_output_format(image: &VipsImage, url_parameters: &UrlParameters<'_>, output_format: &OutputFormat) -> PipelineResult<OutputFormat> {
    match output_format {
        OutputFormat::Webp => {
            let (width, height) = (image.get_width(), image.get_height());
            let downsize = width > WEBP_MAX_WIDTH || height > WEBP_MAX_HEIGHT || (width * height) as f64 > (WEBP_MAX_RESOLUTION * 1_000_000.0);

            if !downsize {
                return Ok(output_format.clone());
            }

            if url_parameters.format != Format::Auto {
                error!("WEBP output image is too large (max. {WEBP_MAX_WIDTH}x{WEBP_MAX_HEIGHT} or {WEBP_MAX_RESOLUTION} MPix)");
                return Err(PipelineError("Failed to save image: too large".to_string()));
            }

            warn!("Very large image, falling back to JPEG/PNG format");

            Ok(match image.image_hasalpha() && width <= PNG_MAX_WIDTH && height <= PNG_MAX_HEIGHT {
                true => OutputFormat::Png,
                false => OutputFormat::Jpg,
            })
        },
        OutputFormat::Avif => {
            let (width, height) = (image.get_width(), image.get_height());
            let downsize = width > AVIF_MAX_WIDTH || height > AVIF_MAX_HEIGHT;

            if !downsize {
                return Ok(output_format.clone());
            }

            if url_parameters.format != Format::Auto {
                error!("AVIF output image is too large (max. {AVIF_MAX_WIDTH}x{AVIF_MAX_HEIGHT})");
                return Err(PipelineError("Failed to save image: too large".to_string()));
            }

            warn!("Very large image, falling back to JPEG format");
            Ok(OutputFormat::Jpg)
        },
        OutputFormat::Png => {
            let (width, height) = (image.get_width(), image.get_height());
            let downsize = width > PNG_MAX_WIDTH || height > PNG_MAX_HEIGHT;

            if !downsize {
                return Ok(output_format.clone());
            }

            if url_parameters.format != Format::Auto {
                error!("PNG output image is too large (max. {PNG_MAX_WIDTH}x{PNG_MAX_HEIGHT})");
                return Err(PipelineError("Failed to save image: too large".to_string()));
            }

            warn!("Very large image, falling back to JPEG format");
            Ok(OutputFormat::Jpg)
        },
        _ => Ok(output_format.clone())
    }
}