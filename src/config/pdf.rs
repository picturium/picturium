use crate::params::background::Background;
use crate::params::pages::MAX_PAGE_SELECTION;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PdfConfig {
    pub load_dpi: u32,
    pub background: String,
    pub max_file_size: u64,
    pub max_pages: usize,
    pub max_image_pixels: u64,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            load_dpi: 72,
            background: "white".into(),
            max_file_size: 256,
            max_pages: 512,
            max_image_pixels: 64,
        }
    }
}

impl PdfConfig {
    pub(super) fn validate(&self) -> Result<()> {
        if self.background.parse::<Background>().is_err() {
            bail!("pdf.background is not a valid color: {}", self.background);
        }

        ensure!(
            (1..=MAX_PAGE_SELECTION).contains(&self.max_pages),
            "pdf.max_pages must be between 1 and {MAX_PAGE_SELECTION}"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_pages_must_fit_the_parse_ceiling() {
        assert!(PdfConfig::default().validate().is_ok());
        assert!(PdfConfig { max_pages: 0, ..Default::default() }.validate().is_err());
        assert!(PdfConfig { max_pages: MAX_PAGE_SELECTION + 1, ..Default::default() }.validate().is_err());
    }
}
