use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SvgConfig {
    pub unlimited: bool,
    pub load_dpi: u32,
    pub allow_local_resources: bool,
}

impl Default for SvgConfig {
    fn default() -> Self {
        Self { unlimited: false, load_dpi: 72, allow_local_resources: false }
    }
}
