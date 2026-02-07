//! Model sharing utilities for symlink-based model sharing between runners.
//!
//! This module provides cross-platform symlink creation with fallbacks for
//! Windows (hard links, then copy) and utilities for tracking shared models.

use crate::ModelCitizenError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Record of shared models and their locations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShareRegistry {
    /// Map of original model path to list of symlink locations.
    pub shares: HashMap<String, Vec<String>>,
}

impl ShareRegistry {
    /// Creates a new empty share registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads the share registry from a JSON file.
    ///
    /// Returns an empty registry if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, ModelCitizenError> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path)?;
        let registry: Self = serde_json::from_str(&content)
            .map_err(|e| ModelCitizenError::parse(e.to_string()))?;
        Ok(registry)
    }

    /// Saves the share registry to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), ModelCitizenError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ModelCitizenError::parse(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Records a new share relationship.
    pub fn add_share(&mut self, original: &Path, symlink: &Path) {
        let original_str = original.to_string_lossy().to_string();
        let symlink_str = symlink.to_string_lossy().to_string();

        self.shares
            .entry(original_str)
            .or_default()
            .push(symlink_str);
    }

    /// Removes a share relationship.
    pub fn remove_share(&mut self, symlink: &Path) {
        let symlink_str = symlink.to_string_lossy().to_string();

        for shares in self.shares.values_mut() {
            shares.retain(|s| s != &symlink_str);
        }

        // Remove empty entries
        self.shares.retain(|_, v| !v.is_empty());
    }

    /// Gets all symlinks for an original model.
    #[must_use]
    pub fn get_shares(&self, original: &Path) -> Vec<PathBuf> {
        let original_str = original.to_string_lossy().to_string();
        self.shares
            .get(&original_str)
            .map(|v| v.iter().map(PathBuf::from).collect())
            .unwrap_or_default()
    }

    /// Finds the original model for a symlink.
    #[must_use]
    pub fn find_original(&self, symlink: &Path) -> Option<PathBuf> {
        let symlink_str = symlink.to_string_lossy().to_string();

        for (original, shares) in &self.shares {
            if shares.contains(&symlink_str) {
                return Some(PathBuf::from(original));
            }
        }

        None
    }
}

/// Creates a symlink to a model file.
///
/// Uses platform-specific strategies:
/// - **Unix (macOS/Linux)**: Creates a symbolic link
/// - **Windows**: Tries symbolic link (requires admin), falls back to hard link,
///   then falls back to file copy
///
/// ## Errors
///
/// Returns an error if symlink creation fails on all strategies.
pub fn create_symlink(src: &Path, dest: &Path) -> Result<(), ModelCitizenError> {
    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove existing file/symlink at destination
    if dest.exists() || dest.is_symlink() {
        std::fs::remove_file(dest)?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest)?;
        Ok(())
    }

    #[cfg(windows)]
    {
        // Try symlink first (requires admin or developer mode)
        if std::os::windows::fs::symlink_file(src, dest).is_ok() {
            return Ok(());
        }

        // Try hard link (only works on same filesystem)
        if std::fs::hard_link(src, dest).is_ok() {
            return Ok(());
        }

        // Fall back to copy
        std::fs::copy(src, dest)?;
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Unsupported platform - just copy
        std::fs::copy(src, dest)?;
        Ok(())
    }
}

/// Validates that a symlink is not broken.
///
/// Returns `true` if the symlink exists and points to a valid target.
#[must_use]
pub fn validate_symlink(path: &Path) -> bool {
    if !path.is_symlink() {
        return false;
    }

    // Check if target exists
    match std::fs::read_link(path) {
        Ok(target) => {
            // Handle relative symlinks
            let absolute_target = if target.is_relative() {
                path.parent().map(|p| p.join(&target)).unwrap_or(target)
            } else {
                target
            };
            absolute_target.exists()
        }
        Err(_) => false,
    }
}

/// Resolves a symlink to its original file.
///
/// Follows symlinks recursively until reaching a non-symlink file.
///
/// ## Errors
///
/// Returns an error if the symlink chain is broken or exceeds depth limit.
pub fn resolve_original(path: &Path) -> Result<PathBuf, ModelCitizenError> {
    const MAX_DEPTH: usize = 20;

    let mut current = path.to_path_buf();
    let mut depth = 0;

    while current.is_symlink() {
        if depth >= MAX_DEPTH {
            return Err(ModelCitizenError::parse(
                "Symlink chain exceeds maximum depth",
            ));
        }

        let target = std::fs::read_link(&current)?;

        // Handle relative symlinks
        current = if target.is_relative() {
            current
                .parent()
                .map(|p| p.join(&target))
                .unwrap_or(target)
        } else {
            target
        };

        depth += 1;
    }

    if !current.exists() {
        return Err(ModelCitizenError::not_found(current));
    }

    Ok(current)
}

/// Checks if a path is a symlink.
#[must_use]
pub fn is_symlink(path: &Path) -> bool {
    path.is_symlink()
}

/// Gets the default shared models directory.
///
/// - Uses `MODEL_CITIZEN_SHARED_DIR` env var if set
/// - Falls back to `~/.local/share/model-citizen/shared` on Unix
/// - Falls back to `%LOCALAPPDATA%\model-citizen\shared` on Windows
#[must_use]
pub fn default_shared_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MODEL_CITIZEN_SHARED_DIR") {
        return Some(PathBuf::from(dir));
    }

    dirs::data_local_dir().map(|d| d.join("model-citizen").join("shared"))
}

/// Gets the default share registry path.
#[must_use]
pub fn default_registry_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("model-citizen").join("shares.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    #[test]
    fn share_registry_new_is_empty() {
        let registry = ShareRegistry::new();
        assert!(registry.shares.is_empty());
    }

    #[test]
    fn share_registry_add_and_get() {
        let mut registry = ShareRegistry::new();

        let original = PathBuf::from("/models/original.gguf");
        let symlink1 = PathBuf::from("/ollama/models/model.gguf");
        let symlink2 = PathBuf::from("/lmstudio/models/model.gguf");

        registry.add_share(&original, &symlink1);
        registry.add_share(&original, &symlink2);

        let shares = registry.get_shares(&original);
        assert_eq!(shares.len(), 2);
        assert!(shares.contains(&symlink1));
        assert!(shares.contains(&symlink2));
    }

    #[test]
    fn share_registry_remove_share() {
        let mut registry = ShareRegistry::new();

        let original = PathBuf::from("/models/original.gguf");
        let symlink1 = PathBuf::from("/ollama/models/model.gguf");
        let symlink2 = PathBuf::from("/lmstudio/models/model.gguf");

        registry.add_share(&original, &symlink1);
        registry.add_share(&original, &symlink2);
        registry.remove_share(&symlink1);

        let shares = registry.get_shares(&original);
        assert_eq!(shares.len(), 1);
        assert!(!shares.contains(&symlink1));
        assert!(shares.contains(&symlink2));
    }

    #[test]
    fn share_registry_find_original() {
        let mut registry = ShareRegistry::new();

        let original = PathBuf::from("/models/original.gguf");
        let symlink = PathBuf::from("/ollama/models/model.gguf");

        registry.add_share(&original, &symlink);

        assert_eq!(registry.find_original(&symlink), Some(original));
        assert_eq!(
            registry.find_original(&PathBuf::from("/unknown")),
            None
        );
    }

    #[test]
    fn share_registry_save_and_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let registry_path = temp_dir.path().join("shares.json");

        let mut registry = ShareRegistry::new();
        registry.add_share(
            &PathBuf::from("/models/original.gguf"),
            &PathBuf::from("/symlink.gguf"),
        );

        registry.save(&registry_path).unwrap();
        let loaded = ShareRegistry::load(&registry_path).unwrap();

        assert_eq!(loaded.shares.len(), 1);
    }

    #[test]
    fn share_registry_load_missing_file() {
        let registry = ShareRegistry::load(Path::new("/nonexistent/shares.json")).unwrap();
        assert!(registry.shares.is_empty());
    }

    #[test]
    fn create_symlink_creates_link() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = create_test_file(temp_dir.path(), "source.gguf", b"model data");
        let dest = temp_dir.path().join("link.gguf");

        create_symlink(&src, &dest).unwrap();

        #[cfg(unix)]
        assert!(dest.is_symlink());

        // On all platforms, the file should exist and have the same content
        assert!(dest.exists());
        let content = std::fs::read(&dest).unwrap();
        assert_eq!(content, b"model data");
    }

    #[test]
    fn create_symlink_replaces_existing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = create_test_file(temp_dir.path(), "source.gguf", b"new data");
        let dest = create_test_file(temp_dir.path(), "link.gguf", b"old data");

        create_symlink(&src, &dest).unwrap();

        let content = std::fs::read(&dest).unwrap();
        assert_eq!(content, b"new data");
    }

    #[test]
    fn create_symlink_creates_parent_dirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = create_test_file(temp_dir.path(), "source.gguf", b"data");
        let dest = temp_dir.path().join("nested/dir/link.gguf");

        create_symlink(&src, &dest).unwrap();
        assert!(dest.exists());
    }

    #[test]
    fn validate_symlink_returns_true_for_valid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = create_test_file(temp_dir.path(), "source.gguf", b"data");
        let dest = temp_dir.path().join("link.gguf");

        create_symlink(&src, &dest).unwrap();

        #[cfg(unix)]
        assert!(validate_symlink(&dest));

        // On Windows with copy fallback, it's not a symlink
        #[cfg(windows)]
        {
            // validate_symlink returns false for non-symlinks
            let _ = validate_symlink(&dest);
        }
    }

    #[test]
    fn validate_symlink_returns_false_for_broken() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = temp_dir.path().join("source.gguf");
        let dest = temp_dir.path().join("link.gguf");

        #[cfg(unix)]
        {
            // Create symlink to non-existent file
            std::os::unix::fs::symlink(&src, &dest).unwrap();
            assert!(!validate_symlink(&dest));
        }
    }

    #[test]
    fn validate_symlink_returns_false_for_regular_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = create_test_file(temp_dir.path(), "file.gguf", b"data");
        assert!(!validate_symlink(&file));
    }

    #[test]
    fn resolve_original_follows_symlink() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = create_test_file(temp_dir.path(), "source.gguf", b"data");
        let link = temp_dir.path().join("link.gguf");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&src, &link).unwrap();
            let resolved = resolve_original(&link).unwrap();
            assert_eq!(resolved, src);
        }

        #[cfg(windows)]
        {
            // On Windows, just verify it works with regular files
            let resolved = resolve_original(&src).unwrap();
            assert_eq!(resolved, src);
        }
    }

    #[test]
    fn resolve_original_returns_regular_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = create_test_file(temp_dir.path(), "file.gguf", b"data");

        let resolved = resolve_original(&file).unwrap();
        assert_eq!(resolved, file);
    }

    #[test]
    fn resolve_original_errors_for_broken_symlink() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = temp_dir.path().join("source.gguf");
        let link = temp_dir.path().join("link.gguf");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&src, &link).unwrap();
            let result = resolve_original(&link);
            assert!(result.is_err());
        }
    }

    #[test]
    fn is_symlink_detects_symlinks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = create_test_file(temp_dir.path(), "file.gguf", b"data");

        assert!(!is_symlink(&file));

        #[cfg(unix)]
        {
            let link = temp_dir.path().join("link.gguf");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(is_symlink(&link));
        }
    }

    #[test]
    fn default_shared_dir_returns_path() {
        let dir = default_shared_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.to_string_lossy().contains("model-citizen"));
    }

    #[test]
    fn default_registry_path_returns_path() {
        let path = default_registry_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("shares.json"));
    }
}
