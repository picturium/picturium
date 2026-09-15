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
                self.memory.limit > 0,
                "cache.memory.limit must be greater than zero when enabled"
            );

            ensure!(
                self.memory.entry_limit > 0,
                "cache.memory.entry_limit must be greater than zero when enabled"
            );

            ensure!(
                self.memory.entry_limit <= self.memory.limit,
                "cache.memory.entry_limit must not exceed cache.memory.limit"
            );
        }

        if self.disk.enabled {
            ensure!(
                self.disk.limit > 0,
                "cache.disk.limit must be greater than zero when enabled"
            );

            ensure!(
                self.disk.entry_limit > 0,
                "cache.disk.entry_limit must be greater than zero when enabled"
            );

            ensure!(
                self.disk.entry_limit <= self.disk.limit,
                "cache.disk.entry_limit must not exceed cache.disk.limit"
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MemoryCacheConfig {
    pub enabled: bool,
    pub limit: usize,
    pub entry_limit: usize,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            limit: 256,
            entry_limit: 2
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiskCacheConfig {
    pub enabled: bool,
    pub limit: usize,
    pub entry_limit: usize,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            limit: 1024,
            entry_limit: 16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_tiers_require_positive_capacities() {
        let mut config = CacheConfig::default();
        config.memory.limit = 0;
        assert!(config.validate().is_err());

        config.memory.enabled = false;
        config.disk.limit = 0;
        assert!(config.validate().is_err());

        config.disk.limit = 16;
        config.disk.entry_limit = 32;
        assert!(config.validate().is_err());
    }

    #[test]
    fn disabled_tiers_ignore_zero_capacities() {
        let config = CacheConfig {
            memory: MemoryCacheConfig {
                enabled: false,
                limit: 0,
                entry_limit: 0,
            },
            disk: DiskCacheConfig {
                enabled: false,
                limit: 0,
                entry_limit: 0,
            },
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }
}
