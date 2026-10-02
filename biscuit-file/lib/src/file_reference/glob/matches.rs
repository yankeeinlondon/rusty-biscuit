//! Lexical membership of one path.

use std::path::{Path, PathBuf};

use super::list::PreparedSet;
use super::roots::canonical_prefix;
use crate::file_reference::PathIdentity;

impl PreparedSet {
    /// Whether the set admits `path`, which need not exist.
    ///
    /// The path's directory is spelled canonically up to its longest
    /// existing prefix ([`canonical_prefix`]); the file name is kept, so a
    /// file symlink is judged where it sits, as a walk would find it. The
    /// judgment itself is containment and glob checks only: no walk.
    pub(crate) fn matches(&self, path: &Path, cwd: &Path) -> bool {
        self.admitting_path(path, cwd).is_some()
    }

    /// Whether a listing that reached the existing file `path` would list
    /// it: [`matches`](Self::matches), less a file symlink the listing skips
    /// because its target leaves the tree.
    pub(crate) fn lists(&self, path: &Path, cwd: &Path) -> bool {
        let Some((admitting, absolute)) = self.admitting_path(path, cwd) else {
            return false;
        };
        let is_link = std::fs::symlink_metadata(&absolute).is_ok_and(|meta| meta.file_type().is_symlink());
        !(is_link && self.escaping_target(admitting, &absolute).is_some())
    }

    /// The admitting pattern and `path` made absolute, when the set admits it.
    fn admitting_path(&self, path: &Path, cwd: &Path) -> Option<(usize, PathBuf)> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        let absolute = crate::file_reference::resolve::normalize_components(&absolute);
        let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) else {
            return None;
        };
        let file = PathIdentity::new(&canonical_prefix(parent).join(name));
        let admitting = self.admitting(&file, name)?;
        Some((admitting, absolute))
    }
}
