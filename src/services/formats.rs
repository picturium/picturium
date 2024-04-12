use std::fmt::Display;
use std::path::Path;
use actix_web::http::header::HeaderValue;

#[derive(Debug, PartialEq)]
pub enum OutputFormat {
    // Avif,
    Webp,
    Jpg
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // OutputFormat::Avif => write!(f, "avif"),
            OutputFormat::Webp => write!(f, "webp"),
            OutputFormat::Jpg => write!(f, "jpg")
        }
    }
}

fn get_extension(path: &Path) -> Result<String, ()> {
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
        // "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "pdf" | "psd" | "ai" | "eps" // Document formats
        "pdf" // Document formats
        => Ok(()),
        _ => Err(())
    }

}

pub fn determine_output_format(accept: Option<&HeaderValue>) -> OutputFormat {

    let accept = match accept {
        Some(accept) => accept,
        None => return OutputFormat::Webp
    };

    let accept = match accept.to_str() {
        Ok(accept) => accept,
        Err(_) => return OutputFormat::Webp
    };

    // if accept.contains("image/avif") {
    //     return OutputFormat::Avif;
    // }

    if accept.contains("image/webp") {
        return OutputFormat::Webp;
    }

    OutputFormat::Jpg

}

pub fn is_thumbnail_format(path: &Path) -> bool {

    let extension = match get_extension(path) {
        Ok(extension) => extension,
        Err(_) => return false
    };

    match extension.as_str() {
        "pdf" => true,
        // "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "pdf" | "psd" | "ai" | "eps" => true,
        _ => false
    }

}

pub fn is_svg(path: &Path) -> bool {

    let extension = match get_extension(path) {
        Ok(extension) => extension,
        Err(_) => return false
    };

    extension == "svg"

}

pub fn supports_transparency(path: &Path) -> bool {

    let extension = match get_extension(path) {
        Ok(extension) => extension,
        Err(_) => return false
    };

    !matches!(extension.as_str(), "jpg" | "jpeg")

}