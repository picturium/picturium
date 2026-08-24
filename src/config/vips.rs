use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VipsConfig {
    pub debug: bool,
    pub concurrency: i32,
}

impl Default for VipsConfig {
    fn default() -> Self {
        Self { debug: false, concurrency: 1 }
    }
}
