use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_CORS_ALLOWED_ORIGINS: &str = "*";

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: String,
}

impl ConfigFromEnv for CorsConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            allowed_origins: parse_env("CORS_ALLOWED_ORIGINS", DEFAULT_CORS_ALLOWED_ORIGINS)?,
        })
    }
}
