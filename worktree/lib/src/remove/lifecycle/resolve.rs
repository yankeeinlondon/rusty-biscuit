//! Which removal a name means: a live worktree, a pending removal, or both.
//!
//! A retry is typed with the name the user removed, which Git may no longer
//! register (the registration moved to the quarantine, or is gone). The name
//! is resolved against the union of live registrations, as
//! [`resolve_worktree`](crate::worktree::resolve_worktree) matches them, and
//! pending records, by their requested name, saved aliases, and original
//! directory name. A live registration and a pending record are the same
//! target only when both identify this operation's checkout: the
//! quarantine's own registration, or the unmoved checkout at the original
//! path with the recorded identity. Anything else that matches twice is
//! ambiguous and refused, never resolved by picking one. Saved names are
//! lookup keys only; they grant no filesystem authority.

use std::path::{Path, PathBuf};

use super::{Presence, presence};
use crate::error::WorktreeError;
use crate::remove::admin_entry::{common_git_dir, same_location};
use crate::remove::recovery::OperationId;
use crate::remove::recovery::record::RecoveryRecord;
use crate::remove::recovery::store::{RecoveryStore, STORE_DIR, StoreError};
use crate::util::dasherize;
use crate::worktree::{WorktreeEntry, parse_worktree_list};

/// A pending removal a name resolved to.
#[derive(Debug, Clone)]
pub struct PendingTarget {
    pub id: OperationId,
    pub record: RecoveryRecord,
}

/// One of several targets a name matched.
#[derive(Debug, Clone)]
pub enum Candidate {
    Live(WorktreeEntry),
    Pending { id: OperationId, original: PathBuf, quarantine: PathBuf },
}

/// What a name resolved to.
#[derive(Debug, Clone)]
pub enum Target {
    Live(WorktreeEntry),
    Pending(Box<PendingTarget>),
    /// Refuse and list them; never pick one.
    Ambiguous(Vec<Candidate>),
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("no worktree or pending removal is named '{0}'")]
    NotFound(String),
    #[error(transparent)]
    Git(WorktreeError),
    #[error(transparent)]
    Store(StoreError),
}

/// Resolves `name` from the base checkout `base`. `base` itself is always
/// the main checkout and never a pending removal.
///
/// ## Errors
///
/// [`ResolveError::NotFound`] when nothing matches; Git's or the store's
/// failure to list. A pending record that cannot be read matches nothing
/// here; starting any removal of the repository refuses while it exists.
pub fn resolve_target(base: &Path, name: &str) -> Result<Target, ResolveError> {
    let listing = crate::git::worktree_list_from(base, base).map_err(ResolveError::Git)?;
    resolve_target_in(base, parse_worktree_list(&listing), name)
}

/// [`resolve_target`] against `entries`, a `git worktree list` the caller
/// has just read (the CLI reads one to find `base` and resolves from it).
///
/// ## Errors
///
/// As [`resolve_target`], except that nothing is listed here.
pub fn resolve_target_in(base: &Path, entries: Vec<WorktreeEntry>, name: &str) -> Result<Target, ResolveError> {
    if name == "base" {
        return entries.into_iter().find(|entry| entry.is_main).map(Target::Live).ok_or_else(|| ResolveError::NotFound(name.into()));
    }
    let pending = pending_records(base)?;

    let dasherized = dasherize(name);
    let names_match = |candidate: &str| candidate == name || candidate == dasherized;
    let mut matched: Vec<&PendingTarget> = pending
        .iter()
        .filter(|target| {
            let record = &target.record;
            let directory = record.original.as_path().file_name().map(|dir| dir.to_string_lossy().into_owned());
            record.requested_name == name || record.aliases.iter().any(|alias| alias == name) || directory.as_deref().is_some_and(names_match)
        })
        .collect();
    let mut live = Vec::new();
    for entry in &entries {
        let basename = entry.path.file_name().map(|dir| dir.to_string_lossy().into_owned());
        let matches = entry.branch.as_deref() == Some(name) || (!entry.is_main && basename.as_deref().is_some_and(names_match));
        if !matches || live.iter().any(|seen: &&WorktreeEntry| seen.path == entry.path) {
            continue;
        }
        match pending.iter().find(|target| same_operation(entry, &target.record)) {
            Some(target) => {
                if !matched.iter().any(|seen| seen.id == target.id) {
                    matched.push(target);
                }
            }
            None => live.push(entry),
        }
    }

    match (live.as_slice(), matched.as_slice()) {
        ([], []) => Err(ResolveError::NotFound(name.into())),
        ([entry], []) => Ok(Target::Live((*entry).clone())),
        ([], [target]) => Ok(Target::Pending(Box::new((*target).clone()))),
        _ => Ok(Target::Ambiguous(
            live.iter()
                .map(|entry| Candidate::Live((*entry).clone()))
                .chain(matched.iter().map(|target| Candidate::Pending {
                    id: target.id.clone(),
                    original: target.record.original.as_path().to_path_buf(),
                    quarantine: target.record.quarantine.as_path().to_path_buf(),
                }))
                .collect(),
        )),
    }
}

/// Every readable pending record of the repository. Reading never creates
/// the store.
fn pending_records(base: &Path) -> Result<Vec<PendingTarget>, ResolveError> {
    let common = common_git_dir(base).map_err(ResolveError::Git)?;
    if matches!(std::fs::symlink_metadata(common.join(STORE_DIR)), Err(error) if error.kind() == std::io::ErrorKind::NotFound) {
        return Ok(Vec::new());
    }
    let store = RecoveryStore::open(&common).map_err(ResolveError::Store)?;
    Ok(store
        .pending()
        .map_err(ResolveError::Store)?
        .into_iter()
        .filter_map(|pending| pending.record.ok().map(|record| PendingTarget { id: pending.id, record }))
        .collect())
}

/// Whether the live registration `entry` is the checkout `record` moves:
/// its quarantine, or the unmoved checkout itself at the original path.
fn same_operation(entry: &WorktreeEntry, record: &RecoveryRecord) -> bool {
    same_location(&entry.path, record.quarantine.as_path())
        || (same_location(&entry.path, record.original.as_path())
            && presence(&entry.path, &record.checkout_identity) == Presence::Checkout)
}
