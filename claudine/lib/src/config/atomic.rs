use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;
use tracing::warn;

/// Write content to a file atomically using temp file + rename.
///
/// Creates a uniquely-named temp file inside the target's parent directory
/// via [`tempfile::NamedTempFile::new_in`], writes the content with
/// `fsync`, then persists it via an atomic `rename(2)` on the same
/// filesystem. On Unix the parent directory is also `fsync`'d so the
/// rename's metadata change survives a power loss.
///
/// ## Concurrency
///
/// Each call uses a unique temp file, so concurrent writers never corrupt
/// each other's in-flight bytes. `rename` serializes on the parent
/// directory inode; the final content is always an intact copy of exactly
/// one writer's payload (last-rename-wins).
///
/// ## Errors
///
/// Returns the [`std::io::Error`] raised if the parent directory cannot be
/// created, the temp file cannot be written or fsync'd, or the atomic rename
/// fails. No non-atomic fallback is attempted — a failed rename indicates a
/// cross-device move or filesystem error that a byte-by-byte copy could
/// leave in an inconsistent state.
///
/// The error type is `io::Error` rather than [`crate::error::ClaudineError`]
/// because every fallible step here *is* an `io::Error`; a wider type would
/// force every caller wanting the real cause to widen with it. Callers
/// returning [`crate::error::Result`] convert for free via `?` and
/// `ClaudineError`'s `#[from] std::io::Error`.
pub fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.as_file_mut().write_all(content)?;
    tmp.as_file_mut().sync_all()?;

    // `persist` performs an atomic `rename(2)` on the same filesystem.
    // If it fails (cross-device, filesystem error), surface the error
    // rather than fall back to a non-atomic copy that could leave the
    // target truncated on a crash mid-write.
    tmp.persist(path).map_err(|e| e.error)?;

    #[cfg(unix)]
    fsync_dir(parent);

    Ok(())
}

#[cfg(unix)]
fn fsync_dir(dir: &Path) {
    match fs::File::open(dir) {
        Ok(f) => {
            if let Err(err) = f.sync_all() {
                warn!(%err, directory = %dir.display(), "fsync on parent directory failed");
            }
        }
        Err(err) => {
            warn!(%err, directory = %dir.display(), "opening parent directory for fsync failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

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
    fn no_stray_tmp_sibling_after_write() {
        // The old implementation left `<name>.tmp` siblings behind on some
        // failure paths. `NamedTempFile::persist` consumes the temp file,
        // so no sibling must survive a successful write.
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("config.json");

        atomic_write(&target, b"{}").unwrap();

        let tmp_sibling = target.with_extension("tmp");
        assert!(!tmp_sibling.exists());
    }

    #[test]
    fn overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("settings.json");

        atomic_write(&target, b"original").unwrap();
        atomic_write(&target, b"updated").unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "updated");
    }

    /// Concurrent writers must produce one of the input payloads — never a
    /// blend. With the previous fixed `.tmp` sibling this could produce a
    /// torn file; with `NamedTempFile::new_in(parent)` each writer owns a
    /// unique temp file so the final content is always an intact copy of
    /// exactly one writer's bytes.
    #[test]
    fn concurrent_writers_produce_intact_payload() {
        let tmp = TempDir::new().unwrap();
        let target = Arc::new(tmp.path().join("shared.json"));

        let payloads: Vec<Vec<u8>> = (0..8)
            .map(|i| format!("payload-{i:08}-{}", "x".repeat(4096)).into_bytes())
            .collect();

        let mut handles = Vec::new();
        for payload in payloads.clone() {
            let target = Arc::clone(&target);
            handles.push(thread::spawn(move || {
                atomic_write(&target, &payload).expect("atomic_write must succeed");
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        let final_bytes = fs::read(&*target).expect("final file readable");
        assert!(
            payloads.iter().any(|p| p == &final_bytes),
            "final content must be exactly one of the input payloads (never a blend)"
        );
    }
}
