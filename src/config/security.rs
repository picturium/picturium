use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_SIGNATURE_ENABLED: &str = "false";
const DEFAULT_SIGNATURE_SECRET: &str = "";

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub signature_enabled: bool,
    pub signature_secret: String,
}

impl ConfigFromEnv for SecurityConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            signature_enabled: parse_env("SIGNATURE_ENABLED", DEFAULT_SIGNATURE_ENABLED)?,
            signature_secret: parse_env("SIGNATURE_SECRET", DEFAULT_SIGNATURE_SECRET)?,
        })
    }
}
