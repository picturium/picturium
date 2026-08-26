use std::sync::Arc;
use picturium_libvips::{Cache, Vips};
use tracing::error;
use crate::config::SharedConfig;
use crate::multithreading::MultiThreading;
use crate::services::http_cache;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: SharedConfig,
    pub multithreading: MultiThreading,
    pub etag_seed: Arc<str>,
    _vips: Arc<Vips>,
}

impl AppState {
    pub fn new(config: SharedConfig) -> Self {
        let multithreading = MultiThreading::new(&config);
        let etag_seed = http_cache::seed(&config).into();
        let _vips = Arc::new(init_vips(&config));

        Self { config, multithreading, etag_seed, _vips }
    }
}

fn init_vips(config: &SharedConfig) -> Vips {
    let app = match Vips::new("picturium") {
        Ok(vips) => vips,
        Err(e) => {
            error!("Failed to initialize libvips: {e}");
            std::process::exit(1);
        },
    };

    app.concurrency(config.vips.concurrency);
    app.cache(Cache::default());

    if config.vips.debug {
        app.check_leaks();
    }

    app
}
