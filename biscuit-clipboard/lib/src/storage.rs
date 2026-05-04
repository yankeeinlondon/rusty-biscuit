//! Disk-spill cache for large clipboard payloads.
//!
//! [`Storage`] owns a single content-addressed cache directory
//! (default: `<dirs::cache_dir>/biscuit-clipboard`). Inline images
//! larger than the configured threshold are written there as
//! `{xxhash:016x}.dat` files; the in-memory entry switches to
//! `ImageSnapshot::Spilled { path, ... }` and the bytes are loaded on
//! demand via [`Storage::load_spilled`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::content::ImageSnapshot;
use crate::error::ClipboardError;

const DEFAULT_SPILL_THRESHOLD: usize = 64 * 1024;

/// Disk-spill cache for large clipboard payloads.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_clipboard::Storage;
///
/// let storage = Storage::new().expect("storage init");
/// // Use the storage to spill or load images...
/// ```
#[derive(Clone)]
pub struct Storage {
    cache_dir: PathBuf,
    spill_threshold: usize,
}

impl Storage {
    /// Build a [`Storage`] rooted at the platform default cache dir.
    ///
    /// ## Errors
    ///
    /// Returns [`ClipboardError::Backend`] if the platform cache
    /// directory cannot be determined; returns [`ClipboardError::Io`]
    /// if creating the directory fails.
    pub fn new() -> Result<Self, ClipboardError> {
        let cache_dir = Self::default_cache_dir()?;
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            cache_dir,
            spill_threshold: DEFAULT_SPILL_THRESHOLD,
        })
    }

    pub fn with_cache_dir(mut self, dir: PathBuf) -> Self {
        self.cache_dir = dir;
        self
    }

    pub fn with_spill_threshold(mut self, threshold: usize) -> Self {
        self.spill_threshold = threshold;
        self
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Compute a content-addressed hash of the inline image bytes.
    /// Returns the same hash for two inline images with identical bytes,
    /// so the spilled file lands at the same path.
    pub fn hash_image_payload(image: &ImageSnapshot) -> u64 {
        match image {
            ImageSnapshot::Inline { data, .. } => biscuit_hash::xx_hash_bytes(data),
            ImageSnapshot::Spilled { size_bytes, .. } => *size_bytes,
        }
    }

    pub fn spill_if_needed(&self, content_hash: u64, image: ImageSnapshot) -> ImageSnapshot {
        match &image {
            ImageSnapshot::Inline {
                data,
                width,
                height,
            } => {
                if data.len() > self.spill_threshold {
                    let file_name = format!("{:016x}.dat", content_hash);
                    let path = self.cache_dir.join(&file_name);
                    if let Ok(()) = fs::write(&path, data) {
                        ImageSnapshot::Spilled {
                            path,
                            width: *width,
                            height: *height,
                            size_bytes: data.len() as u64,
                        }
                    } else {
                        image
                    }
                } else {
                    image
                }
            }
            ImageSnapshot::Spilled { .. } => image,
        }
    }

    pub fn load_spilled(&self, image: &ImageSnapshot) -> Result<Vec<u8>, ClipboardError> {
        match image {
            ImageSnapshot::Inline { data, .. } => Ok(data.clone()),
            ImageSnapshot::Spilled { path, .. } => fs::read(path).map_err(ClipboardError::Io),
        }
    }

    pub fn cleanup_spilled(&self, image: &ImageSnapshot) -> Result<(), ClipboardError> {
        if let ImageSnapshot::Spilled { path, .. } = image
            && path.exists()
        {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn cleanup_entry_files(&self, hash_hex: &str) -> Result<(), ClipboardError> {
        let file_name = format!("{hash_hex}.dat");
        let path = self.cache_dir.join(&file_name);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn default_cache_dir() -> Result<PathBuf, ClipboardError> {
        let base = dirs::cache_dir().ok_or_else(|| {
            ClipboardError::Backend("Could not determine platform cache directory".to_string())
        })?;
        Ok(base.join("biscuit-clipboard"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_spill_inline_below_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf())
            .with_spill_threshold(100);

        let img = ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 10,
            height: 10,
        };
        let result = storage.spill_if_needed(123, img);
        assert!(matches!(result, ImageSnapshot::Inline { .. }));
    }

    #[test]
    fn test_spill_inline_above_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf())
            .with_spill_threshold(10);

        let img = ImageSnapshot::Inline {
            data: vec![1u8; 50],
            width: 10,
            height: 10,
        };
        let result = storage.spill_if_needed(0xABCD, img);
        match result {
            ImageSnapshot::Spilled {
                path,
                width,
                height,
                ..
            } => {
                assert_eq!(width, 10);
                assert_eq!(height, 10);
                assert!(path.exists());
                let loaded = fs::read(&path).unwrap();
                assert_eq!(loaded, vec![1u8; 50]);
            }
            ImageSnapshot::Inline { .. } => panic!("Expected Spilled variant"),
        }
    }

    #[test]
    fn test_load_spilled_inline() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());

        let img = ImageSnapshot::Inline {
            data: vec![42, 43, 44],
            width: 5,
            height: 5,
        };
        let data = storage.load_spilled(&img).unwrap();
        assert_eq!(data, vec![42, 43, 44]);
    }

    #[test]
    fn test_load_spilled_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf())
            .with_spill_threshold(10);

        let img = ImageSnapshot::Inline {
            data: vec![7u8; 20],
            width: 5,
            height: 5,
        };
        let spilled = storage.spill_if_needed(0xBEEF, img);

        let loaded = storage.load_spilled(&spilled).unwrap();
        assert_eq!(loaded, vec![7u8; 20]);
    }

    #[test]
    fn test_cleanup_spilled() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf())
            .with_spill_threshold(10);

        let img = ImageSnapshot::Inline {
            data: vec![1u8; 20],
            width: 5,
            height: 5,
        };
        let spilled = storage.spill_if_needed(0xCAFE, img);
        let path = match &spilled {
            ImageSnapshot::Spilled { path, .. } => path.clone(),
            _ => panic!("Expected Spilled"),
        };
        assert!(path.exists());
        storage.cleanup_spilled(&spilled).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_cleanup_inline_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());

        let img = ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 5,
            height: 5,
        };
        storage.cleanup_spilled(&img).unwrap();
    }

    #[test]
    fn test_disk_spill_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf())
            .with_spill_threshold(16);

        let original_data: Vec<u8> = (0..64).collect();
        let img = ImageSnapshot::Inline {
            data: original_data.clone(),
            width: 8,
            height: 8,
        };

        let spilled = storage.spill_if_needed(0x1234, img.clone());

        match &spilled {
            ImageSnapshot::Spilled {
                path,
                width,
                height,
                ..
            } => {
                assert_eq!(*width, 8);
                assert_eq!(*height, 8);
                assert!(path.exists());
                assert!(path.extension().is_some_and(|e| e == "dat"));
            }
            _ => panic!("Should have spilled"),
        }

        let loaded = storage.load_spilled(&spilled).unwrap();
        assert_eq!(loaded, original_data);
    }
}
