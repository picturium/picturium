use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_SVG_UNLIMITED: &str = "false";
const DEFAULT_SVG_LOAD_DPI: &str = "72";

#[derive(Debug, Clone)]
pub struct SvgConfig {
    pub unlimited: bool,
    pub load_dpi: u32,
}

impl ConfigFromEnv for SvgConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            unlimited: parse_env("SVG_UNLIMITED", DEFAULT_SVG_UNLIMITED)?,
            load_dpi: parse_env("SVG_LOAD_DPI", DEFAULT_SVG_LOAD_DPI)?,
        })
    }
}