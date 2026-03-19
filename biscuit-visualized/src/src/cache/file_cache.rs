use std::fs;
use std::io;
use std::path::PathBuf;

use crate::artifact::OutputFormat;
use crate::cache::VisualizationKind;

/// Backend identifiers for cache key generation.
pub const MERMAID_BACKEND: &str = "mermaid-rs-renderer@0.2.x";
pub const GRAPH_BACKEND: &str = "layout-rs@0.1.x";
pub const RASTERIZER: &str = "resvg@0.45";

/// File-based cache for rendered visualizations.
///
/// Stores artifacts in a hierarchical directory structure organized by
/// visualization kind and output format. Uses xxHash for cache key generation.
///
/// ## Directory Structure
///
/// ```text
/// $TMPDIR/biscuit-visualized/v1/
///   ├── mermaid/
///   │   ├── svg/
///   │   └── png/
///   └── graph/
///       ├── svg/
///       └── png/
/// ```
#[derive(Debug, Clone)]
pub struct FileCache {
    base_dir: PathBuf,
}

impl FileCache {
    /// Creates a new file cache.
    ///
    /// The cache directory is created at `$TMPDIR/biscuit-visualized/v1/`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::cache::FileCache;
    ///
    /// let cache = FileCache::new();
    /// ```
    pub fn new() -> Self {
        let base_dir = std::env::temp_dir().join("biscuit-visualized").join("v1");

        Self { base_dir }
    }

    /// Returns the cache directory for a specific visualization kind and format.
    pub fn cache_dir(&self, kind: VisualizationKind, format: OutputFormat) -> PathBuf {
        self.base_dir.join(kind.as_str()).join(format.extension())
    }

    /// Generates a cache key from visualization parameters.
    ///
    /// The cache key includes the visualization kind, source content, rendering
    /// options, backend identifier, and output format. Changes to any of these
    /// parameters will result in a different cache key.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::cache::{FileCache, VisualizationKind};
    /// use biscuit_visualized::artifact::OutputFormat;
    /// use biscuit_visualized::cache::file_cache::MERMAID_BACKEND;
    ///
    /// let key = FileCache::cache_key(
    ///     VisualizationKind::Mermaid,
    ///     "graph TD; A-->B",
    ///     r#"{"theme":"dark"}"#,
    ///     MERMAID_BACKEND,
    ///     OutputFormat::Png,
    /// );
    /// ```
    pub fn cache_key(
        kind: VisualizationKind,
        source: &str,
        options_json: &str,
        backend_id: &str,
        format: OutputFormat,
    ) -> String {
        let key_string = format!(
            "v1|{}|{}|{}|{}|{}",
            kind.as_str(),
            source,
            options_json,
            backend_id,
            format.extension()
        );
        biscuit_hash::xx_hash(&key_string).to_string()
    }

    /// Retrieves a cached artifact if it exists.
    ///
    /// ## Returns
    ///
    /// The path to the cached file if it exists, otherwise `None`.
    pub fn get(&self, kind: VisualizationKind, key: &str, format: OutputFormat) -> Option<PathBuf> {
        let path = self
            .cache_dir(kind, format)
            .join(format!("{}.{}", key, format.extension()));

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Stores data in the cache.
    ///
    /// Creates the cache directory hierarchy if it doesn't exist.
    ///
    /// ## Returns
    ///
    /// The path to the stored file.
    ///
    /// ## Errors
    ///
    /// Returns an error if directory creation or file writing fails.
    pub fn store(
        &self,
        kind: VisualizationKind,
        key: &str,
        format: OutputFormat,
        data: &[u8],
    ) -> io::Result<PathBuf> {
        let cache_dir = self.cache_dir(kind, format);
        fs::create_dir_all(&cache_dir)?;

        let path = cache_dir.join(format!("{}.{}", key, format.extension()));
        fs::write(&path, data)?;

        Ok(path)
    }

    /// Clears the entire cache.
    ///
    /// Removes the cache directory and all its contents, then recreates
    /// the base directory.
    ///
    /// ## Errors
    ///
    /// Returns an error if directory removal or creation fails.
    pub fn clear(&self) -> io::Result<()> {
        if self.base_dir.exists() {
            fs::remove_dir_all(&self.base_dir)?;
        }
        fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    /// Returns the total size of cached files in bytes.
    pub fn size_bytes(&self) -> u64 {
        fn walk_dir_size(path: &PathBuf) -> u64 {
            if !path.exists() {
                return 0;
            }

            let Ok(entries) = fs::read_dir(path) else {
                return 0;
            };

            entries
                .filter_map(Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        walk_dir_size(&path)
                    } else {
                        fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                    }
                })
                .sum()
        }

        walk_dir_size(&self.base_dir)
    }

    /// Returns the number of cached entries.
    pub fn entry_count(&self) -> usize {
        fn walk_dir_count(path: &PathBuf) -> usize {
            if !path.exists() {
                return 0;
            }

            let Ok(entries) = fs::read_dir(path) else {
                return 0;
            };

            entries
                .filter_map(Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        walk_dir_count(&path)
                    } else {
                        1
                    }
                })
                .sum()
        }

        walk_dir_count(&self.base_dir)
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}
