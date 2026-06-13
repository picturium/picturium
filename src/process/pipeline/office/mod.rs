use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::path_generator::generate_intermediate_path;
use crate::services::cache::sidecar;
use anyhow::{anyhow, Context, Result};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub async fn process(request: &PipelineRequest<'_>) -> Result<String> {
    let source_path = &request.source.path;
    let duration = Duration::from_secs(request.state.config.office.conversion_timeout);

    let pdf_path = generate_intermediate_path(request, source_path, "pdf");

    if sidecar::is_valid(&pdf_path, source_path).await {
        return Ok(pdf_path);
    }

    let pdf_dir = std::path::Path::new(&pdf_path)
        .parent()
        .with_context(|| "Invalid pdf path")?
        .to_owned();
    tokio::fs::create_dir_all(&pdf_dir).await?;

    let mut child = Command::new("soffice")
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(&pdf_dir)
        .arg(source_path)
        .spawn()
        .with_context(|| "Failed to spawn soffice command")?;

    match timeout(duration, child.wait()).await {
        Ok(Ok(status)) if status.success() => {
            sidecar::write(&pdf_path, source_path).await?;
            Ok(pdf_path)
        }
        Ok(Ok(status)) => Err(anyhow!("soffice failed with exit code: {:?}", status.code())),
        Ok(Err(e)) => Err(e).with_context(|| "Failed to wait on soffice process"),
        Err(_) => {
            let _ = child.kill().await;
            Err(anyhow!("soffice conversion timed out after {} seconds", duration.as_secs()))
        }
    }
}