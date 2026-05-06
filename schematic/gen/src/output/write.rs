//! Atomic file writing for generated code.

use std::fs;
use std::path::Path;

use crate::errors::GeneratorError;

/// Writes content to a file atomically using temp file + rename.
///
/// This pattern ensures that:
/// - The file is never left in a partially-written state
/// - Other processes see either the old or new content, never a mix
/// - Power failures or crashes don't corrupt the file
///
/// ## Arguments
///
/// * `path` - The target file path
/// * `content` - The content to write
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

    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, content).map_err(|e| GeneratorError::WriteError {
        path: temp_path.display().to_string(),
        source: e,
    })?;

    fs::rename(&temp_path, path).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}
