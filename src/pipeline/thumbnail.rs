use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use libvips::{ops, VipsImage};
use libvips::ops::{Intent, Interesting, ThumbnailOptions};

use crate::cache;
use crate::cache::{get_document_path_from_url_parameters, index};
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::pipeline::resize::get_rasterize_dimensions;
use crate::services::formats::{get_extension, is_thumbnail_format};
use crate::services::vips::get_error_message;
use crate::pipeline::mpv_thumb::generate_video_thumbnail;

pub(crate) async fn run(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    if !is_thumbnail_format(url_parameters.path) {
        return match VipsImage::new_from_file(&working_file.to_string_lossy()) {
            Ok(image) => Ok(image),
            Err(error) => return Err(PipelineError(format!("Failed to open image: {}", error)))
        };
    }

    let extension = match get_extension(url_parameters.path) {
        Ok(extension) => extension,
        Err(_) => return Err(PipelineError("Failed to determine file extension".to_string()))
    };

    match extension.as_str() {
        "pdf" => generate_pdf_thumbnail(working_file, url_parameters),
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp" | "ogv" | "m4v" => generate_video_thumbnail(working_file, url_parameters),
        "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp" | "rtf" => generate_document_thumbnail(working_file, url_parameters),
        _ => Err(PipelineError("Unsupported file format".to_string()))
    }

}

fn generate_pdf_thumbnail(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let pdf = VipsImage::new_from_file(&working_file.to_string_lossy()).unwrap();
    let page_parameter = format!("[page={}]", (url_parameters.thumbnail.page - 1).min(pdf.get_n_pages() as u32 - 1));

    let pdf = VipsImage::new_from_file(&(working_file.to_string_lossy() + &page_parameter[..])).unwrap();
    let (width, height) = get_rasterize_dimensions(&pdf, url_parameters);
    
    match ops::thumbnail_with_opts(&(working_file.to_string_lossy() + &page_parameter[..]), width, &ThumbnailOptions {
        height,
        import_profile: "sRGB".to_string(),
        export_profile: "sRGB".to_string(),
        intent: Intent::Perceptual,
        crop: Interesting::Centre,
        ..Default::default()
    }) {
        Ok(image) => Ok(image),
        _ => Err(PipelineError(format!("Failed to generate thumbnail for PDF file {working_file:?}: {}", get_error_message())))
    }
    
}

fn generate_document_thumbnail(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    
    let cache_path = get_document_path_from_url_parameters(url_parameters);
    let cache_path = Path::new(&cache_path);
    
    let cache_enable = env::var("CACHE_ENABLE").unwrap_or("true".to_string()) == "true";
    
    if !cache_enable || !cache::is_cached(&cache_path.to_string_lossy(), url_parameters) {
        generate_pdf_from_document(working_file, cache_path)?;
    }
    
    generate_pdf_thumbnail(cache_path, url_parameters)

}

fn generate_pdf_from_document(working_file: &Path, cache_path: &Path) -> PipelineResult<()> {
    
    let file_path = cache_path.parent().unwrap().to_string_lossy();
    let command = format!("soffice --headless --convert-to pdf --outdir {file_path} {working_file:?}");
    
    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output();
    
    match output {
        Ok(output) => match output.status.success() {
            true => {
                index(cache_path.to_string_lossy().to_string(), PathBuf::from(working_file));
                Ok(())
            },
            false => Err(PipelineError(format!("Failed to convert document to PDF: {}", String::from_utf8_lossy(&output.stderr))))
        },
        Err(error) => Err(PipelineError(format!("Failed to convert document to PDF: {}", error)))
    }
    
}