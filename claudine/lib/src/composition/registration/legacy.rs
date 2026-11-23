//! Removing the retired format 1 registration.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{CLAUDINE_DIR, Entry, Obstruction, PublishError, entry_at, legacy_manifest_path, legacy_revisions_dir};

/// Why [`remove_legacy_registration`] removed nothing.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CleanupError {
    /// A link stands in the retired registration; it and its target are
    /// left alone.
    #[error("`{}` is {obstruction}; the retired registration is never followed through one, so nothing was removed. Remove it, then delete `~/.claudine/darkmatter-host.yaml` and `~/.claudine/darkmatter-host/` yourself", .path.display())]
    Obstructed {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        obstruction: Obstruction,
    },
    /// Something other than a regular file or a directory stands where the
    /// retired registration was written.
    #[error("`{}` is not what the retired publisher wrote, so nothing was removed", .path.display())]
    Unexpected {
        #[allow(missing_docs)]
        path: PathBuf,
    },
    /// A file-system operation failed; entries removed before it stay
    /// removed, and the next cleanup removes the rest.
    #[error("could not {action} `{}`: {source}", .path.display())]
    Io {
        #[allow(missing_docs)]
        action: &'static str,
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        source: io::Error,
    },
}

/// What [`remove_legacy_registration`] removed: the retired manifest and the
/// revision tree's root, when each existed. Empty when nothing was left.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyCleanup {
    #[allow(missing_docs)]
    pub removed: Vec<PathBuf>,
}

/// Removes the retired format 1 registration: exactly
/// [`legacy_manifest_path`] and the tree at [`legacy_revisions_dir`], and
/// nothing else under `~/.claudine`. Setup calls it only after the
/// `register` entry naming the old manifest was replaced.
///
/// `~/.claudine`, the manifest, and every entry of the revision tree are
/// examined without following links before anything is removed. A symbolic
/// link, a junction, or another reparse point anywhere is
/// [`CleanupError::Obstructed`]: nothing is removed and nothing is followed.
///
/// ## Errors
///
/// [`CleanupError`]; on an obstruction or an unexpected entry nothing was
/// removed.
pub fn remove_legacy_registration(home: &Path) -> Result<LegacyCleanup, CleanupError> {
    let inspect = |path: &Path| match entry_at(path) {
        Ok(Entry::Obstruction(obstruction)) => Err(CleanupError::Obstructed { path: path.to_path_buf(), obstruction }),
        Ok(entry) => Ok(entry),
        Err(PublishError::Io { action, path, source }) => Err(CleanupError::Io { action, path, source }),
        Err(_) => Err(CleanupError::Unexpected { path: path.to_path_buf() }),
    };
    if inspect(&home.join(CLAUDINE_DIR))? != Entry::Directory {
        return Ok(LegacyCleanup::default());
    }
    let manifest = legacy_manifest_path(home);
    let manifest_present = match inspect(&manifest)? {
        Entry::Absent => false,
        Entry::File => true,
        _ => return Err(CleanupError::Unexpected { path: manifest }),
    };
    let revisions = legacy_revisions_dir(home);
    // Children before their folder, so removal runs front to back.
    let mut tree = Vec::new();
    match inspect(&revisions)? {
        Entry::Absent => {}
        Entry::Directory => collect_tree(&revisions, &inspect, &mut tree)?,
        _ => return Err(CleanupError::Unexpected { path: revisions }),
    }

    let mut cleanup = LegacyCleanup::default();
    for (path, is_directory) in &tree {
        let removed = if *is_directory { fs::remove_dir(path) } else { fs::remove_file(path) };
        removed.map_err(|source| CleanupError::Io { action: "remove", path: path.clone(), source })?;
    }
    if !tree.is_empty() {
        cleanup.removed.push(revisions);
    }
    if manifest_present {
        fs::remove_file(&manifest).map_err(|source| CleanupError::Io { action: "remove", path: manifest.clone(), source })?;
        cleanup.removed.push(manifest);
    }
    Ok(cleanup)
}

/// Every entry below and including the directory `directory`, children
/// first, each marked `true` for a directory.
fn collect_tree(
    directory: &Path,
    inspect: &dyn Fn(&Path) -> Result<Entry, CleanupError>,
    tree: &mut Vec<(PathBuf, bool)>,
) -> Result<(), CleanupError> {
    let io = |source| CleanupError::Io { action: "read", path: directory.to_path_buf(), source };
    for child in fs::read_dir(directory).map_err(io)? {
        let path = child.map_err(io)?.path();
        match inspect(&path)? {
            Entry::Directory => collect_tree(&path, inspect, tree)?,
            Entry::File => tree.push((path, false)),
            Entry::Absent => {}
            _ => return Err(CleanupError::Unexpected { path }),
        }
    }
    tree.push((directory.to_path_buf(), true));
    Ok(())
}
