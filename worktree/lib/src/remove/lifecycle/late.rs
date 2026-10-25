//! R14: the uncommitted files a confirmed removal deletes that its question
//! did not show.
//!
//! Approval covers whatever the checkout holds when deletion reaches it, so
//! nothing found here stops the removal. Just before deletion, `git status`
//! runs once more and every dirty path the question's own `git status` did
//! not list is named in the final line. It covers only what `git status`
//! reports (never ignored files such as `target/`, nor anything written
//! after it runs), so the list never claims to be complete.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::remove::inventory::{Inventory, collect_inventory};

/// The dirty paths the question listed from `inventory`, for [`unseen`]; the
/// shell wrapper's handoff carries them to its second run. `Some(empty)`
/// means the question showed no dirty files; `None` means it showed no
/// inventory at all (Git could not read the checkout), so nothing is named.
pub fn shown(inventory: Option<&Inventory>) -> Option<Vec<PathBuf>> {
    inventory.map(|inventory| inventory.dirty.iter().map(|entry| entry.path.clone()).collect())
}

/// The dirty paths of the checkout at `checkout` that `shown` (from
/// [`shown`]) did not list, checkout-relative, in Git's order. Empty when no
/// question listed files to compare with, or when `git status` fails now;
/// neither blocks deletion.
pub fn unseen(base: &Path, checkout: &Path, shown: Option<&[PathBuf]>) -> Vec<PathBuf> {
    let Some(shown) = shown else { return Vec::new() };
    let Ok(now) = collect_inventory(base, checkout) else { return Vec::new() };
    let shown: HashSet<&Path> = shown.iter().map(PathBuf::as_path).collect();
    now.dirty.into_iter().filter(|entry| !shown.contains(entry.path.as_path())).map(|entry| entry.path).collect()
}
