use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OfficeConfig {
    pub conversion_timeout: u64,
}

impl Default for OfficeConfig {
    fn default() -> Self {
        Self { conversion_timeout: 30 }
    }
}
