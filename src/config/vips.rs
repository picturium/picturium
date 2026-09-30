use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VipsConfig {
    pub debug: bool,
    pub concurrency: i32,
    pub cache_max_mem: usize,
    pub cache_max_files: i32,
    pub cache_max_ops: i32,
}

impl Default for VipsConfig {
    fn default() -> Self {
        Self {
            debug: false,
            concurrency: 1,
            cache_max_mem: 100,
            cache_max_files: 100,
            cache_max_ops: 100,
        }
    }
}
