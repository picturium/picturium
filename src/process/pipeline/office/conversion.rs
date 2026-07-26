use crate::services::cache::sidecar;
use anyhow::{Context, Result, anyhow};
use std::{path::Path, time::Duration};
use tokio::process::Command;
use tokio::{task::JoinHandle, time::timeout};

pub(super) async fn convert_to_pdf(
    source_path: &Path,
    pdf_path: &str,
    filter: Option<&str>,
) -> Result<()> {
    let pdf_dir = Path::new(pdf_path)
        .parent()
        .with_context(|| "Invalid pdf path")?;
    let produced_pdf = pdf_dir
        .join(
            source_path
                .file_stem()
                .with_context(|| format!("Invalid source path: {}", source_path.display()))?,
        )
        .with_extension("pdf");
    let filter = filter.unwrap_or("pdf");

    println!(
        "Command: soffice --headless --convert-to {filter} --outdir {} {}",
        pdf_dir.display(),
        source_path.display()
    );

    let mut child = Command::new("soffice")
        .arg("--headless")
        .arg("--nologo")
        .arg("--nodefault")
        .arg("--norestore")
        .arg("--convert-to")
        .arg(filter)
        .arg("--outdir")
        .arg(pdf_dir)
        .arg(source_path)
        .spawn()
        .with_context(|| "Failed to spawn soffice command")?;

    match child.wait().await {
        Ok(status) if status.success() => {
            tokio::fs::rename(&produced_pdf, pdf_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to rename soffice output from {} to {pdf_path}",
                        produced_pdf.display(),
                    )
                })?;

            sidecar::write(pdf_path, source_path).await?;
            Ok(())
        }
        Ok(status) => Err(anyhow!(
            "soffice failed with exit code: {:?}",
            status.code()
        )),
        Err(err) => Err(err).with_context(|| "Failed to wait on soffice process"),
    }
}

pub(super) async fn wait_for_conversion(
    conversion: &mut JoinHandle<Result<()>>,
    duration: Duration,
) -> Result<()> {
    match timeout(duration, conversion).await {
        Ok(result) => result.context("soffice conversion task failed")?,
        Err(_) => Err(anyhow!(
            "soffice conversion is continuing in the background after {} seconds",
            duration.as_secs()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::wait_for_conversion;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tokio::time::sleep;

    #[tokio::test]
    async fn timeout_does_not_cancel_conversion_task() {
        let completed = Arc::new(AtomicBool::new(false));
        let task_completed = Arc::clone(&completed);
        let mut conversion = tokio::spawn(async move {
            sleep(Duration::from_millis(20)).await;
            task_completed.store(true, Ordering::Release);
            Ok::<(), anyhow::Error>(())
        });

        assert!(
            wait_for_conversion(&mut conversion, Duration::from_millis(1))
                .await
                .is_err()
        );

        drop(conversion);
        sleep(Duration::from_millis(30)).await;

        assert!(completed.load(Ordering::Acquire));
    }
}
