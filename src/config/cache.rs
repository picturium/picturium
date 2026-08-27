use anyhow::{Result, ensure};
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

impl CacheConfig {
    pub fn validate(&self) -> Result<()> {
        if self.memory.enabled {
            ensure!(
                self.memory.capacity > 0,
                "cache.memory.capacity must be greater than zero when enabled"
            );
            ensure!(
                self.memory.entry_limit > 0,
                "cache.memory.entry_limit must be greater than zero when enabled"
            );
        }

        if self.disk.enabled {
            ensure!(
                self.disk.limit > 0,
                "cache.disk.limit must be greater than zero when enabled"
            );
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_tiers_require_positive_capacities() {
        let mut config = CacheConfig::default();
        config.memory.capacity = 0;
        assert!(config.validate().is_err());

        config.memory.enabled = false;
        config.disk.limit = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn disabled_tiers_ignore_zero_capacities() {
        let config = CacheConfig {
            memory: MemoryCacheConfig {
                enabled: false,
                capacity: 0,
                entry_limit: 0,
            },
            disk: DiskCacheConfig {
                enabled: false,
                limit: 0,
            },
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }
}
