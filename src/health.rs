use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    name: &'static str,
    version: String,
    status: String,
    total_workers: usize,
    available_workers: usize,
    queue_size: usize,
    available_queue_size: usize,
}

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let mut status = "healthy".to_string();

    let total_workers = state.multithreading.total_workers;
    let available_workers = state.multithreading.get_available_workers();

    let queue_size = state.config.server.queue_size;
    let available_queue_size = state.multithreading.get_available_queue_size();

    if (available_queue_size as f64 / total_workers as f64) < 0.1 {
        status = "warning".to_string();
    }

    if available_workers == 0 && available_queue_size == 0 {
        status = "unhealthy".to_string();
    }

    Json(HealthResponse {
        name: "picturium",
        version: env!("CARGO_PKG_VERSION").to_string(),
        status,
        total_workers,
        available_workers,
        queue_size,
        available_queue_size,
    })
}