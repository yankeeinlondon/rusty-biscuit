//! Mutation-root-guarded atomic file writes.
//!
//! These helpers are the write primitive for the effect verbs
//! (`set_frontmatter`, file/dir mutations) defined in [`super::verbs`].

use crate::effects::EffectError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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

/// Resolves `target` and verifies it is contained within `root`.
///
/// The deepest existing ancestor is canonicalized so equivalent symlinked
/// spellings compare by filesystem identity and existing symlink escapes are
/// rejected. Any missing tail is checked lexically because mutation targets
/// commonly do not exist yet.
fn normalize_within(root: &Path, target: &Path) -> Result<std::path::PathBuf, EffectError> {
    let joined = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    let cleaned = lexically_clean(&joined);
    let canonical_root =
        canonicalize_with_missing_tail(root).unwrap_or_else(|| lexically_clean(root));
    let canonical_target =
        canonicalize_with_missing_tail(&cleaned).unwrap_or_else(|| cleaned.clone());
    if !canonical_target.starts_with(&canonical_root) {
        return Err(EffectError::OutsideMutationRoot {
            path: cleaned,
            root: root.to_path_buf(),
        });
    }
    Ok(cleaned)
}

/// Canonicalize the deepest existing ancestor and reattach any missing tail.
///
/// Mutation targets commonly do not exist yet. Canonicalizing the existing
/// prefix still makes symlink aliases (`/var` versus `/private/var` on macOS)
/// comparable and prevents an existing symlink inside the root from redirecting
/// a new child outside it.
fn canonicalize_with_missing_tail(path: &Path) -> Option<PathBuf> {
    let mut ancestor = path;
    let mut tail = Vec::<OsString>::new();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(mut canonical) => {
                for component in tail.iter().rev() {
                    canonical.push(component);
                }
                return Some(lexically_clean(&canonical));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tail.push(ancestor.file_name()?.to_os_string());
                ancestor = ancestor.parent()?;
            }
            Err(_) => return None,
        }
    }
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

    #[cfg(unix)]
    #[test]
    fn equivalent_symlinked_root_spellings_are_contained() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let real_root = dir.path().join("real-root");
        let alias_root = dir.path().join("alias-root");
        std::fs::create_dir_all(&real_root).unwrap();
        symlink(&real_root, &alias_root).unwrap();

        let target = real_root.join("nested/new.txt");
        atomic_write_guarded(&alias_root, &target, b"same identity").unwrap();

        assert_eq!(std::fs::read_to_string(target).unwrap(), "same identity");
    }

    #[cfg(unix)]
    #[test]
    fn an_in_root_symlink_cannot_redirect_a_new_file_outside() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("redirect")).unwrap();

        let target = root.join("redirect/new.txt");
        let error = atomic_write_guarded(&root, &target, b"no").unwrap_err();

        assert!(matches!(error, EffectError::OutsideMutationRoot { .. }));
        assert!(!outside.join("new.txt").exists());
    }
}
