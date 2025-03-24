use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use picturium_libvips::{ThumbnailOptions, VipsImage, VipsIntent, VipsInteresting, VipsThumbnails};
use crate::cache;
use crate::cache::{get_document_path_from_url_parameters, index};
use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::services::dimensions::get_rasterize_dimensions;
use crate::services::formats::{get_extension};

pub(crate) async fn run(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let extension = match get_extension(url_parameters.path) {
        Ok(extension) => extension,
        Err(_) => return Err(PipelineError("Failed to determine file extension".to_string()))
    };

    match extension.as_str() {
        "pdf" => generate_pdf_thumbnail(working_file, url_parameters),
        "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp" | "rtf" => generate_document_thumbnail(working_file, url_parameters),
        _ => Err(PipelineError("Unsupported file format".to_string()))
    }

}

fn generate_pdf_thumbnail(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {

    let pdf = match VipsImage::new_from_file(&working_file.to_string_lossy(), None) {
        Ok(image) => image,
        Err(e) => return Err(PipelineError(format!("Failed to open PDF file: {e}")))
    };

    let page = url_parameters.thumbnail.page.unwrap_or(1);
    let page_parameter = format!("[page={}]", (page - 1).min(pdf.get_page_count() as u32 - 1));

    let pdf = match VipsImage::new_from_file(&(working_file.to_string_lossy() + &page_parameter[..]), None) {
        Ok(image) => image,
        Err(e) => return Err(PipelineError(format!("Failed to open PDF page {page_parameter}: {e}")))
    };

    let (width, height) = get_rasterize_dimensions(&pdf, url_parameters);

    match VipsImage::thumbnail(&(working_file.to_string_lossy() + &page_parameter[..]), width, ThumbnailOptions {
        height,
        intent: VipsIntent::Perceptual,
        crop: VipsInteresting::Centre,
        ..Default::default()
    }.into()) {
        Ok(image) => Ok(image),
        Err(e) => Err(PipelineError(format!("Failed to generate thumbnail for PDF file {working_file:?}: {e}")))
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