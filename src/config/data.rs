use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataConfig {
    pub dir: String,
    pub serve_all: bool,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self { dir: "data".into(), serve_all: false }
    }
}
