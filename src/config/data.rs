use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_DATA_DIR: &str = "data";
const DEFAULT_DATA_SERVE_ALL: &str = "false";

#[derive(Debug, Clone)]
pub struct DataConfig {
    pub dir: String,
    pub serve_all: bool,
}

impl ConfigFromEnv for DataConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            dir: parse_env("DATA_DIR", DEFAULT_DATA_DIR)?,
            serve_all: parse_env("DATA_SERVE_ALL", DEFAULT_DATA_SERVE_ALL)?,
        })
    }
}