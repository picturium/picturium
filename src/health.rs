use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::Json;
use picturium_libvips::Vips;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::services::http_cache;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct HealthQuery {
    token: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    #[serde(flatten)]
    details: Option<Details>,
}

#[derive(Serialize)]
struct Details {
    name: &'static str,
    version: &'static str,
    total_workers: usize,
    available_workers: usize,
    queue_size: usize,
    available_queue_size: usize,
    memory: MemoryStats,
}

#[derive(Serialize)]
struct MemoryStats {
    rss_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
    vips_bytes: usize,
    vips_peak_bytes: usize,
    vips_allocations: i32,
    vips_open_files: i32,
    vips_cached_operations: i32,
    cache_bytes: Option<usize>,
    cache_capacity_bytes: Option<usize>,
}

pub async fn health_check(headers: HeaderMap, Query(query): Query<HealthQuery>, State(state): State<AppState>) -> Response {
    let provided = match headers.get(header::AUTHORIZATION) {
        Some(value) => Some(value.to_str().ok().and_then(|value| value.strip_prefix("Bearer "))),
        None => query.token.as_deref().map(Some),
    };

    let detailed = match provided {
        None => false,
        Some(token) => {
            let authorized = token.is_some_and(|token| is_valid_token(&state.config.security.health_token, token));

            if !authorized {
                return (StatusCode::UNAUTHORIZED, [(header::CACHE_CONTROL, http_cache::NO_STORE)], "Invalid health token").into_response();
            }

            true
        }
    };

    let total_workers = state.multithreading.total_workers;
    let available_workers = state.multithreading.get_available_workers();

    let queue_size = state.config.server.queue_size;
    let available_queue_size = state.multithreading.get_available_queue_size();

    let status = if available_workers == 0 && available_queue_size == 0 {
        "unhealthy"
    } else if (available_queue_size as f64 / total_workers as f64) < 0.1 {
        "warning"
    } else {
        "healthy"
    };

    let details = detailed.then(|| Details {
        name: "picturium",
        version: env!("CARGO_PKG_VERSION"),
        total_workers,
        available_workers,
        queue_size,
        available_queue_size,
        memory: memory_stats(&state),
    });

    let response = Json(HealthResponse { status, details });

    ([(header::CACHE_CONTROL, http_cache::NO_STORE)], response).into_response()
}

/// An empty configured token disables the detailed view
fn is_valid_token(configured: &str, provided: &str) -> bool {
    !configured.is_empty() && Sha256::digest(configured) == Sha256::digest(provided)
}

fn memory_stats(state: &AppState) -> MemoryStats {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let cache = state.cache.memory_usage();

    MemoryStats {
        rss_bytes: proc_status_bytes(&status, "VmRSS:"),
        peak_rss_bytes: proc_status_bytes(&status, "VmHWM:"),
        vips_bytes: Vips::get_memory_usage(),
        vips_peak_bytes: Vips::get_peak_memory_usage(),
        vips_allocations: Vips::get_active_allocations(),
        vips_open_files: Vips::get_open_files(),
        vips_cached_operations: Vips::get_cache_size(),
        cache_bytes: cache.map(|(usage, _)| usage),
        cache_capacity_bytes: cache.map(|(_, capacity)| capacity),
    }
}

/// Reads a `Key:    1234 kB` line of `/proc/self/status` as bytes.
fn proc_status_bytes(status: &str, key: &str) -> Option<u64> {
    status.lines()
        .find_map(|line| line.strip_prefix(key))
        .and_then(|value| value.trim().strip_suffix("kB"))
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|kilobytes| kilobytes * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_must_be_configured_and_match() {
        assert!(is_valid_token("secret", "secret"));
        assert!(!is_valid_token("secret", "secre"));
        assert!(!is_valid_token("", ""));
    }

    #[test]
    fn parses_proc_status_sizes() {
        let status = "Name:\tpicturium\nVmHWM:\t  2048 kB\nVmRSS:\t  1024 kB\n";
        assert_eq!(proc_status_bytes(status, "VmRSS:"), Some(1024 * 1024));
        assert_eq!(proc_status_bytes(status, "VmHWM:"), Some(2048 * 1024));
        assert_eq!(proc_status_bytes(status, "VmSwap:"), None);
    }
}
