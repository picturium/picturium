use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_LOG_LEVEL: &str = "debug";
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "20045";
const DEFAULT_WORKERS: &str = "0";
const DEFAULT_QUEUE_SIZE: &str = "100";


#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub log_level: String,
    pub host: String,
    pub port: String,
    pub workers: usize,
    pub queue_size: usize,
}

impl ConfigFromEnv for ServerConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            log_level: parse_env("LOG_LEVEL", DEFAULT_LOG_LEVEL)?,
            host: parse_env("HOST", DEFAULT_HOST)?,
            port: parse_env("PORT", DEFAULT_PORT)?,
            workers: parse_env::<usize>("WORKERS", DEFAULT_WORKERS)?,
            queue_size: parse_env::<usize>("QUEUE_SIZE", DEFAULT_QUEUE_SIZE)?,
        })
    }
}

impl ServerConfig {
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}