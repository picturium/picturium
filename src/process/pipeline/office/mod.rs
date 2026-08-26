mod conversion;
mod first_page;

use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::path_generator::generate_intermediate_path;
use crate::services::cache::sidecar;
use anyhow::{Context, Result};
use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::task::JoinHandle;
use tracing::error;

use self::conversion::{convert_to_pdf, wait_for_conversion};
use super::lock::acquire_conversion_lock;

pub async fn process(request: &PipelineRequest<'_>) -> Result<String> {
    let source_path = &request.source.path;
    let duration = Duration::from_secs(request.state.config.office.conversion_timeout);
    let pdf_path = generate_intermediate_path(request, source_path, "pdf");

    if sidecar::is_valid(&pdf_path, source_path).await {
        return Ok(pdf_path);
    }

    let requested_pages = &request.parameters.pages;

    if first_page::is_requested(requested_pages, &request.output_format) {
        return first_page::process(source_path, &pdf_path, duration, request).await;
    }

    let pdf_dir = Path::new(&pdf_path)
        .parent()
        .with_context(|| "Invalid pdf path")?;

    tokio::fs::create_dir_all(pdf_dir).await?;

    let conversion_lock = acquire_conversion_lock(&pdf_path).await?;

    if sidecar::is_valid(&pdf_path, source_path).await {
        return Ok(pdf_path);
    }

    let mut conversion = spawn_full_conversion(
        source_path.to_owned(),
        pdf_path.clone(),
        first_page::pdf_path(request, source_path),
        conversion_lock,
    );
    wait_for_conversion(&mut conversion, duration).await?;

    Ok(pdf_path)
}

pub(super) fn spawn_full_conversion(
    source_path: PathBuf,
    pdf_path: String,
    first_page_pdf_path: String,
    conversion_lock: File,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let _conversion_lock = conversion_lock;

        let result = async {
            if !sidecar::is_valid(&pdf_path, &source_path).await {
                convert_to_pdf(&source_path, &pdf_path, None).await?;
            }

            first_page::remove_pdf(&first_page_pdf_path).await;
            Ok(())
        }
        .await;

        if let Err(err) = &result {
            error!("background soffice conversion failed: {err:#}");
        }

        result
    })
}
