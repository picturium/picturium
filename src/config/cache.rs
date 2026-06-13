use anyhow::Result;
use crate::config::{parse_env, ConfigFromEnv};

const DEFAULT_CACHE_DIR: &str = "cache";

const DEFAULT_CACHE_MEMORY_ENABLED: &str = "true";
const DEFAULT_CACHE_MEMORY_CAPACITY: &str = "1000";
const DEFAULT_CACHE_MEMORY_ENTRY_LIMIT: &str = "2";

const DEFAULT_CACHE_DISK_ENABLED: &str = "true";
const DEFAULT_CACHE_DISK_LIMIT: &str = "1024";

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub dir: String,
    pub memory: MemoryCacheConfig,
    pub disk: DiskCacheConfig,
}

impl ConfigFromEnv for CacheConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            dir: parse_env("CACHE_DIR", DEFAULT_CACHE_DIR)?,
            memory: MemoryCacheConfig::from_env()?,
            disk: DiskCacheConfig::from_env()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MemoryCacheConfig {
    pub enabled: bool,
    pub capacity: usize,
    pub entry_limit: usize,
}

impl ConfigFromEnv for MemoryCacheConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            enabled: parse_env("CACHE_MEMORY_ENABLED", DEFAULT_CACHE_MEMORY_ENABLED)?,
            capacity: parse_env("CACHE_MEMORY_CAPACITY", DEFAULT_CACHE_MEMORY_CAPACITY)?,
            entry_limit: parse_env("CACHE_MEMORY_ENTRY_LIMIT", DEFAULT_CACHE_MEMORY_ENTRY_LIMIT)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DiskCacheConfig {
    pub enabled: bool,
    pub limit: usize,
}

impl ConfigFromEnv for DiskCacheConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            enabled: parse_env("CACHE_DISK_ENABLED", DEFAULT_CACHE_DISK_ENABLED)?,
            limit: parse_env("CACHE_DISK_LIMIT", DEFAULT_CACHE_DISK_LIMIT)?,
        })
    }
}