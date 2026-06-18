//! Mutation-root-guarded atomic file writes.
//!
//! These helpers are the write primitive for the effect verbs
//! (`set_frontmatter`, file/dir mutations) defined in [`super::verbs`].

use crate::effects::EffectError;
use std::path::Path;

/// Writes `bytes` to `target` atomically (temp file in the same directory +
/// rename), but only if `target` resolves inside `root`. Creates parent
/// directories under `root` as needed.
pub(crate) fn atomic_write_guarded(
    root: &Path,
    target: &Path,
    bytes: &[u8],
) -> Result<(), EffectError> {
    let normalized = normalize_within(root, target)?;
    if let Some(parent) = normalized.parent() {
        std::fs::create_dir_all(parent).map_err(|source| EffectError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut tmp =
        tempfile::NamedTempFile::new_in(normalized.parent().unwrap_or(root)).map_err(|source| {
            EffectError::Io {
                path: normalized.clone(),
                source,
            }
        })?;
    use std::io::Write;
    tmp.write_all(bytes).map_err(|source| EffectError::Io {
        path: normalized.clone(),
        source,
    })?;
    tmp.persist(&normalized).map_err(|e| EffectError::Io {
        path: normalized.clone(),
        source: e.error,
    })?;
    Ok(())
}

/// Resolves `target` and verifies it is contained within `root`. Uses lexical
/// containment after joining relative targets onto `root` — the path may not
/// exist yet, so containment is checked without touching disk.
fn normalize_within(root: &Path, target: &Path) -> Result<std::path::PathBuf, EffectError> {
    let joined = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    let cleaned = lexically_clean(&joined);
    if !cleaned.starts_with(root) {
        return Err(EffectError::OutsideMutationRoot {
            path: cleaned,
            root: root.to_path_buf(),
        });
    }
    Ok(cleaned)
}

/// Removes `.` and resolves `..` segments lexically without touching disk.
fn lexically_clean(path: &Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Containment check returning the cleaned in-root path, for verbs that resolve
/// a target before reading or mutating it.
pub(crate) fn ensure_within(
    root: &Path,
    target: &Path,
) -> Result<std::path::PathBuf, EffectError> {
    normalize_within(root, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_inside_root_succeeds_and_outside_is_refused() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let inside = root.join("ok.txt");
        atomic_write_guarded(&root, &inside, b"hi").unwrap();
        assert_eq!(std::fs::read_to_string(&inside).unwrap(), "hi");

        let outside = root.parent().unwrap().join("escape.txt");
        let err = atomic_write_guarded(&root, &outside, b"no").unwrap_err();
        assert!(matches!(
            err,
            crate::effects::EffectError::OutsideMutationRoot { .. }
        ));
    }
}
