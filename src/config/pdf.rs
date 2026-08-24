use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PdfConfig {
    pub load_dpi: u32,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self { load_dpi: 72 }
    }
}
