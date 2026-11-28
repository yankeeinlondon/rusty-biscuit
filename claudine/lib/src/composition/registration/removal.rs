//! The registration module's removals: the one removal primitive (and the
//! test seam that makes a chosen removal fail), leftover temporary files,
//! and folders a retired member leaves empty.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{Entry, Layout, PublishError, RegistrationInventory, TEMP_PREFIX, access_log, entry_at, io_error, one_entry};

/// Removes the regular file (or, with `directory`, the empty folder) at
/// `path`: every removal of this module goes through it, so a test can make
/// one fail ([`fail_removals`]).
pub(super) fn remove_entry(path: &Path, directory: bool) -> io::Result<()> {
    #[cfg(test)]
    if removal_fails(path) {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    if directory { fs::remove_dir(path) } else { fs::remove_file(path) }
}

#[cfg(test)]
thread_local! {
    static FAILING_REMOVALS: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn removal_fails(path: &Path) -> bool {
    FAILING_REMOVALS.with(|failing| failing.borrow().iter().any(|failing| failing == path))
}

/// Makes every removal of one of `paths` on this thread fail with
/// `PermissionDenied`, as a read-only parent folder would, until the guard
/// is dropped.
#[cfg(test)]
pub(crate) fn fail_removals(paths: impl IntoIterator<Item = PathBuf>) -> FailingRemovals {
    FAILING_REMOVALS.with(|failing| failing.borrow_mut().extend(paths));
    FailingRemovals
}

/// Restores ordinary removals when dropped; see [`fail_removals`].
#[cfg(test)]
pub(crate) struct FailingRemovals;

#[cfg(test)]
impl Drop for FailingRemovals {
    fn drop(&mut self) {
        FAILING_REMOVALS.with(|failing| failing.borrow_mut().clear());
    }
}

/// Removes every regular file named with [`TEMP_PREFIX`] directly in
/// `directory`, when it exists, appending each to `removed` as it goes, so
/// a later failure keeps the earlier removals.
pub(super) fn remove_temps(directory: &Path, removed: &mut Vec<PathBuf>) -> Result<(), PublishError> {
    access_log::note("list", directory);
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_error("list", directory)(error)),
    };
    for entry in entries {
        let entry = entry.map_err(io_error("list", directory))?;
        if !entry.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
            continue;
        }
        let path = entry.path();
        if entry_at(&path)? != Entry::File {
            continue;
        }
        access_log::note("remove", &path);
        match remove_entry(&path, false) {
            Ok(()) => removed.push(path),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("remove", &path)(error)),
        }
    }
    Ok(())
}

/// Removes the folders of the retired `path` that no member needs and that
/// are now empty, appending each to `removed`; a folder that still holds
/// anything stays.
pub(super) fn remove_empty_parents(layout: &Layout, path: &str, inventory: &RegistrationInventory, removed: &mut Vec<String>) {
    let mut folder = path;
    while let Some((parent, _)) = folder.rsplit_once('/') {
        let prefix = format!("{parent}/");
        if inventory.files().iter().any(|file| file.path().starts_with(&prefix)) {
            return;
        }
        let destination = layout.destination(parent);
        // A differently spelled folder a member needs, on a case-insensitive
        // volume.
        let needed = inventory.files().iter().any(|file| {
            let mut member = file.path();
            while let Some((folder, _)) = member.rsplit_once('/') {
                if one_entry(&destination, &layout.destination(folder)).unwrap_or(true) {
                    return true;
                }
                member = folder;
            }
            false
        });
        if needed {
            return;
        }
        access_log::note("remove", &destination);
        if remove_entry(&destination, true).is_err() {
            return;
        }
        removed.push(parent.to_string());
        folder = parent;
    }
}
