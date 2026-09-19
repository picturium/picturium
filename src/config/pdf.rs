use crate::params::background::Background;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PdfConfig {
    pub load_dpi: u32,
    pub background: String,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            load_dpi: 72,
            background: "white".into(),
        }
    }
}

impl PdfConfig {
    pub(super) fn validate(&self) -> Result<()> {
        if self.background.parse::<Background>().is_err() {
            bail!("pdf.background is not a valid color: {}", self.background);
        }

        Ok(())
    }
}
