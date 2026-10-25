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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sniff::filesystem::query::FileIdentity;

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
/// new approval refuses while it exists. When nothing matches and such a
/// record exists, its [`ResolveError::Store`] explains why instead, since the
/// name may well be that operation's.
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
    let (pending, unreadable) = pending_operations(base)?;
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
        ([], []) => Err(unreadable.map_or_else(|| ResolveError::NotFound(name.into()), ResolveError::Store)),
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

/// The checkouts a `wt remove` has started and not finished, by the path
/// Git lists each at, with the name its removal was started with. Other
/// commands leave these alone: `wt list` runs no `git status` in one, and
/// `--ff` never moves files in one.
#[derive(Debug, Clone, Default)]
pub struct PendingRemovals(HashMap<PathBuf, String>);

impl PendingRemovals {
    /// The entries of `entries` that a stored operation of `base`'s
    /// repository is removing, matched as [`resolve_in`] matches them. Best
    /// effort: a store or record that cannot be read marks nothing.
    pub fn read(base: &Path, entries: &[WorktreeEntry]) -> Self {
        let Ok((pending, _)) = pending_operations(base) else { return Self::default() };
        Self(
            entries
                .iter()
                .filter_map(|entry| {
                    let op = pending.iter().find(|op| op.is_checkout_of(entry))?;
                    Some((entry.path.clone(), op.requested_name.clone()))
                })
                .collect(),
        )
    }

    /// The name the removal of the checkout Git lists at `path` was started
    /// with, when one is pending.
    pub fn name_for(&self, path: &Path) -> Option<&str> {
        self.0.get(path).map(String::as_str)
    }
}

impl FromIterator<(PathBuf, String)> for PendingRemovals {
    fn from_iter<I: IntoIterator<Item = (PathBuf, String)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
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
    /// What the operation approved at `original`; `None` when nothing was
    /// there.
    identity: Option<FileIdentity>,
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
                identity: record.target.original_identity(),
            },
            Stored::Legacy(legacy) => Self {
                id,
                requested_name: legacy.record.requested_name,
                aliases: legacy.record.aliases,
                original: legacy.record.original.as_path().to_path_buf(),
                deletion_path: legacy.record.quarantine.as_path().to_path_buf(),
                identity: Some(legacy.record.checkout_identity),
            },
        }
    }

    /// Whether the live registration `entry` is this operation's checkout:
    /// at its deletion path, or at the original path when that holds the
    /// approved object itself (the unmoved checkout, the link, the file) or
    /// nothing (Git still lists the original after a crash between the
    /// rename and its record, and after the checkout vanished). Anything
    /// else at the original is not this operation's.
    fn is_checkout_of(&self, entry: &WorktreeEntry) -> bool {
        let at = |path: &Path| same_location(&entry.path, path);
        let unmoved_or_gone = || match rename::identity(&entry.path) {
            Ok(identity) => self.identity == Some(identity),
            Err(error) => error.kind() == std::io::ErrorKind::NotFound,
        };
        (at(&self.deletion_path) && self.deletion_path != self.original) || (at(&self.original) && unmoved_or_gone())
    }
}

/// Every readable stored operation of the repository with something left to
/// finish, and the first record that could not be read. Reading never creates
/// the store; a legacy operation whose every step finished is deleted
/// instead ([`RecoveryStore::discard_completed`]).
fn pending_operations(base: &Path) -> Result<(Vec<Pending>, Option<StoreError>), ResolveError> {
    let common = common_git_dir(base).map_err(ResolveError::Git)?;
    if matches!(std::fs::symlink_metadata(common.join(STORE_DIR)), Err(error) if error.kind() == std::io::ErrorKind::NotFound) {
        return Ok((Vec::new(), None));
    }
    let store = RecoveryStore::open(&common).map_err(ResolveError::Store)?;
    let mut pending = Vec::new();
    let mut unreadable = None;
    for op in store.operations().map_err(ResolveError::Store)? {
        match op.record {
            // Nothing is left to finish; an earlier `wt` kept its record and
            // report after finishing, and only a resume would delete them.
            Ok(Stored::Legacy(legacy)) if legacy.is_completed() => store.discard_completed(&op.id, &legacy),
            Ok(stored) => pending.push(Pending::of(op.id, stored)),
            Err(error) => {
                unreadable.get_or_insert(error);
            }
        }
    }
    Ok((pending, unreadable))
}
