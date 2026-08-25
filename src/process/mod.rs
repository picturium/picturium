mod document;
mod download;
pub mod pipeline;
mod raw;
pub mod source;

use crate::enums::download::Download;
use crate::enums::input::InputFormat;
use crate::enums::output_format::{OutputFormat, get_output_extension, get_output_mime};
use crate::params::RequestParams;
use crate::params::parsed::Parameters;
use crate::process::source::Source;
use crate::services::signature::verify_signature;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, Uri};
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
    let _permit = match state.multithreading.get_permit().await {
        Some(permit) => permit,
        None => {
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::from("Too many requests"))
                .unwrap();
        }
    };

    let _guard = scopeguard::guard(state.multithreading.clone(), |mt| mt.release_worker());

    if !verify_signature(&state.config, &uri) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
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
                .body(Body::from(include_str!("../../templates/404.html")))
                .unwrap();
        }
    };

    let parameters = Parameters::new(&state.config, params);

    if parameters.original == true {
        return serve_raw(&headers, &source.path, &parameters.download).await;
    }

    if matches!(source.format, InputFormat::Unsupported | InputFormat::Video(_)) {
        if !state.config.data.may_serve(&source.path) {
            debug!("Refusing to serve unprocessable file {}", source.path.display());
            return unsupported_source(&source.path);
        }

        return serve_raw(&headers, &source.path, &parameters.download).await;
    }

    let mut pipeline_request = PipelineRequest::new(&headers, &state, &mut source, &parameters);

    if matches!(pipeline_request.output_format, OutputFormat::Pdf | OutputFormat::Svg) {
        return document::serve(&headers, &pipeline_request).await.unwrap_or_else(|e| {
            tracing::error!("Error serving document: {e}");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Error serving document"))
                .unwrap()
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
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Error processing file"))
                .unwrap();
        }
    };

    let result = match tokio::task::block_in_place(|| {
        pipeline::process_image(&mut pipeline_request, &source_path)
    }) {
        Ok(result) => Body::from(result),
        Err(e) => {
            tracing::error!("Error processing file: {e}");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Error processing file"))
                .unwrap();
        }
    };

    create_response(&pipeline_request, result)
}

async fn serve_raw(headers: &HeaderMap, path: &std::path::Path, download: &Download) -> Response {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");

    match raw::serve(headers, path, download, name).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Error serving {}: {e}", path.display());
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Error serving file"))
                .unwrap()
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
        .body(Body::from(format!(
            "Unsupported source file type \"{extension}\""
        )))
        .unwrap()
}

fn create_response(pipeline_request: &PipelineRequest, result: Body) -> Response {
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

    if let Some(name) = fallback_name {
        builder = apply_disposition(builder, &pipeline_request.parameters.download, &name);
    }

    builder.body(result).unwrap()
}
