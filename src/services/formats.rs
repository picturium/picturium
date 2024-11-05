use std::env;
use std::fmt::Display;
use std::path::Path;
use actix_web::http::header::HeaderValue;
use crate::parameters::format::Format;
use crate::parameters::UrlParameters;

#[derive(Debug, PartialEq)]
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
        "pdf" | // Document formats
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp" | "ogv" | "m4v" // Video formats
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
    matches!(extension.as_str(), "pdf" | "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp" | "rtf" | "mp4" | "mkv" | "webm")
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