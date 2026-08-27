use super::conversion::{convert_to_pdf, wait_for_conversion};
use super::spawn_full_conversion;
use crate::enums::input::{InputFormat, OfficeInputFormat};
use crate::enums::output_format::OutputFormat;
use crate::process::pipeline::ResolvedSource;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::source_key;
use anyhow::{Result, anyhow};
use std::{path::PathBuf, time::Duration};

const PDF_FILTER_DATA: &str = r#"{"PageRange":{"type":"string","value":"1"}}"#;

pub(super) async fn process(
    source_path: PathBuf,
    full_key: String,
    duration: Duration,
    request: &PipelineRequest<'_>,
) -> Result<ResolvedSource> {
    let forced = request.forced;

    if !forced && let Some(pdf) = request.state.cache.get(&full_key).await {
        return ResolvedSource::materialize(&pdf, ".pdf").await;
    }

    let first_key = source_key("office:first", &request.state.etag_seed, &source_path, "").await?;
    let filter = pdf_filter(&request.source.format)?;
    let first_source = source_path.clone();
    let cache = request.state.cache.clone();
    let full_cache = request.state.cache.clone();
    
    let mut conversion = tokio::spawn(async move {
        let convert = move || async move { convert_to_pdf(&first_source, Some(&filter)).await };
        let result = cache.resolve(first_key, forced, convert).await;

        if result.is_ok() {
            spawn_full_conversion(full_cache, full_key, source_path, forced);
        }

        result
    });
    
    let pdf = wait_for_conversion(&mut conversion, duration).await?;

    ResolvedSource::materialize(&pdf, ".pdf").await
}

pub(super) fn is_requested(pages: &Option<Vec<u32>>, output_format: &OutputFormat) -> bool {
    match pages {
        Some(pages) => pages.iter().all(|&page| page == 1),
        None => output_format != &OutputFormat::Pdf,
    }
}

fn pdf_filter(input_format: &InputFormat) -> Result<String> {
    let filter_name = match input_format {
        InputFormat::Office(OfficeInputFormat::Doc) => "writer_pdf_Export",
        InputFormat::Office(OfficeInputFormat::Ppt) => "impress_pdf_Export",
        InputFormat::Office(OfficeInputFormat::Xls) => "calc_pdf_Export",
        _ => return Err(anyhow!("expected an office input format")),
    };

    Ok(format!("pdf:{filter_name}:{PDF_FILTER_DATA}"))
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
