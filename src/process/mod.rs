mod document;
mod download;
mod outline;
pub mod pipeline;
mod raw;
pub mod source;

use crate::enums::download::Download;
use crate::enums::force::Force;
use crate::enums::input::InputFormat;
use crate::enums::output_format::{OutputFormat, get_output_extension, get_output_mime};
use crate::params::RequestParams;
use crate::params::parsed::Parameters;
use crate::process::source::Source;
use crate::services::http_cache::{self, Validators};
use crate::services::signature::verify_signature;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::Response;
use bytes::Bytes;
use download::apply_disposition;
use pipeline::request::PipelineRequest;
use std::future::Future;
use std::path::Path as FileSystemPath;
use tracing::debug;

use crate::services::cache::CacheStore;

pub async fn process_file(
    headers: HeaderMap,
    uri: Uri,
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    Query(params): Query<RequestParams>,
) -> Response {
    if !verify_signature(&state.config, &uri) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(header::CACHE_CONTROL, http_cache::NO_STORE)
            .body(Body::from("Invalid signature"))
            .unwrap();
    }

    let mut source = match Source::new(&state.config, &file_path, &params) {
        Ok(source) => source,
        Err(e) => {
            debug!("Source resolution failed for {file_path}: {e}");
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/html; charset=utf-8")
                .header(header::CACHE_CONTROL, http_cache::NO_STORE)
                .body(Body::from(include_str!("../../templates/404.html")))
                .unwrap();
        }
    };

    let forced = params.force == Some(Force::True);
    let parameters = Parameters::new(&state.config, params);
    
    let cache_control = if forced {
        http_cache::NO_STORE
    } else {
        state.config.cache.cache_control.as_str()
    };
    
    let passthrough = parameters.original == true || matches!(source.format, InputFormat::Unsupported);

    if passthrough {
        if parameters.original != true && !state.config.data.may_serve(&source.path) {
            debug!(
                "Refusing to serve unprocessable file {}",
                source.path.display()
            );
            return unsupported_source(&source.path);
        }

        let _permit = match acquire_permit(&state).await {
            Ok(permit) => permit,
            Err(response) => return response,
        };

        let _guard = scopeguard::guard(state.multithreading.clone(), |mt| mt.release_worker());

        return serve_raw(&headers, &source.path, &parameters.download, cache_control, forced).await;
    }

    let pipeline_request = PipelineRequest::new(&headers, &state, &mut source, &parameters, forced);

    let validators = match tokio::fs::metadata(&pipeline_request.source.path).await {
        Ok(metadata) => Some(Validators::new(
            &state.etag_seed,
            &pipeline_request.source.path,
            &metadata,
            &parameters,
            &pipeline_request.output_format,
        )),
        Err(e) => {
            debug!(
                "Cannot read metadata of {}: {e}",
                pipeline_request.source.path.display()
            );
            None
        }
    };

    let vary = negotiated_vary(&parameters);

    if !forced && let Some(validators) = &validators && validators.is_not_modified(&headers) {
        return validators
            .not_modified(cache_control, vary)
            .header("Content-Type", get_output_mime(&pipeline_request.output_format))
            .body(Body::empty())
            .unwrap();
    }

    if !forced && let Some(validators) = &validators && let Some(result) = state.cache.get(&validators.cache_key).await {
        return create_response(
            &pipeline_request.output_format,
            &pipeline_request.source.path,
            &pipeline_request.parameters.download,
            Body::from(result),
            Some(validators),
            cache_control,
            vary,
        );
    }

    if matches!(pipeline_request.output_format, OutputFormat::Pdf | OutputFormat::Svg) {
        let _permit = match acquire_permit(&state).await {
            Ok(permit) => permit,
            Err(response) => return response,
        };

        let _guard = scopeguard::guard(state.multithreading.clone(), |mt| mt.release_worker());

        return document::serve(
                &headers,
                &pipeline_request,
                validators.as_ref(),
                cache_control,
            ).await
            .unwrap_or_else(|e| {
                tracing::error!("Error serving document: {e}");
                error_response("Error serving document")
            });
    }

    let output_format = pipeline_request.output_format.clone();
    let source_path = pipeline_request.source.path.clone();
    let download = pipeline_request.parameters.download.clone();
    let cache_key = validators.as_ref().map(|validators| validators.cache_key.clone());
    
    drop(pipeline_request);

    let render_state = state.clone();
    let render_headers = headers.clone();
    let result = render_response(&state.cache, cache_key, forced, move || async move {
        let mut pipeline_request = PipelineRequest::new(
            &render_headers,
            &render_state,
            &mut source,
            &parameters,
            forced,
        );

        render_raster(&mut pipeline_request).await
    }).await;

    let result = match result {
        Ok(result) => result,
        Err(error) if error.chain().any(|cause| cause.is::<WorkerQueueFull>()) => {
            return too_many_requests_response();
        }
        Err(error) => {
            tracing::error!("Error processing file: {error}");
            return error_response("Error processing file");
        }
    };

    create_response(
        &output_format,
        &source_path,
        &download,
        Body::from(result),
        validators.as_ref(),
        cache_control,
        vary,
    )
}

async fn render_response<F, Fut>(cache: &CacheStore, cache_key: Option<String>, forced: bool, render: F) -> anyhow::Result<Bytes>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<Bytes>> + Send + 'static,
{
    match cache_key {
        Some(cache_key) => cache.resolve(cache_key, forced, render).await,
        None => render().await,
    }
}

async fn render_raster(request: &mut PipelineRequest<'_>) -> anyhow::Result<Bytes> {
    let _permit = request.state.multithreading.get_permit().await.ok_or(WorkerQueueFull)?;
    let _guard = scopeguard::guard(request.state.multithreading.clone(), |mt| {
        mt.release_worker()
    });

    debug!(
        "Output format: {:?}, through: {:?}",
        request.output_format, request.intermediate_format
    );

    let source_path = pipeline::resolve_source_path(request).await?;
    let source_path_text = source_path.path().to_string_lossy();
    let result = tokio::task::block_in_place(|| pipeline::process_image(request, &source_path_text))?;

    Ok(Bytes::from(result))
}

#[derive(Debug)]
struct WorkerQueueFull;

impl std::fmt::Display for WorkerQueueFull {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("worker queue is full")
    }
}

impl std::error::Error for WorkerQueueFull {}

/// `Vary: Accept` only when the format really was negotiated from the header
fn negotiated_vary(parameters: &Parameters) -> Option<&'static str> {
    (parameters.format == OutputFormat::Auto).then_some("Accept")
}

async fn acquire_permit(state: &AppState) -> Result<tokio::sync::SemaphorePermit<'_>, Response> {
    match state.multithreading.get_permit().await {
        Some(permit) => Ok(permit),
        None => Err(too_many_requests_response()),
    }
}

fn too_many_requests_response() -> Response {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CACHE_CONTROL, http_cache::NO_STORE)
        .body(Body::from("Too many requests"))
        .unwrap()
}

fn error_response(message: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CACHE_CONTROL, http_cache::NO_STORE)
        .body(Body::from(message))
        .unwrap()
}

async fn serve_raw(headers: &HeaderMap, path: &std::path::Path, download: &Download, cache_control: &str, forced: bool) -> Response {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");

    match raw::serve(headers, path, download, name, cache_control, forced).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Error serving {}: {e}", path.display());
            error_response("Error serving file")
        }
    }
}

fn unsupported_source(path: &std::path::Path) -> Response {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");

    Response::builder()
        .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header(header::CACHE_CONTROL, http_cache::NO_STORE)
        .body(Body::from(format!(
            "Unsupported source file type \"{extension}\""
        )))
        .unwrap()
}

fn create_response(
    output_format: &OutputFormat,
    source_path: &FileSystemPath,
    download: &Download,
    result: Body,
    validators: Option<&Validators>,
    cache_control: &str,
    vary: Option<&str>,
) -> Response {
    // Derive a download filename from the source stem + the resolved output
    // extension (e.g. `photo.png` converted to jpeg → `photo.jpg`).
    let fallback_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|stem| format!("{}.{}", stem, get_output_extension(output_format)));

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", get_output_mime(output_format));

    builder = match validators {
        Some(validators) => validators.apply(builder, cache_control, vary),
        None => http_cache::apply(builder, None, None, cache_control, vary),
    };

    if let Some(name) = fallback_name {
        builder = apply_disposition(builder, download, &name);
    }

    builder.body(result).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn identical_response_misses_render_once() {
        let root = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.cache.dir = root.path().to_string_lossy().into_owned();
        config.cache.memory.enabled = true;
        config.cache.memory.capacity = 2;
        config.cache.memory.entry_limit = 1;
        config.cache.disk.enabled = false;
        let cache = CacheStore::new(&config).await.unwrap();
        let calls = Arc::new(AtomicUsize::new(0));

        let first_calls = Arc::clone(&calls);
        let first = render_response(
            &cache,
            Some("response:key".into()),
            false,
            move || async move {
                first_calls.fetch_add(1, Ordering::Relaxed);
                tokio::task::yield_now().await;
                Ok(Bytes::from_static(b"rendered"))
            },
        );
        let second_calls = Arc::clone(&calls);
        let second = render_response(
            &cache,
            Some("response:key".into()),
            false,
            move || async move {
                second_calls.fetch_add(1, Ordering::Relaxed);
                Ok(Bytes::from_static(b"duplicate"))
            },
        );

        let (first, second) = tokio::join!(first, second);

        assert_eq!(first.unwrap(), Bytes::from_static(b"rendered"));
        assert_eq!(second.unwrap(), Bytes::from_static(b"rendered"));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
