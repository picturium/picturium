use super::conversion::{convert_to_pdf, wait_for_conversion};
use super::super::lock::{acquire_conversion_lock, try_acquire_conversion_lock};
use super::spawn_full_conversion;
use crate::enums::input::{InputFormat, OfficeInputFormat};
use crate::enums::output_format::OutputFormat;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::path_generator::generate_intermediate_path;
use crate::services::cache::sidecar;
use anyhow::{Context, Result, anyhow};
use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};
use tracing::error;

const PDF_SUFFIX: &str = "first-page.pdf";
const PDF_FILTER_DATA: &str = r#"{"PageRange":{"type":"string","value":"1"}}"#;

pub(super) async fn process(
    source_path: &PathBuf,
    full_pdf_path: &str,
    duration: Duration,
    request: &PipelineRequest<'_>,
) -> Result<String> {
    let first_page_pdf_path = pdf_path(request, source_path);

    if sidecar::is_valid(&first_page_pdf_path, source_path).await {
        start_full_conversion(
            source_path.to_owned(),
            full_pdf_path.to_owned(),
            first_page_pdf_path.clone(),
        )
        .await?;
        return Ok(first_page_pdf_path);
    }

    let first_page_pdf_dir = Path::new(&first_page_pdf_path)
        .parent()
        .with_context(|| "Invalid first-page pdf path")?;

    tokio::fs::create_dir_all(first_page_pdf_dir).await?;

    let _first_page_conversion_lock = acquire_conversion_lock(&first_page_pdf_path).await?;

    if sidecar::is_valid(full_pdf_path, source_path).await {
        return Ok(full_pdf_path.to_owned());
    }

    if sidecar::is_valid(&first_page_pdf_path, source_path).await {
        start_full_conversion(
            source_path.to_owned(),
            full_pdf_path.to_owned(),
            first_page_pdf_path.clone(),
        )
        .await?;
        return Ok(first_page_pdf_path);
    }

    let full_conversion_lock = match try_acquire_conversion_lock(full_pdf_path).await? {
        Some(lock) => lock,
        None => acquire_conversion_lock(full_pdf_path).await?,
    };

    if sidecar::is_valid(full_pdf_path, source_path).await {
        return Ok(full_pdf_path.to_owned());
    }

    let mut conversion = spawn_first_page_conversion(
        source_path.to_owned(),
        first_page_pdf_path.clone(),
        full_pdf_path.to_owned(),
        pdf_filter(request)?,
        _first_page_conversion_lock,
        full_conversion_lock,
    );
    wait_for_conversion(&mut conversion, duration).await?;

    Ok(first_page_pdf_path)
}

pub(super) fn is_requested(pages: &Option<Vec<u32>>, output_format: &OutputFormat) -> bool {
    match pages {
        Some(pages) => pages.iter().all(|&page| page == 1),
        None => output_format != &OutputFormat::Pdf,
    }
}

pub(super) fn pdf_path(request: &PipelineRequest<'_>, source_path: &PathBuf) -> String {
    generate_intermediate_path(request, source_path, PDF_SUFFIX)
}

pub(super) async fn remove_pdf(first_page_pdf_path: &str) {
    for path in [
        first_page_pdf_path.to_owned(),
        sidecar::sidecar_path(first_page_pdf_path),
    ] {
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => error!("failed to remove temporary first-page pdf {path}: {err}"),
        }
    }
}

fn pdf_filter(request: &PipelineRequest<'_>) -> Result<String> {
    let filter_name = match request.source.format {
        InputFormat::Office(OfficeInputFormat::Doc) => "writer_pdf_Export",
        InputFormat::Office(OfficeInputFormat::Ppt) => "impress_pdf_Export",
        InputFormat::Office(OfficeInputFormat::Xls) => "calc_pdf_Export",
        _ => return Err(anyhow!("expected an office input format")),
    };

    Ok(format!("pdf:{filter_name}:{PDF_FILTER_DATA}"))
}

async fn start_full_conversion(
    source_path: PathBuf,
    pdf_path: String,
    first_page_pdf_path: String,
) -> Result<()> {
    if let Some(lock) = try_acquire_conversion_lock(&pdf_path).await? {
        spawn_full_conversion(source_path, pdf_path, first_page_pdf_path, lock);
    }

    Ok(())
}

fn spawn_first_page_conversion(
    source_path: PathBuf,
    first_page_pdf_path: String,
    full_pdf_path: String,
    filter: String,
    first_page_conversion_lock: File,
    conversion_lock: File,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let _first_page_conversion_lock = first_page_conversion_lock;
        let result = async {
            convert_to_pdf(&source_path, &first_page_pdf_path, Some(&filter)).await?;

            spawn_full_conversion(
                source_path,
                full_pdf_path,
                first_page_pdf_path,
                conversion_lock,
            );

            Ok(())
        }
        .await;

        if let Err(err) = &result {
            error!("background first-page soffice conversion failed: {err:#}");
        }

        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_request_only_needs_the_first_page() {
        assert!(is_requested(&None, &OutputFormat::Webp));
    }

    #[test]
    fn asking_for_the_document_as_pdf_needs_every_page() {
        assert!(!is_requested(&None, &OutputFormat::Pdf));
    }

    #[test]
    fn an_explicit_page_selection_decides_regardless_of_output_format() {
        assert!(is_requested(&Some(vec![1]), &OutputFormat::Pdf));
        assert!(!is_requested(&Some(vec![2]), &OutputFormat::Webp));
        assert!(!is_requested(&Some(vec![1, 2]), &OutputFormat::Pdf));
    }
}
