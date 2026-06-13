pub mod source;
pub mod pipeline;

use axum::body::Body;
use crate::state::AppState;
use crate::params::RequestParams;
use crate::process::source::Source;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::Response;
use tracing::debug;
use crate::enums::output_format::get_output_mime;
use crate::params::parsed::Parameters;
use pipeline::request::PipelineRequest;
use crate::services::signature::verify_signature;

pub async fn process_file(
    headers: HeaderMap,
    uri: Uri,
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    Query(params): Query<RequestParams>,
) -> Response {
    // Multithreading
    let _permit = match state.multithreading.get_permit().await {
        Some(permit) => permit,
        None => return Response::builder().status(StatusCode::SERVICE_UNAVAILABLE).body(Body::from("Too many requests")).unwrap(),
    };

    let _guard = scopeguard::guard(state.multithreading.clone(), |mt| mt.release_worker());

    // Signature verification
    if !verify_signature(&state.config, &uri) {
        return Response::builder().status(StatusCode::FORBIDDEN).body(Body::from("Invalid signature")).unwrap();
    }

    // Source file resolution
    let source = match Source::new(&state.config, &file_path, &params) {
        Ok(source) => source,
        Err(e) => return Response::builder().status(StatusCode::NOT_FOUND).body(Body::from(format!("File not found: {e}"))).unwrap(),
    };

    let parameters = Parameters::new(&state.config, params);

    // TODO: Check cache for existing processed file

    let pipeline_request = PipelineRequest::new(&headers, &state, &source, &parameters);

    debug!("Output format: {:?}, through: {:?}", pipeline_request.output_format, pipeline_request.intermediate_format);

    // Async pre-pipeline: office/video → intermediate file
    let source_path = match pipeline::resolve_source_path(&pipeline_request).await {
        Ok(path) => path,
        Err(e) => {
            tracing::error!("Error in pre-pipeline: {e}");
            return Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("Error processing file")).unwrap();
        }
    };

    // Blocking vips pipeline: offload to blocking thread pool
    let result = match tokio::task::block_in_place(|| pipeline::process_image(&pipeline_request, &source_path)) {
        Ok(result) => Body::from(result),
        Err(e) => {
            tracing::error!("Error processing file: {e}");
            return Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("Error processing file")).unwrap();
        }
    };

    // TODO: Store result in cache

    create_response(&pipeline_request, result)
}

fn create_response(pipeline_request: &PipelineRequest, result: Body) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", get_output_mime(&pipeline_request.output_format))
        .body(result)
        .unwrap()
}
