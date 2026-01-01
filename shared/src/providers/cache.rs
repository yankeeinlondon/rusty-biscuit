//! Caching for provider lists to reduce API calls

use super::types::LlmEntry;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::sync::Mutex;

/// Cache TTL: 24 hours
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Cached provider list with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedProviderList {
    /// When this cache entry was created
    timestamp: SystemTime,
    /// The cached provider entries
    entries: Vec<LlmEntry>,
}

/// Global lock to prevent concurrent API calls
static FETCH_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("System time error: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}

/// Get the cache file path
fn cache_path() -> Result<PathBuf, CacheError> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    let cache_dir = PathBuf::from(home).join(".cache").join("dockhand");
    std::fs::create_dir_all(&cache_dir)?;

    Ok(cache_dir.join("provider_list.json"))
}

/// Check if cached data exists and is still valid
pub fn check_cache() -> Result<Option<Vec<LlmEntry>>, CacheError> {
    let path = cache_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&path)?;
    let cached: CachedProviderList = serde_json::from_str(&contents)?;

    let now = SystemTime::now();
    let age = now.duration_since(cached.timestamp)?;

    if age < CACHE_TTL {
        tracing::debug!(
            "Cache hit: {} entries, age: {:?}",
            cached.entries.len(),
            age
        );
        Ok(Some(cached.entries))
    } else {
        tracing::debug!("Cache expired: age {:?} > TTL {:?}", age, CACHE_TTL);
        Ok(None)
    }
}

/// Write provider list to cache
pub fn write_cache(entries: &[LlmEntry]) -> Result<(), CacheError> {
    let path = cache_path()?;

    let cached = CachedProviderList {
        timestamp: SystemTime::now(),
        entries: entries.to_vec(),
    };

    let json = serde_json::to_string_pretty(&cached)?;
    std::fs::write(&path, json)?;

    tracing::debug!("Wrote {} entries to cache at {:?}", entries.len(), path);
    Ok(())
}

/// Invalidate the cache (delete the file)
pub fn invalidate_cache() -> Result<(), CacheError> {
    let path = cache_path()?;

    if path.exists() {
        std::fs::remove_file(&path)?;
        tracing::debug!("Cache invalidated at {:?}", path);
    }

    Ok(())
}

/// Acquire the global fetch lock (prevents concurrent API calls)
pub async fn acquire_fetch_lock() -> tokio::sync::MutexGuard<'static, ()> {
    FETCH_LOCK.lock().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_uses_home() {
        let path = cache_path().unwrap();
        assert!(path.to_string_lossy().contains(".cache"));
        assert!(path.to_string_lossy().contains("dockhand"));
        assert!(path.file_name().unwrap() == "provider_list.json");
    }

    #[test]
    #[serial_test::serial]
    fn check_cache_returns_none_when_missing() {
        // Ensure cache doesn't exist
        let _ = invalidate_cache();

        let result = check_cache().unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn write_and_read_cache() {
        // Clear cache first
        let _ = invalidate_cache();

        let entries = vec![
            LlmEntry::new("openai", "gpt-4"),
            LlmEntry::new("anthropic", "claude-opus"),
        ];

        write_cache(&entries).unwrap();
        let cached = check_cache().unwrap();

        assert!(cached.is_some());
        let cached_entries = cached.unwrap();
        assert_eq!(cached_entries.len(), 2);
        assert_eq!(cached_entries[0].provider, "openai");
        assert_eq!(cached_entries[1].model, "claude-opus");

        // Cleanup
        let _ = invalidate_cache();
    }

    #[test]
    #[serial_test::serial]
    fn invalidate_removes_cache() {
        // Clear cache first
        let _ = invalidate_cache();

        let entries = vec![LlmEntry::new("test", "model")];
        write_cache(&entries).unwrap();

        assert!(check_cache().unwrap().is_some());

        invalidate_cache().unwrap();

        assert!(check_cache().unwrap().is_none());
    }
}
