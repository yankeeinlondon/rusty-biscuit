//! Lexical path normalization shared by context validation and resolution.
//! Nothing here reads the filesystem.

use std::path::{Path, PathBuf};

use crate::file_reference::portable::normalize_native;

/// Strip a Windows `\\?\` verbatim prefix from a search root. No-op elsewhere.
///
/// Roots arrive in whichever spelling the caller produced: `std::fs::canonicalize`
/// yields a verbatim path, while `gix` worktree discovery and a home read from
/// the environment usually yield the legacy form. Both spellings must be collapsed *before* anything is joined
/// onto a root, because Win32 performs no path normalization under a verbatim
/// prefix -- a reference's own `/` separators would stay literal filename
/// characters and every probe would miss. Collapsing here also makes two
/// spellings of one directory compare equal, which is what lets candidate
/// dedupe fold them into a single entry.
///
/// [`dunce::simplified`] declines to strip when the legacy form would not be
/// equivalent (reserved DOS names, over-long paths), so a genuinely
/// verbatim-only path survives intact.
pub(crate) fn simplify_root(root: &Path) -> &Path {
    dunce::simplified(root)
}

/// Collapse `.` and `..` by the shared [`PathIdentity`](super::PathIdentity) rules without touching
/// the filesystem, then reduce a Windows `\\?\` verbatim path to its legacy
/// spelling.
///
/// Every lexical path comparison in resolution and context selection funnels
/// through here, so an excess `..` at a root is dropped exactly as identity
/// comparison drops it, and dot segments under `\\?\` stay literal names.
/// Stripping the prefix is what lets a verbatim and a legacy spelling of one
/// directory compare equal -- required for candidate dedupe, repository
/// containment, and `diff_paths`' common-prefix walk. [`dunce::simplified`]
/// keeps the prefix on a verbatim path that carries literal dot segments,
/// because no legacy spelling names the same directory.
pub(crate) fn normalize_components(path: &Path) -> PathBuf {
    simplify_root(&normalize_native(path)).to_path_buf()
}
