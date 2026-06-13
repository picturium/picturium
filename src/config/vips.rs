use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_VIPS_CONCURRENCY: &str = "1";

#[derive(Debug, Clone)]
pub struct VipsConfig {
    pub debug: bool,
    pub concurrency: i32,
}

impl ConfigFromEnv for VipsConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            debug: parse_env("VIPS_DEBUG", "false")?,
            concurrency: parse_env("VIPS_CONCURRENCY", DEFAULT_VIPS_CONCURRENCY)?,
        })
    }
}