//! Which removal a name means: a live worktree, a pending confirmed removal
//! (or one an earlier `wt` saved), or both.
//!
//! A pending removal is found by its requested name, its saved aliases, and
//! its original directory's name; a live registration by its branch and
//! directory name. The two are one target only when the live registration
//! is the pending operation's own checkout: the moved checkout Git now lists
//! at the hidden path, or the unmoved checkout at the original path with the
//! recorded identity. Anything else that matches twice is ambiguous and
//! refused, never resolved by picking one; a saved name is a lookup key and
//! grants no filesystem authority.

use std::path::{Path, PathBuf};

use sniff::filesystem::query::FileIdentity;

use crate::availability::{self, EntryKind};
use crate::error::WorktreeError;
use crate::remove::admin_entry::{common_git_dir, same_location};
use crate::remove::quarantine::rename;
use crate::remove::recovery::OperationId;
use crate::remove::recovery::store::Stored;
use crate::remove::recovery::store::{RecoveryStore, STORE_DIR, StoreError};
use crate::util::dasherize;
use crate::worktree::{WorktreeEntry, parse_worktree_list};

/// What a name resolved to.
#[derive(Debug, Clone)]
pub enum Resolved {
    Live(WorktreeEntry),
    /// Finish it with [`finish_pending`](super::finish_pending).
    Pending(OperationId),
    /// Refuse and list them; never pick one.
    Ambiguous(Vec<Candidate>),
}

/// One of several targets a name matched.
#[derive(Debug, Clone)]
pub enum Candidate {
    Live(WorktreeEntry),
    Pending { id: OperationId, original: PathBuf },
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

/// Resolves `name` from the base checkout `base`.
///
/// ## Errors
///
/// [`ResolveError::NotFound`] when nothing matches; Git's or the store's
/// failure to list. A record that cannot be read matches nothing; saving a
/// new approval refuses while it exists.
pub fn resolve(base: &Path, name: &str) -> Result<Resolved, ResolveError> {
    let listing = crate::git::worktree_list_from(base, base).map_err(ResolveError::Git)?;
    resolve_in(base, parse_worktree_list(&listing), name)
}

/// [`resolve`] against `entries`, a `git worktree list` the caller has just
/// read.
///
/// ## Errors
///
/// As [`resolve`], except that nothing is listed here.
pub fn resolve_in(base: &Path, entries: Vec<WorktreeEntry>, name: &str) -> Result<Resolved, ResolveError> {
    if name == "base" {
        return entries.into_iter().find(|entry| entry.is_main).map(Resolved::Live).ok_or_else(|| ResolveError::NotFound(name.into()));
    }
    let pending = pending_operations(base)?;
    let dasherized = dasherize(name);
    let names_match = |candidate: &str| candidate == name || candidate == dasherized;

    let mut matched: Vec<&Pending> = pending
        .iter()
        .filter(|op| {
            let directory = op.original.file_name().map(|dir| dir.to_string_lossy().into_owned());
            op.requested_name == name || op.aliases.iter().any(|alias| alias == name) || directory.as_deref().is_some_and(names_match)
        })
        .collect();
    let mut live: Vec<&WorktreeEntry> = Vec::new();
    for entry in &entries {
        let basename = entry.path.file_name().map(|dir| dir.to_string_lossy().into_owned());
        let matches = entry.branch.as_deref() == Some(name) || (!entry.is_main && basename.as_deref().is_some_and(names_match));
        if !matches || live.iter().any(|seen| seen.path == entry.path) {
            continue;
        }
        match pending.iter().find(|op| op.is_checkout_of(entry)) {
            Some(op) if !matched.iter().any(|seen| seen.id == op.id) => matched.push(op),
            Some(_) => {}
            None => live.push(entry),
        }
    }

    match (live.as_slice(), matched.as_slice()) {
        ([], []) => Err(ResolveError::NotFound(name.into())),
        ([entry], []) => Ok(Resolved::Live((*entry).clone())),
        ([], [op]) => Ok(Resolved::Pending(op.id.clone())),
        _ => Ok(Resolved::Ambiguous(
            live.iter()
                .map(|entry| Candidate::Live((*entry).clone()))
                .chain(matched.iter().map(|op| Candidate::Pending { id: op.id.clone(), original: op.original.clone() }))
                .collect(),
        )),
    }
}

/// What resolution reads of a stored operation, in either format.
struct Pending {
    id: OperationId,
    requested_name: String,
    aliases: Vec<String>,
    original: PathBuf,
    /// Where Git lists the checkout once it is moved (the original when it is
    /// deleted in place).
    deletion_path: PathBuf,
    identity: FileIdentity,
}

impl Pending {
    fn of(id: OperationId, stored: Stored) -> Self {
        match stored {
            Stored::Compact(record) => Self {
                id,
                requested_name: record.requested_name,
                aliases: record.aliases,
                original: record.original.as_path().to_path_buf(),
                deletion_path: record.deletion_path.as_path().to_path_buf(),
                identity: record.checkout_identity,
            },
            Stored::Legacy(legacy) => Self {
                id,
                requested_name: legacy.record.requested_name,
                aliases: legacy.record.aliases,
                original: legacy.record.original.as_path().to_path_buf(),
                deletion_path: legacy.record.quarantine.as_path().to_path_buf(),
                identity: legacy.record.checkout_identity,
            },
        }
    }

    /// Whether the live registration `entry` is this operation's checkout:
    /// at its deletion path, or at the original path when that holds the
    /// unmoved checkout itself or nothing (Git still lists the original
    /// after a crash between the rename and its record, and after the
    /// checkout vanished). Anything else at the original is not this
    /// operation's.
    fn is_checkout_of(&self, entry: &WorktreeEntry) -> bool {
        let at = |path: &Path| same_location(&entry.path, path);
        let unmoved_or_gone = || match availability::inspect(&entry.path) {
            Ok(EntryKind::Directory) => rename::identity(&entry.path).is_ok_and(|identity| identity == self.identity),
            Err(error) => error.kind() == std::io::ErrorKind::NotFound,
            Ok(_) => false,
        };
        (at(&self.deletion_path) && self.deletion_path != self.original) || (at(&self.original) && unmoved_or_gone())
    }
}

/// Every readable stored operation of the repository. Reading never creates
/// the store.
fn pending_operations(base: &Path) -> Result<Vec<Pending>, ResolveError> {
    let common = common_git_dir(base).map_err(ResolveError::Git)?;
    if matches!(std::fs::symlink_metadata(common.join(STORE_DIR)), Err(error) if error.kind() == std::io::ErrorKind::NotFound) {
        return Ok(Vec::new());
    }
    let store = RecoveryStore::open(&common).map_err(ResolveError::Store)?;
    Ok(store
        .operations()
        .map_err(ResolveError::Store)?
        .into_iter()
        .filter_map(|op| op.record.ok().map(|stored| Pending::of(op.id, stored)))
        .collect())
}
