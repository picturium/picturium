use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VideoConfig {
    pub default_time: String,
    pub extraction_timeout: u64,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            default_time: "1".into(),
            extraction_timeout: 15,
        }
    }
}
