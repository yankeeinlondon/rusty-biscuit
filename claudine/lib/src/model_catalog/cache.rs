//! Cache read/write for provider model listings.
//!
//! Cached listings live under `~/.claudine/cache/models/<provider>.json`.
//! Each entry stores the model list plus a fetch timestamp. Listings are
//! drift-channel input (diffed against the expected-offering baseline);
//! they do not feed model validation.

use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::provider::Provider;
/// A single cached catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCacheEntry {
    /// Provider this cache entry belongs to.
    pub provider: Provider,
    /// Normalized model identifiers.
    pub models: Vec<String>,
    /// When this cache entry was last successfully refreshed.
    pub fetched_at: DateTime<Utc>,
}

/// Disk-backed cache for provider model catalogs.
#[derive(Debug, Clone)]
pub struct ModelCache {
    cache_dir: PathBuf,
}

impl Default for ModelCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelCache {
    /// Create a new cache using the default `~/.claudine/cache/models` directory.
    pub fn new() -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claudine")
            .join("cache")
            .join("models");
        Self { cache_dir }
    }

    /// Create a cache with a custom directory (useful for tests).
    pub fn with_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Read a cached catalog for the given provider, if it exists.
    pub fn read(&self, provider: Provider) -> Option<ModelCacheEntry> {
        let path = self.cache_path(provider);
        if !path.exists() {
            return None;
        }
        let contents = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Write a catalog entry to the cache.
    pub fn write(&self, entry: &ModelCacheEntry) -> std::io::Result<()> {
        let path = self.cache_path(entry.provider);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp_path = path.with_extension("tmp");
        let contents = serde_json::to_string_pretty(entry)?;
        std::fs::write(&temp_path, contents)?;
        std::fs::rename(&temp_path, &path)?;
        Ok(())
    }

    /// Return the cache file path for a provider.
    fn cache_path(&self, provider: Provider) -> PathBuf {
        let filename = format!("{}.json", provider.as_slug());
        self.cache_dir.join(filename)
    }

    /// Return the set of model IDs from the cache, if any.
    pub fn model_set(&self, provider: Provider) -> Option<HashSet<String>> {
        self.read(provider)
            .map(|entry| entry.models.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = ModelCache::with_dir(tmp.path().to_path_buf());

        let entry = ModelCacheEntry {
            provider: Provider::Codex,
            models: vec!["gpt-4o".into(), "o3-mini".into()],
            fetched_at: Utc::now(),
        };

        cache.write(&entry).unwrap();
        let read_back = cache.read(Provider::Codex).unwrap();

        assert_eq!(read_back.provider, Provider::Codex);
        assert_eq!(read_back.models, vec!["gpt-4o", "o3-mini"]);
    }

    #[test]
    fn cache_miss_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = ModelCache::with_dir(tmp.path().to_path_buf());
        assert!(cache.read(Provider::Claude).is_none());
    }

    #[test]
    fn model_set_returns_hashset() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = ModelCache::with_dir(tmp.path().to_path_buf());

        let entry = ModelCacheEntry {
            provider: Provider::Claude,
            models: vec!["claude-sonnet-4".into(), "claude-opus-4".into()],
            fetched_at: Utc::now(),
        };
        cache.write(&entry).unwrap();

        let set = cache.model_set(Provider::Claude).unwrap();
        assert!(set.contains("claude-sonnet-4"));
        assert!(set.contains("claude-opus-4"));
    }
}
