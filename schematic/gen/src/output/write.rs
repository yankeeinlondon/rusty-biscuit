//! Atomic file writing for generated code.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::GeneratorError;

/// Monotonic counter that disambiguates temp file names within a process.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes content to a file via a temp file + rename.
///
/// The rename is atomic, so a reader observes either the old or the new content
/// and never a partial write. This is *not* fsync-durable: a crash between the
/// rename and the OS flushing its buffers can still lose the write. The temp
/// file carries a per-process-unique suffix so concurrent invocations targeting
/// the same path do not clobber each other's temp file.
///
/// ## Returns
///
/// `Ok(())` on success.
///
/// ## Errors
///
/// Returns `GeneratorError::WriteError` if:
/// - Parent directories cannot be created
/// - The temp file cannot be written
/// - The rename operation fails
pub fn write_atomic(path: &Path, content: &str) -> Result<(), GeneratorError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| GeneratorError::WriteError {
            path: parent.display().to_string(),
            source: e,
        })?;
    }

    // A fixed `.tmp` name lets concurrent runs overwrite each other's temp
    // file; pid + counter keeps each write's scratch path distinct.
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(
        "{}.{}.{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "out".to_string()),
        std::process::id(),
        counter,
    );
    let temp_path = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(temp_name),
        _ => Path::new(&temp_name).to_path_buf(),
    };

    fs::write(&temp_path, content).map_err(|e| GeneratorError::WriteError {
        path: temp_path.display().to_string(),
        source: e,
    })?;

    fs::rename(&temp_path, path).map_err(|e| {
        // Best-effort cleanup so a failed rename does not strand the temp file.
        let _ = fs::remove_file(&temp_path);
        GeneratorError::WriteError {
            path: path.display().to_string(),
            source: e,
        }
    })?;

    Ok(())
}
