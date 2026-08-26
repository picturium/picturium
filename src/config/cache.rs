use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CacheConfig {
    pub dir: String,
    pub cache_control: String,
    pub memory: MemoryCacheConfig,
    pub disk: DiskCacheConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: "cache".into(),
            cache_control: "public, max-age=604800, must-revalidate".into(),
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MemoryCacheConfig {
    pub enabled: bool,
    pub capacity: usize,
    pub entry_limit: usize,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self { enabled: true, capacity: 1000, entry_limit: 2 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiskCacheConfig {
    pub enabled: bool,
    pub limit: usize,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        Self { enabled: true, limit: 1024 }
    }
}
