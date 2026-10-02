//! Lexical membership of one path.

use std::path::Path;

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
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        let absolute = crate::file_reference::resolve::normalize_components(&absolute);
        let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) else {
            return false;
        };
        let file = PathIdentity::new(&canonical_prefix(parent).join(name));
        self.admitting(&file, name).is_some()
    }
}
