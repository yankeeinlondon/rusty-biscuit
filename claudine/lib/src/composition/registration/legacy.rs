//! Removing the retired format 1 registration.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    CLAUDINE_DIR, Entry, Obstruction, PublishError, access_log, entry_at, legacy_manifest_path, legacy_revisions_dir,
    remove_entry,
};

/// Why [`remove_legacy_registration`] stopped; [`CleanupFailure`] pairs it
/// with what was removed before.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CleanupError {
    /// A link stands in the retired registration; it and its target are
    /// left alone.
    #[error("`{}` is {obstruction}; the retired registration is never followed through one, so it was left in place. Remove it, then delete `~/.claudine/darkmatter-host.yaml` and `~/.claudine/darkmatter-host/` yourself", .path.display())]
    Obstructed {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        obstruction: Obstruction,
    },
    /// Something other than a regular file or a directory stands where the
    /// retired registration was written.
    #[error("`{}` is not what the retired publisher wrote, so it was left in place", .path.display())]
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

/// What [`remove_legacy_registration`] removed: the revision tree's root
/// and the retired manifest, when each was there to remove. Empty when
/// nothing was left.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyCleanup {
    #[allow(missing_docs)]
    pub removed: Vec<PathBuf>,
}

/// A [`remove_legacy_registration`] that stopped partway.
#[derive(Debug, thiserror::Error)]
#[error("{error}{}", already_removed(.removed))]
pub struct CleanupFailure {
    /// What was removed before the failure, each entry once: a removed
    /// folder stands for everything that was in it, so a revision tree whose
    /// root could not be removed is listed by the entries removed from it.
    /// Empty when nothing was removed.
    pub removed: Vec<PathBuf>,
    #[allow(missing_docs)]
    #[source]
    pub error: CleanupError,
}

fn already_removed(removed: &[PathBuf]) -> String {
    if removed.is_empty() {
        "; nothing was removed".to_string()
    } else {
        let paths: Vec<String> = removed.iter().map(|path| format!("`{}`", path.display())).collect();
        format!("; already removed: {}", paths.join(", "))
    }
}

/// Removes the retired format 1 registration: exactly
/// [`legacy_manifest_path`] and the tree at [`legacy_revisions_dir`], and
/// nothing else under `~/.claudine`. Setup and doctor's repair call it only
/// after the `register` entry naming the old manifest was replaced.
///
/// `~/.claudine`, the manifest, and every entry of the revision tree are
/// examined without following links before anything is removed, and each
/// entry again just before it is removed. A symbolic link, a junction, or
/// another reparse point anywhere is [`CleanupError::Obstructed`]: nothing
/// (or, for one created during the cleanup, nothing more) is removed, and
/// nothing is followed. The next cleanup removes only what is left.
///
/// ## Errors
///
/// [`CleanupFailure`]: why the cleanup stopped and what it removed first.
/// An obstruction or unexpected entry found before anything is removed
/// leaves everything in place.
pub fn remove_legacy_registration(home: &Path) -> Result<LegacyCleanup, CleanupFailure> {
    let mut removed = Vec::new();
    let result = remove_into(home, &mut removed);
    // Children are removed before their folder, so a folder's descendants
    // precede it; only the outermost removed entries are reported.
    let outermost: Vec<PathBuf> = removed
        .iter()
        .filter(|path| !removed.iter().any(|other| other != *path && path.starts_with(other)))
        .cloned()
        .collect();
    match result {
        Ok(()) => Ok(LegacyCleanup { removed: outermost }),
        Err(error) => Err(CleanupFailure { removed: outermost, error }),
    }
}

/// The removal itself, appending each entry to `removed` as it is removed.
fn remove_into(home: &Path, removed: &mut Vec<PathBuf>) -> Result<(), CleanupError> {
    let plan = legacy_plan(home)?;
    for (path, is_directory) in &plan.tree {
        match entry_at(path) {
            Ok(Entry::Obstruction(obstruction)) => return Err(CleanupError::Obstructed { path: path.clone(), obstruction }),
            Ok(Entry::Other) => return Err(CleanupError::Unexpected { path: path.clone() }),
            _ => {}
        }
        access_log::note("remove", path);
        match remove_entry(path, *is_directory) {
            Ok(()) => removed.push(path.clone()),
            // Another cleanup got there first.
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(CleanupError::Io { action: "remove", path: path.clone(), source }),
        }
    }
    if let Some(manifest) = plan.manifest {
        match entry_at(&manifest) {
            Ok(Entry::Obstruction(obstruction)) => return Err(CleanupError::Obstructed { path: manifest, obstruction }),
            Ok(Entry::Other | Entry::Directory) => return Err(CleanupError::Unexpected { path: manifest }),
            _ => {}
        }
        access_log::note("remove", &manifest);
        match remove_entry(&manifest, false) {
            Ok(()) => removed.push(manifest),
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(CleanupError::Io { action: "remove", path: manifest, source }),
        }
    }
    Ok(())
}

/// What [`remove_legacy_registration`] would remove, found by its no-follow
/// examination and nothing else.
#[derive(Debug, Default)]
pub(super) struct LegacyPlan {
    /// The retired manifest, when it is a regular file.
    pub(super) manifest: Option<PathBuf>,
    /// Every entry of the revision tree, children before their folder, each
    /// marked `true` for a directory; empty when there is no tree.
    pub(super) tree: Vec<(PathBuf, bool)>,
}

/// Examines the retired registration under `home` without following links
/// or changing anything.
///
/// ## Errors
///
/// As [`remove_legacy_registration`], for what it would refuse.
pub(super) fn legacy_plan(home: &Path) -> Result<LegacyPlan, CleanupError> {
    let inspect = |path: &Path| match entry_at(path) {
        Ok(Entry::Obstruction(obstruction)) => Err(CleanupError::Obstructed { path: path.to_path_buf(), obstruction }),
        Ok(entry) => Ok(entry),
        Err(PublishError::Io { action, path, source }) => Err(CleanupError::Io { action, path, source }),
        Err(_) => Err(CleanupError::Unexpected { path: path.to_path_buf() }),
    };
    let mut plan = LegacyPlan::default();
    if inspect(&home.join(CLAUDINE_DIR))? != Entry::Directory {
        return Ok(plan);
    }
    let manifest = legacy_manifest_path(home);
    match inspect(&manifest)? {
        Entry::Absent => {}
        Entry::File => plan.manifest = Some(manifest),
        _ => return Err(CleanupError::Unexpected { path: manifest }),
    }
    let revisions = legacy_revisions_dir(home);
    match inspect(&revisions)? {
        Entry::Absent => {}
        Entry::Directory => collect_tree(&revisions, &inspect, &mut plan.tree)?,
        _ => return Err(CleanupError::Unexpected { path: revisions }),
    }
    Ok(plan)
}

/// Every entry below and including the directory `directory`, children
/// first, each marked `true` for a directory. A directory removed while it
/// is listed (by a concurrent cleanup) is left out.
fn collect_tree(
    directory: &Path,
    inspect: &dyn Fn(&Path) -> Result<Entry, CleanupError>,
    tree: &mut Vec<(PathBuf, bool)>,
) -> Result<(), CleanupError> {
    let io = |source| CleanupError::Io { action: "read", path: directory.to_path_buf(), source };
    access_log::note("list", directory);
    let children = match fs::read_dir(directory) {
        Ok(children) => children,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io(error)),
    };
    for child in children {
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
