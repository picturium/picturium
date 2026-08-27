use crate::config::Config;
use anyhow::{Context, Result, anyhow, ensure};
use bytes::Bytes;
use foyer::{
    BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCache, HybridCachePolicy,
    HybridCacheProperties, Location,
};
use fs4::FileExt;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
use tracing::warn;

const MIB: usize = 1024 * 1024;
const MAX_BLOCK_SIZE: usize = 16 * MIB;
const BLOB_INDEX_SIZE: usize = 4 * 1024;
const STORE_VERSION: &str = "foyer-v1";

#[derive(Clone)]
pub struct CacheStore {
    inner: Option<HybridCache<String, Bytes>>,
    memory_enabled: bool,
    disk_enabled: bool,
    memory_entry_limit: usize,
    disk_entry_limit: usize,
    _owner_lock: Option<Arc<File>>,
}

impl fmt::Debug for CacheStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CacheStore")
            .field("memory_enabled", &self.memory_enabled)
            .field("disk_enabled", &self.disk_enabled)
            .field("memory_entry_limit", &self.memory_entry_limit)
            .field("disk_entry_limit", &self.disk_entry_limit)
            .finish()
    }
}

impl CacheStore {
    pub async fn new(config: &Config) -> Result<Self> {
        let memory = &config.cache.memory;
        let disk = &config.cache.disk;
        let memory_entry_limit = if memory.enabled {
            to_bytes(memory.entry_limit, "cache.memory.entry_limit")?
        } else {
            0
        };

        if !memory.enabled && !disk.enabled {
            return Ok(Self {
                inner: None,
                memory_enabled: false,
                disk_enabled: false,
                memory_entry_limit,
                disk_entry_limit: 0,
                _owner_lock: None,
            });
        }

        let memory_capacity = if memory.enabled { memory.capacity } else { 1 };
        let memory_enabled = memory.enabled;
        
        let mut builder = HybridCache::<String, Bytes>::builder()
            .with_name("picturium")
            .with_policy(HybridCachePolicy::WriteOnEviction)
            .with_flush_on_close(false)
            .memory(memory_capacity)
            .with_weighter(|_, _| 1)
            .with_filter(move |_, value: &Bytes| {
                memory_enabled && value.len() <= memory_entry_limit
            })
            .storage();

        let (owner_lock, disk_entry_limit) = if disk.enabled {
            let disk_capacity = to_bytes(disk.limit, "cache.disk.limit")?;
            let cache_dir = PathBuf::from(&config.cache.dir);
            let owner_lock = acquire_owner_lock(cache_dir.clone()).await?;
            
            remove_legacy_cache(&cache_dir).await?;

            let block_size = disk_capacity.min(MAX_BLOCK_SIZE);
            
            let device = FsDeviceBuilder::new(cache_dir.join(STORE_VERSION))
                .with_capacity(disk_capacity)
                .build()
                .context("failed to create disk cache device")?;
            
            let engine = BlockEngineConfig::new(device)
                .with_block_size(block_size)
                .with_buffer_pool_size(block_size)
                .with_submit_queue_size_threshold(block_size.saturating_mul(2));

            builder = builder.with_engine_config(engine);
            
            (Some(Arc::new(owner_lock)), block_size.saturating_sub(BLOB_INDEX_SIZE))
        } else {
            (None, 0)
        };

        let inner = builder.build().await.context("failed to initialize cache")?;

        Ok(Self {
            inner: Some(inner),
            memory_enabled: memory.enabled,
            disk_enabled: disk.enabled,
            memory_entry_limit,
            disk_entry_limit,
            _owner_lock: owner_lock,
        })
    }

    pub async fn get(&self, key: &str) -> Option<Bytes> {
        let cache = self.inner.as_ref()?;

        match cache.get(key).await {
            Ok(Some(entry)) => Some(entry.value().clone()),
            Ok(None) => None,
            Err(error) => {
                warn!(%error, "cache read failed; treating it as a miss");
                None
            }
        }
    }

    pub fn insert(&self, key: String, value: Bytes) {
        let Some(cache) = &self.inner else {
            return;
        };

        let properties = self.properties(value.len());
        let _ = cache.insert_with_properties(key, value, properties);
    }

    pub async fn get_or_insert_with<F, Fut>(&self, key: String, fetch: F) -> Result<Bytes>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Bytes>> + Send + 'static,
    {
        let Some(cache) = &self.inner else {
            return fetch().await;
        };

        let fetch = Arc::new(Mutex::new(Some(fetch)));
        let fetched = Arc::new(Mutex::new(None::<Bytes>));
        let fetch_for_cache = Arc::clone(&fetch);
        let fetched_for_cache = Arc::clone(&fetched);
        let limits = self.clone();

        let result = cache.get_or_fetch(&key, move || async move {
                let fetch = fetch_for_cache.lock().map_err(|_| anyhow!("cache fetch lock poisoned"))?.take().context("cache fetch already consumed")?;
                let value = fetch().await?;

                *fetched_for_cache.lock().map_err(|_| anyhow!("cache result lock poisoned"))? = Some(value.clone());

                let properties = limits.properties(value.len());
                Ok::<_, anyhow::Error>((value, properties))
            }).await;

        match result {
            Ok(entry) => Ok(entry.value().clone()),
            Err(error) => {
                warn!(%error, "cache fetch failed; falling back to uncached work");

                if let Some(value) = fetched.lock().map_err(|_| anyhow!("cache result lock poisoned"))?.take() {
                    return Ok(value);
                }

                let fetch = fetch.lock().map_err(|_| anyhow!("cache fetch lock poisoned"))?.take();

                match fetch {
                    Some(fetch) => fetch().await,
                    None => Err(error.into()),
                }
            }
        }
    }

    /// `forced` re-renders and overwrites the entry instead of reusing it.
    pub async fn resolve<F, Fut>(&self, key: String, forced: bool, fetch: F) -> Result<Bytes> 
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Bytes>> + Send + 'static,
    {
        if !forced {
            return self.get_or_insert_with(key, fetch).await;
        }

        let value = fetch().await?;
        self.insert(key, value.clone());

        Ok(value)
    }

    fn properties(&self, size: usize) -> HybridCacheProperties {
        let location = if self.memory_enabled && size <= self.memory_entry_limit {
            if self.disk_enabled && size > self.disk_entry_limit {
                Location::InMem
            } else {
                Location::Default
            }
        } else if self.disk_enabled && size <= self.disk_entry_limit {
            Location::OnDisk
        } else {
            Location::InMem
        };

        HybridCacheProperties::default().with_location(location)
    }
}

pub fn key(namespace: &str, seed: &str, source_path: &Path, metadata: &std::fs::Metadata, variant: &str) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());

    let mut hasher = Sha256::new();
    hasher.update(b"picturium-cache-v1\0");
    hasher.update(namespace.as_bytes());
    hasher.update(b"\0");
    hasher.update(seed.as_bytes());
    hasher.update(b"\0");
    hasher.update(source_path.to_string_lossy().as_bytes());
    hasher.update(b"\0");
    hasher.update(modified.to_le_bytes());
    hasher.update(metadata.len().to_le_bytes());
    hasher.update(variant.as_bytes());

    format!("{namespace}:{}", hex::encode(hasher.finalize()))
}

pub async fn source_key(namespace: &str, seed: &str, source_path: &Path, variant: &str) -> Result<String> {
    let metadata = tokio::fs::metadata(source_path).await
        .with_context(|| format!("failed to read metadata for {}", source_path.display()))?;

    Ok(key(namespace, seed, source_path, &metadata, variant))
}

async fn acquire_owner_lock(cache_dir: PathBuf) -> Result<File> {
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&cache_dir).with_context(|| format!("failed to create cache directory {}", cache_dir.display()))?;
        
        let lock_path = cache_dir.join(".picturium-cache.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open cache owner lock {}", lock_path.display()))?;

        FileExt::try_lock(&file).with_context(|| {
            format!(
                "cache directory {} is already owned by another Picturium process",
                cache_dir.display()
            )
        })?;

        Ok(file)
    })
    .await
    .context("cache owner lock task failed")?
}

async fn remove_legacy_cache(cache_dir: &Path) -> Result<()> {
    let legacy = cache_dir.join("intermediate");

    match tokio::fs::remove_dir_all(&legacy).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove legacy cache {}", legacy.display())),
    }
}

fn to_bytes(mebibytes: usize, name: &str) -> Result<usize> {
    ensure!(mebibytes > 0, "{name} must be greater than zero when enabled");
    mebibytes
        .checked_mul(MIB)
        .with_context(|| format!("{name} is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn config(root: &Path, memory: bool, disk: bool) -> Config {
        let mut config = Config::default();
        config.cache.dir = root.to_string_lossy().into_owned();
        config.cache.memory.enabled = memory;
        config.cache.memory.capacity = 2;
        config.cache.memory.entry_limit = 1;
        config.cache.disk.enabled = disk;
        config.cache.disk.limit = 1;
        config
    }

    #[tokio::test]
    async fn disabled_cache_runs_the_fetch_every_time() {
        let root = tempfile::tempdir().unwrap();
        let cache = CacheStore::new(&config(root.path(), false, false)).await.unwrap();
        let calls = Arc::new(AtomicUsize::new(0));

        for _ in 0..2 {
            let calls = Arc::clone(&calls);
            cache
                .get_or_insert_with("key".into(), move || async move {
                    calls.fetch_add(1, Ordering::Relaxed);
                    Ok(Bytes::from_static(b"value"))
                })
                .await
                .unwrap();
        }

        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn concurrent_fetches_are_coalesced() {
        let root = tempfile::tempdir().unwrap();
        let cache = CacheStore::new(&config(root.path(), true, false)).await.unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let first = {
            let cache = cache.clone();
            let calls = Arc::clone(&calls);
            tokio::spawn(async move {
                cache
                    .get_or_insert_with("key".into(), move || async move {
                        calls.fetch_add(1, Ordering::Relaxed);
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        Ok(Bytes::from_static(b"value"))
                    })
                    .await
            })
        };
        tokio::task::yield_now().await;
        let second = {
            let cache = cache.clone();
            let calls = Arc::clone(&calls);
            tokio::spawn(async move {
                cache
                    .get_or_insert_with("key".into(), move || async move {
                        calls.fetch_add(1, Ordering::Relaxed);
                        Ok(Bytes::from_static(b"other"))
                    })
                    .await
            })
        };

        assert_eq!(first.await.unwrap().unwrap(), Bytes::from_static(b"value"));
        assert_eq!(second.await.unwrap().unwrap(), Bytes::from_static(b"value"));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn disk_only_cache_does_not_retain_values_in_memory() {
        let root = tempfile::tempdir().unwrap();
        let cache = CacheStore::new(&config(root.path(), false, true)).await.unwrap();
        let value = cache
            .get_or_insert_with("key".into(), || async {
                Ok(Bytes::from_static(b"disk value"))
            })
            .await
            .unwrap();

        assert_eq!(value, Bytes::from_static(b"disk value"));
        assert!(cache.inner.as_ref().unwrap().memory().get("key").is_none());
        assert_eq!(cache.get("key").await, Some(Bytes::from_static(b"disk value")));
    }

    #[tokio::test]
    async fn oversized_memory_only_values_are_not_retained() {
        let root = tempfile::tempdir().unwrap();
        let cache = CacheStore::new(&config(root.path(), true, false)).await.unwrap();
        let value = Bytes::from(vec![0; MIB + 1]);

        cache.insert("large".into(), value);

        assert!(cache.get("large").await.is_none());
    }

    #[tokio::test]
    async fn a_forced_resolve_replaces_the_cached_value() {
        let root = tempfile::tempdir().unwrap();
        let cache = CacheStore::new(&config(root.path(), true, false)).await.unwrap();

        cache
            .resolve("key".into(), false, || async { Ok(Bytes::from_static(b"stale")) })
            .await
            .unwrap();
        let fresh = cache
            .resolve("key".into(), true, || async { Ok(Bytes::from_static(b"fresh")) })
            .await
            .unwrap();

        assert_eq!(fresh, Bytes::from_static(b"fresh"));
        assert_eq!(cache.get("key").await, Some(Bytes::from_static(b"fresh")));
    }

    #[test]
    fn key_changes_with_source_identity_and_variant() {
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"one").unwrap();
        let metadata = source.as_file().metadata().unwrap();
        let first = key("response", "seed", source.path(), &metadata, "a");

        assert_eq!(first, key("response", "seed", source.path(), &metadata, "a"));
        assert_ne!(first, key("response", "seed", source.path(), &metadata, "b"));

        let other = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(other.path(), b"one").unwrap();
        assert_ne!(
            first,
            key(
                "response",
                "seed",
                other.path(),
                &other.as_file().metadata().unwrap(),
                "a"
            )
        );
    }
}
