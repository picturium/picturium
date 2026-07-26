use anyhow::{Context, Result};
use fs4::{FileExt, TryLockError};
use std::fs::{File, OpenOptions};

pub(super) async fn acquire_conversion_lock(pdf_path: &str) -> Result<File> {
    let lock_path = format!("{pdf_path}.lock");

    tokio::task::spawn_blocking(move || -> Result<File> {
        let lock_file = open_lock_file(&lock_path)?;

        FileExt::lock(&lock_file)
            .with_context(|| format!("failed to acquire conversion lock {lock_path}"))?;

        Ok(lock_file)
    })
    .await
    .context("conversion lock task failed")?
}

pub(super) async fn try_acquire_conversion_lock(pdf_path: &str) -> Result<Option<File>> {
    let lock_path = format!("{pdf_path}.lock");

    tokio::task::spawn_blocking(move || -> Result<Option<File>> {
        let lock_file = open_lock_file(&lock_path)?;

        match <File as FileExt>::try_lock(&lock_file) {
            Ok(()) => Ok(Some(lock_file)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(err)) => {
                Err(err).with_context(|| format!("failed to acquire conversion lock {lock_path}"))
            }
        }
    })
    .await
    .context("conversion lock task failed")?
}

fn open_lock_file(lock_path: &str) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)
        .with_context(|| format!("failed to open conversion lock {lock_path}"))
}
