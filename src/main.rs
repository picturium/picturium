mod config;
mod health;
mod process;
mod enums;
mod params;
mod startup_logs;
mod state;
mod multithreading;
mod services;

use crate::health::health_check;
use crate::process::process_file;
use anyhow::Result;
use axum::{response::{Html, IntoResponse}, routing::get, Router};
use crate::config::{Config, SharedConfig};
use std::sync::Arc;
use crate::state::AppState;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::startup_logs::print_startup_logs;

async fn root() -> impl IntoResponse {
    Html(include_str!("../templates/index.html"))
}

fn setup_tracing(log_level: &str) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn configure_cors(config: &SharedConfig) -> CorsLayer {
    if config.cors.is_permissive() {
        return CorsLayer::permissive();
    }

    let origins = config
        .cors
        .allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(Any)
        .allow_headers(Any)
}

fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/{*file_path}", get(process_file))
        .nest_service("/public", ServeDir::new("public"))
        .layer(TraceLayer::new_for_http())
        .layer(configure_cors(&state.config))
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(Config::load()?);
    setup_tracing(&config.server.log_level);

    let state = AppState::new(config.clone()).await?;

    print_startup_logs(&config, &state);

    let app = create_app(state);
    let listener = tokio::net::TcpListener::bind(&config.server.get_address()).await?;

    info!("Server listening on {}", config.server.get_address());

    axum::serve(listener, app).await?;
    Ok(())
}
