mod document;
mod download;
mod outline;
pub mod pipeline;
mod raw;
pub mod source;

use crate::enums::download::Download;
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
use download::apply_disposition;
use pipeline::request::PipelineRequest;
use tracing::debug;

pub async fn process_file(
    headers: HeaderMap,
    uri: Uri,
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    Query(params): Query<RequestParams>,
) -> Response {
    let cache_control = state.config.cache.cache_control.as_str();

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

    let parameters = Parameters::new(&state.config, params);
    let passthrough = parameters.original == true || matches!(source.format, InputFormat::Unsupported);

    if passthrough {
        if parameters.original != true && !state.config.data.may_serve(&source.path) {
            debug!("Refusing to serve unprocessable file {}", source.path.display());
            return unsupported_source(&source.path);
        }

        let _permit = match acquire_permit(&state).await {
            Ok(permit) => permit,
            Err(response) => return response,
        };

        let _guard = scopeguard::guard(state.multithreading.clone(), |mt| mt.release_worker());

        return serve_raw(&headers, &source.path, &parameters.download, cache_control).await;
    }

    let mut pipeline_request = PipelineRequest::new(&headers, &state, &mut source, &parameters);

    let validators = match tokio::fs::metadata(&pipeline_request.source.path).await {
        Ok(metadata) => Some(Validators::new(
            &state.etag_seed,
            &metadata,
            &parameters,
            &pipeline_request.output_format,
        )),
        Err(e) => {
            debug!("Cannot read metadata of {}: {e}", pipeline_request.source.path.display());
            None
        }
    };

    let vary = negotiated_vary(&parameters);

    if let Some(validators) = &validators && validators.is_not_modified(&headers) {
        return validators
            .not_modified(cache_control, vary)
            .header("Content-Type", get_output_mime(&pipeline_request.output_format))
            .body(Body::empty())
            .unwrap();
    }

    let _permit = match acquire_permit(&state).await {
        Ok(permit) => permit,
        Err(response) => return response,
    };

    let _guard = scopeguard::guard(state.multithreading.clone(), |mt| mt.release_worker());

    if matches!(pipeline_request.output_format, OutputFormat::Pdf | OutputFormat::Svg) {
        return document::serve(&headers, &pipeline_request, validators.as_ref(), cache_control)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Error serving document: {e}");
                error_response("Error serving document")
            });
    }

    debug!(
        "Output format: {:?}, through: {:?}",
        pipeline_request.output_format, pipeline_request.intermediate_format
    );

    let source_path = match pipeline::resolve_source_path(&pipeline_request).await {
        Ok(path) => path,
        Err(e) => {
            tracing::error!("Error in pre-pipeline: {e}");
            return error_response("Error processing file");
        }
    };

    let result = match tokio::task::block_in_place(|| {
        pipeline::process_image(&mut pipeline_request, &source_path)
    }) {
        Ok(result) => Body::from(result),
        Err(e) => {
            tracing::error!("Error processing file: {e}");
            return error_response("Error processing file");
        }
    };

    create_response(&pipeline_request, result, validators.as_ref(), cache_control, vary)
}

/// `Vary: Accept` only when the format really was negotiated from the header
fn negotiated_vary(parameters: &Parameters) -> Option<&'static str> {
    (parameters.format == OutputFormat::Auto).then_some("Accept")
}

async fn acquire_permit(state: &AppState) -> Result<tokio::sync::SemaphorePermit<'_>, Response> {
    match state.multithreading.get_permit().await {
        Some(permit) => Ok(permit),
        None => Err(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(header::CACHE_CONTROL, http_cache::NO_STORE)
            .body(Body::from("Too many requests"))
            .unwrap()),
    }
}

fn error_response(message: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CACHE_CONTROL, http_cache::NO_STORE)
        .body(Body::from(message))
        .unwrap()
}

async fn serve_raw(headers: &HeaderMap, path: &std::path::Path, download: &Download, cache_control: &str) -> Response {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");

    match raw::serve(headers, path, download, name, cache_control).await {
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

fn create_response(pipeline_request: &PipelineRequest, result: Body, validators: Option<&Validators>, cache_control: &str, vary: Option<&str>) -> Response {
    let output_format = &pipeline_request.output_format;

    // Derive a download filename from the source stem + the resolved output
    // extension (e.g. `photo.png` converted to jpeg → `photo.jpg`).
    let fallback_name = pipeline_request
        .source
        .path
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
        builder = apply_disposition(builder, &pipeline_request.parameters.download, &name);
    }

    builder.body(result).unwrap()
}
