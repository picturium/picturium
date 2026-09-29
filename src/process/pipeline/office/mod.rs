mod conversion;
mod first_page;

use crate::process::pipeline::ResolvedSource;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::cache::{CacheStore, source_key};
use anyhow::Result;
use bytes::Bytes;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task::JoinHandle};
use tracing::error;

use self::conversion::{convert_to_pdf, wait_for_conversion};

pub async fn process(request: &PipelineRequest<'_>) -> Result<ResolvedSource> {
    let source_path = request.source.path.clone();
    let duration = Duration::from_secs(request.state.config.office.conversion_timeout);
    let full_key = source_key("office:full", &request.state.etag_seed, &source_path, "").await?;

    if first_page::is_requested(&request.parameters.pages, &request.output_format) {
        return first_page::process(source_path, full_key, duration, request).await;
    }

    let mut conversion = spawn_full_conversion(
        request.state.cache.clone(),
        full_key,
        source_path,
        request.forced,
        Duration::from_secs(request.state.config.office.kill_timeout),
        request.state.soffice.clone(),
    );
    
    let pdf = wait_for_conversion(&mut conversion, duration).await?;
    ResolvedSource::materialize(&pdf, ".pdf").await
}

pub(super) fn spawn_full_conversion(
    cache: CacheStore,
    key: String,
    source_path: PathBuf,
    forced: bool,
    kill_timeout: Duration,
    processes: Arc<Semaphore>,
) -> JoinHandle<Result<Bytes>> {
    tokio::spawn(async move {
        let convert = move || async move { convert_to_pdf(&source_path, None, kill_timeout, processes).await };
        let result = cache.resolve(key, forced, convert).await;

        if let Err(error) = &result {
            error!(%error, "background soffice conversion failed");
        }

        result
    })
}
