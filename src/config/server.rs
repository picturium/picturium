use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub log_level: String,
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub queue_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            log_level: "debug".into(),
            host: "127.0.0.1".into(),
            port: 20045,
            workers: 0,
            queue_size: 100,
        }
    }
}

impl ServerConfig {
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
