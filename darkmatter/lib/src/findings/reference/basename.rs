//! The bounded repository search for files that share a reference's file
//! name: the recovery a host offers when an explicit reference matched
//! nothing and the nearby directories hold no close name.
//!
//! The host decides which subtrees are worth reading (its `keep` filter) and
//! which file name to look for; this module owns the walk and its bounds.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use biscuit_file::to_forward_slash_string;
use ignore::WalkBuilder;

/// How much of a repository one basename search may read, and how many
/// matches it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasenameBounds {
    /// The most walk items read, counting the root and unreadable entries,
    /// so a huge or broken tree costs the same as a large healthy one.
    pub entry_budget: usize,
    /// The most matches returned.
    pub max_matches: usize,
}

impl BasenameBounds {
    /// The entry budget hosts use unless they have a reason not to.
    pub const ENTRY_BUDGET: usize = 20_000;
}

/// One item of the walk, as the collector counts it.
enum WalkItem {
    Root,
    Error,
    Entry { relative_path: PathBuf, file_name: OsString, is_file: bool },
}

/// Files below `root` whose whole file name is `file_name`, as
/// root-relative paths written with `/`, sorted, at most
/// `bounds.max_matches`.
///
/// The walk honors `.gitignore` (and the global and `info/exclude` rules),
/// includes hidden entries, and never follows a directory symlink; a file
/// symlink counts when its target is a file. `keep` is asked about every
/// entry below the root by its name, and an entry it rejects is skipped
/// with everything beneath it. The result is advisory: an unusable root
/// gives no matches, an unreadable entry is skipped (it still spends the
/// budget), and reaching the budget keeps the matches already found. The
/// walk visits entries in path order, so the same tree always gives the same
/// matches.
pub fn basename_matches<F>(root: &Path, file_name: &OsStr, bounds: BasenameBounds, keep: F) -> Vec<String>
where
    F: Fn(&OsStr) -> bool + Send + Sync + 'static,
{
    if file_name.is_empty() || bounds.max_matches == 0 || !root.is_dir() {
        return Vec::new();
    }
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .follow_links(false)
        .filter_entry(move |entry| entry.depth() == 0 || keep(entry.file_name()))
        .sort_by_file_path(|left, right| left.cmp(right));
    let items = builder.build().map(|result| match result {
        Ok(entry) if entry.depth() == 0 => WalkItem::Root,
        Ok(entry) => {
            let Ok(relative_path) = entry.path().strip_prefix(root).map(Path::to_path_buf) else {
                return WalkItem::Error;
            };
            let is_file = match entry.file_type() {
                Some(kind) if kind.is_file() => true,
                Some(kind) if kind.is_symlink() => match std::fs::metadata(entry.path()) {
                    Ok(metadata) => metadata.is_file(),
                    Err(_) => return WalkItem::Error,
                },
                _ => false,
            };
            WalkItem::Entry { relative_path, file_name: entry.file_name().to_os_string(), is_file }
        }
        Err(_) => WalkItem::Error,
    });
    collect(file_name, bounds, items)
}

fn collect(file_name: &OsStr, bounds: BasenameBounds, items: impl IntoIterator<Item = WalkItem>) -> Vec<String> {
    let mut matches = BTreeSet::new();
    for item in items.into_iter().take(bounds.entry_budget) {
        if let WalkItem::Entry { relative_path, file_name: name, is_file: true } = item
            && name == file_name
        {
            matches.insert(to_forward_slash_string(&relative_path));
            if matches.len() == bounds.max_matches {
                break;
            }
        }
    }
    matches.into_iter().collect()
}
