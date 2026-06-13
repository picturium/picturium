use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_OFFICE_CONVERSION_TIMEOUT: &str = "30";

#[derive(Debug, Clone)]
pub struct OfficeConfig {
    pub conversion_timeout: u64,
}

impl ConfigFromEnv for OfficeConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            conversion_timeout: parse_env("OFFICE_CONVERSION_TIMEOUT", DEFAULT_OFFICE_CONVERSION_TIMEOUT)?,
        })
    }
}
