use crate::process::pipeline::ResolvedSource;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::source_key;
use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy)]
enum Target {
    Pdf,
    Svg,
}

impl Target {
    fn extension(self) -> &'static str {
        match self {
            Self::Pdf => ".pdf",
            Self::Svg => ".svg",
        }
    }

    fn namespace(self) -> &'static str {
        match self {
            Self::Pdf => "vector:pdf",
            Self::Svg => "vector:svg",
        }
    }
}

pub async fn process(request: &PipelineRequest<'_>) -> Result<ResolvedSource> {
    convert(request, Target::Pdf).await
}

pub async fn process_svg(request: &PipelineRequest<'_>) -> Result<ResolvedSource> {
    convert(request, Target::Svg).await
}

async fn convert(request: &PipelineRequest<'_>, target: Target) -> Result<ResolvedSource> {
    let source_path = request.source.path.clone();
    let duration = Duration::from_secs(request.state.config.vector.conversion_timeout);

    let key = source_key(
        target.namespace(),
        &request.state.etag_seed,
        &source_path,
        "",
    ).await?;

    let export = move || async move { export(&source_path, target, duration).await };
    let value = request.state.cache.resolve(key, request.forced, export).await?;

    ResolvedSource::materialize(&value, target.extension()).await
}

async fn export(source_path: &Path, target: Target, conversion_timeout: Duration) -> Result<Bytes> {
    let output = tempfile::Builder::new()
        .prefix("picturium-vector-")
        .suffix(target.extension())
        .tempfile()
        .context("failed to create temporary inkscape output")?;

    run_inkscape(source_path, output.path(), target, conversion_timeout).await?;

    tokio::fs::read(output.path()).await
        .map(Bytes::from)
        .context("failed to read inkscape output")
}

async fn run_inkscape(source_path: &Path, output_path: &Path, target: Target, conversion_timeout: Duration) -> Result<()> {
    let mut command = Command::new("inkscape");

    command.arg(match target {
        Target::Pdf => "--export-type=pdf",
        Target::Svg => "--export-type=svg",
    });

    if let Target::Svg = target {
        command.arg("--export-plain-svg");
    }

    let mut child = command
        .arg("--export-area-page")
        .arg("--export-overwrite")
        .arg("--export-filename")
        .arg(output_path)
        .arg(source_path)
        .spawn()
        .context("failed to spawn inkscape command")?;

    let status = match timeout(conversion_timeout, child.wait()).await {
        Ok(status) => status.context("failed to wait on inkscape process")?,
        Err(_) => {
            child.kill().await.ok();
            return Err(anyhow!(
                "inkscape conversion timed out after {} seconds",
                conversion_timeout.as_secs()
            ));
        }
    };

    if !status.success() {
        return Err(anyhow!("inkscape failed with exit code: {:?}", status.code()));
    }

    // Inkscape exits 0 when an importer silently fails, leaving the file empty.
    let written = tokio::fs::metadata(output_path).await
        .is_ok_and(|metadata| metadata.len() > 0);

    match written {
        true => Ok(()),
        false => Err(anyhow!("inkscape produced no output, the source is unreadable")),
    }
}

#[cfg(test)]
mod tests {
    use super::Target;

    #[test]
    fn the_two_targets_never_share_a_cache_entry() {
        assert_ne!(Target::Pdf.namespace(), Target::Svg.namespace());
        assert_ne!(Target::Pdf.extension(), Target::Svg.extension());
    }
}
