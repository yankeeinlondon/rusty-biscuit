use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use fs2::FileExt;

use crate::error::{ClaudineError, Result};

/// Write content to a file atomically using temp file + rename.
///
/// Uses `fs2::FileExt::lock_exclusive()` to prevent concurrent writes.
/// Writes to a `.tmp` sibling file, then renames atomically.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Build temp path in same directory for atomic rename
    let tmp_path = path.with_extension("tmp");

    // Write to temp file with exclusive lock
    let result = (|| -> Result<()> {
        let mut file = File::create(&tmp_path)?;
        file.lock_exclusive()
            .map_err(|_| ClaudineError::LockError {
                path: tmp_path.to_path_buf(),
            })?;
        file.write_all(content)?;
        file.sync_all()?;
        // Lock released on drop
        Ok(())
    })();

    if let Err(e) = result {
        // Clean up temp file on error
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    // Atomic rename (same filesystem)
    if fs::rename(&tmp_path, path).is_err() {
        // Fall back to copy + remove
        let copy_result: std::result::Result<u64, ClaudineError> =
            fs::copy(&tmp_path, path).map_err(ClaudineError::from);
        let _ = fs::remove_file(&tmp_path);
        copy_result?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn writes_content_atomically() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("output.json");

        let content = b"hello world";
        atomic_write(&target, content).unwrap();

        assert_eq!(fs::read(&target).unwrap(), content);
    }

    #[test]
    fn creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("nested").join("dir").join("file.txt");

        atomic_write(&target, b"test").unwrap();

        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "test");
    }

    #[test]
    fn temp_file_cleaned_up_after_write() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("config.json");

        atomic_write(&target, b"{}").unwrap();

        let tmp_path = target.with_extension("tmp");
        assert!(!tmp_path.exists());
    }

    #[test]
    fn overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("settings.json");

        atomic_write(&target, b"original").unwrap();
        atomic_write(&target, b"updated").unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "updated");
    }
}
