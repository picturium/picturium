use anyhow::Result;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tokio::io::AsyncWriteExt;

/// Returns the sidecar path for a given intermediate file path.
pub fn sidecar_path(intermediate_path: &str) -> String {
    format!("{}.meta", intermediate_path)
}

/// Checks whether the cached intermediate file is still valid by comparing
/// mtime and size recorded in the sidecar against the current source file metadata.
pub async fn is_valid(intermediate_path: &str, source_path: &Path) -> bool {
    let sidecar = sidecar_path(intermediate_path);

    let recorded = match tokio::fs::read_to_string(&sidecar).await {
        Ok(content) => content,
        Err(_) => return false,
    };

    let mut parts = recorded.trim().splitn(2, ' ');
    let (recorded_mtime, recorded_size) = match (parts.next(), parts.next()) {
        (Some(m), Some(s)) => (m, s),
        _ => return false,
    };

    let meta = match tokio::fs::metadata(source_path).await {
        Ok(m) => m,
        Err(_) => return false,
    };

    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_default();

    let size = meta.len().to_string();

    mtime == recorded_mtime && size == recorded_size
}

/// Writes a sidecar file recording the source file's current mtime and size.
pub async fn write(intermediate_path: &str, source_path: &Path) -> Result<()> {
    let meta = tokio::fs::metadata(source_path).await?;

    let mtime = meta
        .modified()?
        .duration_since(UNIX_EPOCH)?
        .as_nanos();

    let size = meta.len();

    let sidecar = sidecar_path(intermediate_path);
    let mut file = tokio::fs::File::create(&sidecar).await?;
    file.write_all(format!("{} {}", mtime, size).as_bytes()).await?;

    Ok(())
}