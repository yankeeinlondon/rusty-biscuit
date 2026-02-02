//! Caching for Mermaid diagram renders.
//!
//! This module provides a file-based cache for mermaid diagram renders
//! using xxHash for fast, collision-resistant cache key generation.
//!
//! ## Cache Key Components
//!
//! The cache key is generated from ALL parameters that affect the rendered output:
//!
//! 1. **instructions** - The mermaid diagram source code
//! 2. **theme** - The mermaid theme (dark, default, forest, neutral)
//! 3. **scale** - The render scale factor
//! 4. **config** - The JSON configuration (quadrant fills, font sizes, etc.)
//! 5. **transparent_background** - Whether background is transparent
//! 6. **title** - Optional diagram title
//! 7. **mmdc_version** - The mmdc CLI version (different versions may render differently)
//!
//! This ensures that any change to render parameters produces a different cache key,
//! avoiding stale or incorrect cached renders.
//!
//! ## Cache Location
//!
//! Cached renders are stored in the OS-appropriate temp directory:
//! - macOS: `/var/folders/.../mermaid-cache/`
//! - Linux: `/tmp/mermaid-cache/`
//!
//! Cache cleanup is left to the OS auto-cleanup mechanism.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::mermaid_cache::{MermaidCacheKey, MermaidCache};
//! use biscuit_terminal::components::mermaid::{MermaidTheme, MermaidConfig};
//! use std::path::Path;
//!
//! let key = MermaidCacheKey::new(
//!     "flowchart LR\n    A --> B",
//!     MermaidTheme::Dark,
//!     2,
//!     &MermaidConfig::new(),
//!     false,
//!     None,
//!     "10.9.1",
//! );
//!
//! let cache = MermaidCache::new();
//!
//! // Check for cached render
//! if let Some(path) = cache.get(&key) {
//!     println!("Cache hit: {}", path.display());
//! }
//!
//! // Store a new render (would be a real path in practice)
//! let rendered_path = Path::new("/tmp/diagram.png");
//! cache.store(&key, rendered_path).ok();
//! ```

use std::path::{Path, PathBuf};

use biscuit_hash::xx_hash;

use super::mermaid::{MermaidConfig, MermaidTheme};

/// A cache key for mermaid diagram renders.
///
/// Generated from all parameters that affect the rendered output to ensure
/// cache correctness and collision resistance.
#[derive(Debug, Clone)]
pub struct MermaidCacheKey {
    /// The xxHash of all input parameters
    hash: String,
}

impl MermaidCacheKey {
    /// Creates a new cache key from all render parameters.
    ///
    /// ## Parameters
    ///
    /// - `instructions` - The mermaid diagram source
    /// - `theme` - The mermaid theme
    /// - `scale` - The render scale factor
    /// - `config` - The JSON configuration
    /// - `transparent_background` - Whether background is transparent
    /// - `title` - Optional diagram title
    /// - `mmdc_version` - The mmdc CLI version
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::mermaid_cache::MermaidCacheKey;
    /// use biscuit_terminal::components::mermaid::{MermaidTheme, MermaidConfig};
    ///
    /// let key1 = MermaidCacheKey::new(
    ///     "flowchart LR\n    A --> B",
    ///     MermaidTheme::Dark,
    ///     2,
    ///     &MermaidConfig::new(),
    ///     false,
    ///     None,
    ///     "10.9.1",
    /// );
    ///
    /// let key2 = MermaidCacheKey::new(
    ///     "flowchart LR\n    A --> B",
    ///     MermaidTheme::Default, // Different theme
    ///     2,
    ///     &MermaidConfig::new(),
    ///     false,
    ///     None,
    ///     "10.9.1",
    /// );
    ///
    /// // Different inputs produce different keys
    /// assert_ne!(key1.hash(), key2.hash());
    /// ```
    pub fn new(
        instructions: &str,
        theme: MermaidTheme,
        scale: u32,
        config: &MermaidConfig,
        transparent_background: bool,
        title: Option<&str>,
        mmdc_version: &str,
    ) -> Self {
        // Build a deterministic string from all parameters
        let config_json = config.to_json().unwrap_or_default();
        let title_str = title.unwrap_or("");

        // Format: "v1|{instructions}|{theme}|{scale}|{config}|{transparent}|{title}|{mmdc_version}"
        // The "v1" prefix allows cache format versioning if needed in the future
        let cache_input = format!(
            "v1|{}|{}|{}|{}|{}|{}|{}",
            instructions,
            theme.as_str(),
            scale,
            config_json,
            transparent_background,
            title_str,
            mmdc_version,
        );

        let hash = format!("{:016x}", xx_hash(&cache_input));

        Self { hash }
    }

    /// Returns the hash string for this cache key.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Returns the filename for this cache entry (hash + .png).
    pub fn filename(&self) -> String {
        format!("{}.png", self.hash)
    }
}

/// File-based cache for mermaid diagram renders.
///
/// Uses the OS temp directory for storage with auto-cleanup.
#[derive(Debug)]
pub struct MermaidCache {
    /// Base directory for cache storage
    cache_dir: PathBuf,
}

impl Default for MermaidCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MermaidCache {
    /// Creates a new cache instance.
    ///
    /// Uses the OS-appropriate temp directory for storage.
    pub fn new() -> Self {
        let cache_dir = std::env::temp_dir().join("mermaid-cache");
        Self { cache_dir }
    }

    /// Creates a cache with a custom cache directory.
    ///
    /// Useful for testing or custom cache locations.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Returns the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Checks if a cached render exists for the given key.
    ///
    /// Returns the path to the cached file if it exists, None otherwise.
    pub fn get(&self, key: &MermaidCacheKey) -> Option<PathBuf> {
        let path = self.cache_dir.join(key.filename());
        if path.exists() {
            tracing::debug!(
                cache_key = %key.hash(),
                path = %path.display(),
                "Cache hit for mermaid render"
            );
            Some(path)
        } else {
            tracing::debug!(
                cache_key = %key.hash(),
                "Cache miss for mermaid render"
            );
            None
        }
    }

    /// Stores a rendered image in the cache.
    ///
    /// Copies the file at `source_path` to the cache location.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The cache directory cannot be created
    /// - The file cannot be copied
    pub fn store(&self, key: &MermaidCacheKey, source_path: &Path) -> std::io::Result<PathBuf> {
        // Ensure cache directory exists
        if !self.cache_dir.exists() {
            std::fs::create_dir_all(&self.cache_dir)?;
            tracing::debug!(
                cache_dir = %self.cache_dir.display(),
                "Created mermaid cache directory"
            );
        }

        let dest_path = self.cache_dir.join(key.filename());
        std::fs::copy(source_path, &dest_path)?;

        tracing::debug!(
            cache_key = %key.hash(),
            source = %source_path.display(),
            dest = %dest_path.display(),
            "Stored mermaid render in cache"
        );

        Ok(dest_path)
    }

    /// Returns the path where a cached render would be stored.
    ///
    /// Does not check if the file exists.
    pub fn cache_path(&self, key: &MermaidCacheKey) -> PathBuf {
        self.cache_dir.join(key.filename())
    }

    /// Clears all cached renders.
    ///
    /// ## Errors
    ///
    /// Returns an error if the cache directory cannot be removed.
    pub fn clear(&self) -> std::io::Result<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
            tracing::info!(
                cache_dir = %self.cache_dir.display(),
                "Cleared mermaid cache"
            );
        }
        Ok(())
    }

    /// Returns the approximate size of the cache in bytes.
    ///
    /// Returns 0 if the cache directory doesn't exist or cannot be read.
    pub fn size_bytes(&self) -> u64 {
        if !self.cache_dir.exists() {
            return 0;
        }

        std::fs::read_dir(&self.cache_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Returns the number of cached renders.
    ///
    /// Returns 0 if the cache directory doesn't exist or cannot be read.
    pub fn entry_count(&self) -> usize {
        if !self.cache_dir.exists() {
            return 0;
        }

        std::fs::read_dir(&self.cache_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "png")
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_same_inputs_same_hash() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        assert_eq!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_instructions() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> C", // Different
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_theme() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Default, // Different
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_scale() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            3, // Different
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_config() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let config = MermaidConfig::new().with_point_label_font_size(18);
        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &config, // Different
            false,
            None,
            "10.9.1",
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_transparent() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            true, // Different
            None,
            "10.9.1",
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_title() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            Some("My Title"), // Different
            "10.9.1",
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_mmdc_version() {
        let key1 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let key2 = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "11.0.0", // Different
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_filename() {
        let key = MermaidCacheKey::new(
            "flowchart LR\n    A --> B",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let filename = key.filename();
        assert!(filename.ends_with(".png"));
        assert!(filename.len() > 4); // More than just ".png"
    }

    #[test]
    fn test_cache_new() {
        let cache = MermaidCache::new();
        assert!(
            cache
                .cache_dir()
                .to_string_lossy()
                .contains("mermaid-cache")
        );
    }

    #[test]
    fn test_cache_with_custom_dir() {
        let custom_dir = std::env::temp_dir().join("my-custom-cache");
        let cache = MermaidCache::with_cache_dir(custom_dir.clone());
        assert_eq!(cache.cache_dir(), custom_dir);
    }

    #[test]
    fn test_cache_get_miss() {
        let cache =
            MermaidCache::with_cache_dir(std::env::temp_dir().join("mermaid-cache-test-miss"));

        let key = MermaidCacheKey::new(
            "nonexistent",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_store_and_get() {
        let cache =
            MermaidCache::with_cache_dir(std::env::temp_dir().join("mermaid-cache-test-store"));
        cache.clear().ok(); // Clean up from previous runs

        let key = MermaidCacheKey::new(
            "test diagram",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        // Create a temp file to "cache"
        let temp_dir = tempfile::tempdir().unwrap();
        let source_file = temp_dir.path().join("test.png");
        std::fs::write(&source_file, b"fake png data").unwrap();

        // Store it
        let stored_path = cache.store(&key, &source_file).unwrap();
        assert!(stored_path.exists());

        // Retrieve it
        let cached_path = cache.get(&key);
        assert!(cached_path.is_some());
        assert_eq!(cached_path.unwrap(), stored_path);

        // Clean up
        cache.clear().ok();
    }

    #[test]
    fn test_cache_entry_count() {
        let cache =
            MermaidCache::with_cache_dir(std::env::temp_dir().join("mermaid-cache-test-count"));
        cache.clear().ok();

        assert_eq!(cache.entry_count(), 0);

        // Add a cached entry
        let key = MermaidCacheKey::new(
            "test",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let temp_dir = tempfile::tempdir().unwrap();
        let source_file = temp_dir.path().join("test.png");
        std::fs::write(&source_file, b"fake png data").unwrap();

        cache.store(&key, &source_file).unwrap();
        assert_eq!(cache.entry_count(), 1);

        cache.clear().ok();
    }

    #[test]
    fn test_cache_size_bytes() {
        let cache =
            MermaidCache::with_cache_dir(std::env::temp_dir().join("mermaid-cache-test-size"));
        cache.clear().ok();

        assert_eq!(cache.size_bytes(), 0);

        // Add a cached entry
        let key = MermaidCacheKey::new(
            "test",
            MermaidTheme::Dark,
            2,
            &MermaidConfig::new(),
            false,
            None,
            "10.9.1",
        );

        let temp_dir = tempfile::tempdir().unwrap();
        let source_file = temp_dir.path().join("test.png");
        let data = b"fake png data with some content";
        std::fs::write(&source_file, data).unwrap();

        cache.store(&key, &source_file).unwrap();
        assert!(cache.size_bytes() >= data.len() as u64);

        cache.clear().ok();
    }
}
