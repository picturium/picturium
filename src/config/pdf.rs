use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_PDF_LOAD_DPI: &str = "72";

#[derive(Debug, Clone)]
pub struct PdfConfig {
    pub load_dpi: u32,
}

impl ConfigFromEnv for PdfConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            load_dpi: parse_env("PDF_LOAD_DPI", DEFAULT_PDF_LOAD_DPI)?,
        })
    }
}
