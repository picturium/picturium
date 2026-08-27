use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use std::{path::Path, time::Duration};
use tokio::process::Command;
use tokio::{task::JoinHandle, time::timeout};

pub(super) async fn convert_to_pdf(source_path: &Path, filter: Option<&str>) -> Result<Bytes> {
    let output = tempfile::tempdir().context("failed to create soffice output directory")?;
    
    let produced_pdf = output
        .path()
        .join(
            source_path
                .file_stem()
                .with_context(|| format!("invalid source path: {}", source_path.display()))?,
        )
        .with_extension("pdf");
    
    let filter = filter.unwrap_or("pdf");

    let status = Command::new("soffice")
        .arg("--headless")
        .arg("--nologo")
        .arg("--nodefault")
        .arg("--norestore")
        .arg("--convert-to")
        .arg(filter)
        .arg("--outdir")
        .arg(output.path())
        .arg(source_path)
        .status()
        .await
        .context("failed to run soffice command")?;

    if !status.success() {
        return Err(anyhow!(
            "soffice failed with exit code: {:?}",
            status.code()
        ));
    }

    tokio::fs::read(&produced_pdf)
        .await
        .map(Bytes::from)
        .with_context(|| format!("failed to read soffice output {}", produced_pdf.display()))
}

pub(super) async fn wait_for_conversion(conversion: &mut JoinHandle<Result<Bytes>>, duration: Duration) -> Result<Bytes> {
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
    use bytes::Bytes;
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
            Ok::<_, anyhow::Error>(Bytes::new())
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
