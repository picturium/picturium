use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self { allowed_origins: vec!["*".into()] }
    }
}

impl CorsConfig {
    pub fn is_permissive(&self) -> bool {
        self.allowed_origins.iter().any(|origin| origin == "*")
    }
}
