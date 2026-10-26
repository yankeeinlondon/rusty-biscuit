//! A removal the user confirmed (or forced): after "yes", the checkout is
//! deleted. Nothing about its content stops it; only the boundaries that
//! keep deletion inside the approved checkout, a permanent refusal by the
//! operating system, and the user's own Ctrl-C do.
//!
//! [`remove`] runs the steps in the order of the flowchart in
//! `2026-10-22-confirmed-remove-always-deletes`, each recorded in the
//! operation's compact record ([`CompactRecord`]) before the next starts:
//!
//! 1. **Save the approval.** The record binds the objects the question
//!    showed ([`ApprovedTarget`]), the repository, the worktree's Git record,
//!    and the approved branch steps, each by the identity
//!    [`gather`](super::facts::gather) read before the question; anything at
//!    those paths with another identity now refuses ([`Refusal::Replaced`]).
//!    The record is durable, and the operation's lock held, before anything
//!    is unlocked, moved, deleted, or unregistered; when it cannot be saved
//!    nothing is changed ([`Refusal::NotSaved`]).
//! 2. **Unlock** a locked worktree, with `git worktree unlock`, when the
//!    question showed the lock.
//! 3. **Place** the checkout: move it to its hidden quarantine, or mark it
//!    for deletion in place where a move is unsafe (a nested mount, a
//!    filesystem without a no-replacement rename, a held file on Windows, a
//!    `.git` link that cannot be kept).
//! 4. Run `git status` once more ([`late`]) to name uncommitted files the
//!    question did not show; this never blocks.
//! 5. **Delete** the placed checkout with the plain engine
//!    ([`delete_tree`](super::delete::delete_tree)), waiting out temporary
//!    contention. A nested mount or a permanent failure keeps the record, the
//!    registration, and the branches until a later `wt remove` finishes.
//! 6. **Unregister** with plain `git worktree remove` of the now absent
//!    checkout (which deletes the worktree's Git record, its submodules'
//!    `modules/` included), then the copy record.
//! 7. **Branch steps** ([`branches`]), each bound to its approved tip,
//!    endpoint, and lease; a changed branch is kept and the run exits 1.
//! 8. **Finish**: the record is deleted, so a completed removal leaves
//!    nothing.
//!
//! A worktree that is not a checkout directory takes the same steps without
//! placement: a link to its own checkout deletes that checkout in place and
//! then the link; a file or a link to anything else is removed as itself;
//! a checkout already gone has nothing to delete. A path that could not be
//! identified before the question is not removed at all.
//!
//! An interrupted operation is finished by [`resume::finish_pending`], which
//! reads the paths and the registration as they are rather than trusting
//! the record's progress marker.

pub mod branches;
pub mod late;
pub mod resolve;
pub mod resume;
#[cfg(test)]
mod tests;

pub use branches::{BranchOutcome, BranchResult, Revalidate};
pub use resolve::{Candidate, PendingRemovals, Resolved, ResolveError, resolve, resolve_in};
pub use resume::{finish_handoff, finish_pending};

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sniff::filesystem::query::FileIdentity;

use crate::availability::{self, EntryKind};
use crate::error::WorktreeError;
use crate::worktree::WorktreeEntry;

use super::admin_entry::{AdminEntry, BackReferenceError, admin_entry_in, common_git_dir, read_back_reference, same_location};
use super::clock::Clock;
use super::delete::{self, Deletion, Failure, UsageHolders, Wait};
use super::facts::{self, Facts, LinkTarget, Target, Warning};
use super::faults::FsFaults;
use super::mounts::{Mount, MountInfo};
use super::quarantine::association::{Faults, MoveError, MoveErrorClass, MovePlan, MoveRefusal, plan_move};
use super::quarantine::placement::{Placement, in_place_for_refusal, place_with};
use super::quarantine::rename;
use super::recovery::OperationId;
use super::recovery::compact::{
    ApprovedTarget, BranchAction, BranchStep, COMPACT_RECORD_VERSION, CompactRecord, DeletionMode, Progress, StepApproval, StepState,
};
use super::recovery::lock::{LockError, OperationLock};
use super::recovery::record::RepositoryBinding;
use super::recovery::store::{PendingRemoval, RecoveryStore, StoreError};
use super::usage::UsageSource;

/// One branch deletion the user approved, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedBranch {
    /// [`BranchAction::Local`] or [`BranchAction::Remote`].
    pub action: BranchAction,
    pub approval: StepApproval,
}

/// One confirmed removal of `entry`, as the question described it.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The base (main) checkout, from which every Git command runs.
    pub base: &'a Path,
    pub entry: &'a WorktreeEntry,
    /// The name the user typed; saved so a later run finds the operation.
    pub requested_name: &'a str,
    /// What the question showed ([`gather`](super::facts::gather)). Its
    /// target decides the steps, its identities bind what they act on, its
    /// lock warning approves unlocking, and its inventory is what [`late`]
    /// compares against.
    pub facts: &'a Facts,
    /// Local before remote; at most one of each.
    pub branches: &'a [ApprovedBranch],
}

/// Told what the removal is waiting for, so the user can see it.
pub trait Notify {
    /// Another `wt remove` of this worktree holds its lock (R16); this run
    /// waits for it to finish or exit.
    fn removal_running(&self) {}

    /// Deletion waits on held or refilling entries (R22, R24); called when
    /// the wait starts and when what it reports changes.
    fn waiting(&self, _wait: &Wait) {}
}

/// [`Notify`] that shows nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Quiet;

impl Notify for Quiet {}

/// What the removal depends on outside the repository; tests inject each.
pub struct Env<'a> {
    /// Names the programs a deletion wait is waiting on.
    pub usage: &'a dyn UsageSource,
    pub mounts: &'a dyn MountInfo,
    /// Every wait: deletion's backoff and the lock wait.
    pub clock: &'a dyn Clock,
    pub faults: &'a dyn FsFaults,
    /// The user's Ctrl-C; polled at every entry and by every wait.
    pub cancelled: &'a dyn Fn() -> bool,
    /// Rechecks a conditionally approved branch on a later run.
    pub revalidate: &'a dyn Revalidate,
    pub notify: &'a dyn Notify,
}

/// How the CLI exits (N5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitClass {
    /// 0: the checkout is gone and every branch step is done.
    Success,
    /// 1: something needs the user (a permanent failure, a retained mount, a
    /// kept branch, an approval that could not be saved, Ctrl-C).
    Failure,
    /// 3: refused before anything changed, because what is there is not
    /// what was approved, or a record cannot be used.
    Refused,
    /// 4: refused before the question: the shell running `wt` stands in the
    /// checkout on Windows (R15).
    Blocked,
}

/// Why nothing was unlocked, moved, or deleted.
#[derive(Debug, thiserror::Error)]
pub enum Refusal {
    #[error("the main checkout is never removed")]
    MainCheckout,
    /// Windows R15: the shell's working directory is inside the checkout.
    #[error("the shell running wt is inside the worktree, which blocks deleting it")]
    ShellInside,
    /// The approval could not be saved, so nothing was deleted (R29).
    #[error(transparent)]
    NotSaved(StoreError),
    #[error(transparent)]
    Store(StoreError),
    /// The checkout cannot be bound to its own Git record, or changed since
    /// it was inspected.
    #[error(transparent)]
    Ineligible(MoveRefusal),
    /// The move failed with the checkout and its registration where they
    /// were. A lock the user approved removing stays removed.
    #[error(transparent)]
    Move(MoveError),
    #[error(transparent)]
    Git(WorktreeError),
    /// What the removal would act on is no longer what the question showed
    /// or the operation approved, or could not be identified before the
    /// question.
    #[error("{0}")]
    Replaced(String),
    #[error("the removal was cancelled before anything was deleted")]
    Cancelled,
}

impl Refusal {
    pub fn exit_class(&self) -> ExitClass {
        match self {
            Self::ShellInside => ExitClass::Blocked,
            Self::MainCheckout | Self::Ineligible(_) | Self::Replaced(_) => ExitClass::Refused,
            Self::Store(StoreError::Record { .. } | StoreError::Pending(_)) => ExitClass::Refused,
            Self::Move(error) if error.class() == MoveErrorClass::Refused => ExitClass::Refused,
            Self::NotSaved(_) | Self::Store(_) | Self::Move(_) | Self::Git(_) | Self::Cancelled => ExitClass::Failure,
        }
    }
}

/// What became of the checkout itself.
#[derive(Debug)]
pub enum CheckoutResult {
    /// Deleted, with its Git record.
    Removed,
    /// It was already gone (R17): only its Git record was removed.
    AlreadyRemoved,
    /// Everything else is deleted; these mounts and the directories above
    /// them are kept, with the registration, the record, and the branches,
    /// until they are unmounted (R7).
    RetainedMounts(Vec<Mount>),
    /// Everything else is deleted; these entries are kept until their cause
    /// is fixed (R29). Mounts kept as well are listed.
    Permanent { failures: Vec<Failure>, retained_mounts: Vec<Mount> },
    /// The user pressed Ctrl-C; `waiting_on` is what a wait was for.
    Cancelled { waiting_on: Vec<PathBuf> },
    /// The operation stopped where a step could not be completed; the
    /// sentence names the path and the cause.
    Stopped(String),
}

/// A removal that got past saving its approval.
#[derive(Debug)]
pub struct Finished {
    pub id: OperationId,
    /// The worktree's path, as the user knows it.
    pub path: PathBuf,
    pub checkout: CheckoutResult,
    /// Entries deleted by this run; `None` when not counted.
    pub entries: Option<u64>,
    /// Uncommitted files `git status` listed just before deletion that the
    /// question did not show (R14), checkout-relative.
    pub late_changes: Vec<PathBuf>,
    pub branches: Vec<BranchResult>,
    /// New content found at the original path after the move; kept.
    pub original_kept: Option<PathBuf>,
    /// Bookkeeping that failed without changing the outcome.
    pub warnings: Vec<String>,
}

impl Finished {
    fn new(id: OperationId, path: &Path) -> Self {
        Self {
            id,
            path: path.to_path_buf(),
            checkout: CheckoutResult::Removed,
            entries: None,
            late_changes: Vec::new(),
            branches: Vec::new(),
            original_kept: None,
            warnings: Vec::new(),
        }
    }

    /// Whether the record is kept for a later `wt remove` to finish.
    pub fn pending(&self) -> bool {
        !matches!(self.checkout, CheckoutResult::Removed | CheckoutResult::AlreadyRemoved)
    }

    pub fn exit_class(&self) -> ExitClass {
        let branches_done = self.branches.iter().all(|branch| branch.outcome.is_done());
        if !self.pending() && branches_done { ExitClass::Success } else { ExitClass::Failure }
    }
}

/// How one run ended.
#[derive(Debug)]
pub enum Removal {
    Refused(Box<Refusal>),
    Finished(Box<Finished>),
}

impl Removal {
    pub fn exit_class(&self) -> ExitClass {
        match self {
            Self::Refused(refusal) => refusal.exit_class(),
            Self::Finished(finished) => finished.exit_class(),
        }
    }
}

/// Removes `request.entry` after the user confirmed the question (see the
/// module docs).
pub fn remove(request: &Request<'_>, env: &Env<'_>) -> Removal {
    match run_with(request, env, &mut Seams::none()) {
        Ok(removal) => removal,
        Err(Crashed) => unreachable!("only a test hook stops the lifecycle"),
    }
}

/// A point the lifecycle passes, for crash-injection tests; in flowchart
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Boundary {
    /// The approval is durable; nothing has changed.
    Begun,
    Unlocked,
    Moved,
    MarkedInPlace,
    /// Another `delete::BATCH_ENTRIES` entries are gone (the total so far).
    /// Passed, but cannot crash in process.
    DeletionBatch(u64),
    ContentRemoved,
    Unregistered,
    /// Branch step `i` is recorded as finished.
    BranchAction(usize),
    BranchesDone,
}

/// A test hook stopped the lifecycle at a [`Boundary`] as if the process had
/// died there: nothing more is written.
#[derive(Debug)]
pub(crate) struct Crashed;

/// Returns `true` to crash at the boundary.
pub(crate) type Hook<'h> = dyn FnMut(Boundary) -> bool + 'h;

/// The crate-internal seams of one run.
pub(crate) struct Seams<'s> {
    pub hook: Box<Hook<'s>>,
    /// Faults for the move's rename and association steps.
    pub moves: Faults,
}

impl Seams<'_> {
    pub(crate) fn none() -> Self {
        Self { hook: Box::new(|_| false), moves: Faults::default() }
    }

    fn at(&mut self, boundary: Boundary) -> Result<(), Crashed> {
        if (self.hook)(boundary) { Err(Crashed) } else { Ok(()) }
    }
}

pub(crate) fn run_with(request: &Request<'_>, env: &Env<'_>, seams: &mut Seams<'_>) -> Result<Removal, Crashed> {
    let refuse = |refusal: Refusal| Ok(Removal::Refused(Box::new(refusal)));
    if request.entry.is_main {
        return refuse(Refusal::MainCheckout);
    }
    if request.facts.blocker.is_some() {
        return refuse(Refusal::ShellInside);
    }
    match approve(request, env) {
        Approval::Saved(saved) => {
            let (pending, plan) = *saved;
            seams.at(Boundary::Begun)?;
            let finished = Finished::new(pending.id().clone(), pending.record().original.as_path());
            let operation = Operation { base: request.base, env, pending, finished, fresh: true };
            let shown = late::shown(request.facts.inventory.as_ref());
            operation.run(Progress::Approved, plan, shown.as_deref(), seams)
        }
        // An earlier run of this worktree's removal exited (R16, R19):
        // finish that one, with its own approval. Its question is not this
        // run's, so no late change is named.
        Approval::Earlier(other) => resume::resume_with(request.base, &other, env, seams, None, None),
        Approval::Refused(refusal) => refuse(refusal),
    }
}

/// What an approval binds, checked against the question's identities.
struct Bound {
    original: PathBuf,
    deletion_path: PathBuf,
    deletion_mode: DeletionMode,
    target: ApprovedTarget,
    /// The move to make; `None` deletes in place.
    plan: Option<MovePlan>,
}

/// Binds what `request` approved to what is there now, before anything is
/// saved or changed. Every object is compared with the identity the
/// question's facts read: one that changed since, or that the facts could
/// not identify, refuses.
fn bind(request: &Request<'_>, env: &Env<'_>, id: &OperationId) -> Result<(Bound, RepositoryBinding), Refusal> {
    let Request { base, entry, facts, .. } = *request;
    let path = &entry.path;
    let changed = |what: &Path| Refusal::Replaced(format!("{} changed after the question, so nothing was removed", what.display()));
    let unidentified = || Refusal::Replaced(format!("{} could not be identified before the question, so nothing was removed", path.display()));
    let Some(gathered) = &facts.identities.repository else {
        return Err(Refusal::Replaced(format!(
            "Git's record of {} could not be identified before the question, so nothing was removed",
            path.display()
        )));
    };
    let is = |at: &Path, identity: FileIdentity| rename::identity(at).is_ok_and(|found| found == identity);
    let in_place = |target: ApprovedTarget, deletion_path: PathBuf| Bound {
        original: path.clone(),
        deletion_path,
        deletion_mode: DeletionMode::InPlace,
        target,
        plan: None,
    };
    let (bound, admin) = match &facts.target {
        Target::Checkout { .. } => {
            let identity = facts.identities.entry.ok_or_else(unidentified)?;
            if !is(path, identity) {
                return Err(changed(path));
            }
            let (bound, admin) = bind_checkout(request, id, env)?;
            if bound.target != (ApprovedTarget::Checkout { identity }) {
                return Err(changed(path));
            }
            (bound, admin)
        }
        Target::Link { own: true, resolved: LinkTarget::Resolved(real), .. } => {
            let (Some(link), Some(checkout)) = (facts.identities.entry, facts.identities.resolved) else { return Err(unidentified()) };
            if !is(path, link) {
                return Err(changed(path));
            }
            if !is(real, checkout) {
                return Err(changed(real));
            }
            (in_place(ApprovedTarget::OwnLink { link, checkout }, real.clone()), gathered.admin_dir.as_path().to_path_buf())
        }
        Target::Link { .. } | Target::NotADirectory | Target::Uninspectable => {
            let identity = facts.identities.entry.ok_or_else(unidentified)?;
            if !is(path, identity) {
                return Err(changed(path));
            }
            (in_place(ApprovedTarget::Entry { identity }, path.clone()), gathered.admin_dir.as_path().to_path_buf())
        }
        Target::Missing | Target::Vanished => {
            if !absent(path) {
                return Err(Refusal::Replaced(format!("{} appeared after the question, so nothing was removed", path.display())));
            }
            (in_place(ApprovedTarget::Absent, path.clone()), gathered.admin_dir.as_path().to_path_buf())
        }
    };
    let changed_record = || changed(gathered.admin_dir.as_path());
    let current = facts::repository_binding(base, &AdminEntry { dir: admin }).map_err(|_| changed_record())?;
    let same = current.common_dir_identity == gathered.common_dir_identity
        && current.admin_dir_identity == gathered.admin_dir_identity
        && current.admin_name() == gathered.admin_name();
    if !same {
        return Err(changed_record());
    }
    Ok((bound, current))
}

/// Binds a checkout directory: planned for the move, or bound in place
/// where it cannot be moved. Returns the admin directory it is bound to.
fn bind_checkout(request: &Request<'_>, id: &OperationId, env: &Env<'_>) -> Result<(Bound, PathBuf), Refusal> {
    let Request { base, entry, facts, .. } = *request;
    let (identity, admin, plan) = match plan_move(base, entry, id) {
        Ok(plan) => (plan.identity, plan.admin.dir.clone(), Some(plan)),
        // R10, R13: the question said Git cannot read the checkout, so a
        // `.git` the move cannot follow is no reason to stop; the checkout
        // is bound by its Git record's back-reference and deleted in place.
        Err(refusal) if in_place_for_refusal(&refusal).is_some() || unreadable_by_git(facts, &refusal) => {
            let (identity, admin) = bind_in_place(base, entry)?;
            (identity, admin, None)
        }
        Err(refusal) => return Err(Refusal::Ineligible(refusal)),
    };
    let original = entry.path.clone();
    let moved = plan.as_ref().filter(|_| nested_mounts(env.mounts, &original).is_empty());
    let (deletion_path, deletion_mode) = match moved {
        Some(plan) => (plan.quarantine.clone(), DeletionMode::Moved),
        None => (original.clone(), DeletionMode::InPlace),
    };
    Ok((Bound { original, deletion_path, deletion_mode, target: ApprovedTarget::Checkout { identity }, plan }, admin))
}

/// Binds the checkout at `entry.path` to its own Git record without a move
/// plan, for a checkout that will be deleted in place: its identity and its
/// admin directory.
fn bind_in_place(base: &Path, entry: &WorktreeEntry) -> Result<(FileIdentity, PathBuf), Refusal> {
    let original = &entry.path;
    match availability::inspect(original) {
        Ok(EntryKind::Directory) => {}
        Ok(_) => return Err(Refusal::Ineligible(MoveRefusal::NotARealDirectory(original.clone()))),
        Err(error) => return Err(Refusal::Ineligible(MoveRefusal::Uninspectable(format!("{}: {error}", original.display())))),
    }
    let identity = rename::identity(original).map_err(|error| Refusal::Git(WorktreeError::Io(error)))?;
    let common_dir = common_git_dir(base).map_err(Refusal::Git)?;
    let admin = admin_entry_in(&common_dir, original).map_err(|error| Refusal::Ineligible(MoveRefusal::Association(error)))?;
    Ok((identity, admin.dir))
}

/// Saves the approval of `request` as a pending operation and returns its
/// id, changing nothing else: the shell wrapper's first run (R18) prints the
/// id in its handoff, and the second run finishes it with
/// [`finish_handoff`]. An operation an earlier run of this worktree left is
/// returned instead, to be finished the same way.
///
/// ## Errors
///
/// The [`Refusal`] that kept the approval from being saved; a target that
/// is not a checkout directory is [`Refusal::Ineligible`], since it has
/// nothing for a shell to stand in.
pub fn save_approval(request: &Request<'_>, env: &Env<'_>) -> Result<OperationId, Refusal> {
    if request.entry.is_main {
        return Err(Refusal::MainCheckout);
    }
    if request.facts.blocker.is_some() {
        return Err(Refusal::ShellInside);
    }
    if !matches!(request.facts.target, Target::Checkout { .. }) {
        return Err(Refusal::Ineligible(MoveRefusal::Unsupported("only a checkout directory is handed off".into())));
    }
    match approve(request, env) {
        Approval::Saved(saved) => Ok(saved.0.id().clone()),
        Approval::Earlier(other) => Ok(other),
        Approval::Refused(refusal) => Err(refusal),
    }
}

/// How saving an approval ended.
enum Approval {
    Saved(Box<(PendingRemoval, Option<MovePlan>)>),
    /// Another operation of this worktree is stored.
    Earlier(OperationId),
    Refused(Refusal),
}

fn approve(request: &Request<'_>, env: &Env<'_>) -> Approval {
    let refuse = Approval::Refused;
    let Request { entry, requested_name, facts, branches, .. } = *request;
    let id = match OperationId::generate() {
        Ok(id) => id,
        Err(error) => return refuse(Refusal::Git(error)),
    };
    let (bound, repository) = match bind(request, env, &id) {
        Ok(bound) => bound,
        Err(refusal) => return refuse(refusal),
    };
    let store = match RecoveryStore::open(repository.common_dir.as_path()) {
        Ok(store) => store,
        Err(error) => return refuse(Refusal::NotSaved(error)),
    };
    let lock = match take_lock(&store, &repository, env) {
        Ok(lock) => lock,
        Err(error) => return refuse(error),
    };
    let Bound { original, deletion_path, deletion_mode, target, plan } = bound;
    let record = CompactRecord {
        record_version: COMPACT_RECORD_VERSION,
        operation_id: id.clone(),
        created_at_ms: now_ms(),
        requested_name: requested_name.to_string(),
        aliases: aliases(entry, &original, requested_name),
        original: original.as_path().into(),
        deletion_path: deletion_path.as_path().into(),
        deletion_mode,
        target,
        repository,
        unlock_needed: facts.warnings.iter().any(|warning| matches!(warning, Warning::Locked { .. })),
        branch_steps: branches
            .iter()
            .map(|branch| BranchStep { action: branch.action.clone(), approval: branch.approval, state: StepState::Pending })
            .collect(),
        progress: Progress::Approved,
    };
    match store.approve(lock, record) {
        Ok(pending) => Approval::Saved(Box::new((pending, plan))),
        Err(StoreError::Pending(other)) => Approval::Earlier(other),
        Err(error @ (StoreError::NotSaved(_) | StoreError::Io(_) | StoreError::Storage { .. })) => refuse(Refusal::NotSaved(error)),
        Err(error) => refuse(Refusal::Store(error)),
    }
}

/// Whether `refusal` is about the checkout's `.git` link, which the question
/// already warned Git cannot read.
fn unreadable_by_git(facts: &Facts, refusal: &MoveRefusal) -> bool {
    let warned = facts.warnings.iter().any(|warning| matches!(warning, Warning::GitUnreadable { .. } | Warning::Unchecked { .. }));
    warned && matches!(refusal, MoveRefusal::Unsupported(_) | MoveRefusal::Uninspectable(_) | MoveRefusal::Association(_))
}

/// Takes the worktree's removal lock, telling [`Notify::removal_running`]
/// and waiting while another removal holds it.
fn take_lock(store: &RecoveryStore, repository: &RepositoryBinding, env: &Env<'_>) -> Result<OperationLock, Refusal> {
    match store.lock(repository) {
        Ok(lock) => Ok(lock),
        Err(StoreError::Lock(LockError::Busy)) => {
            env.notify.removal_running();
            store.lock_waiting(repository, env.clock, env.cancelled).map_err(|error| match error {
                StoreError::Cancelled => Refusal::Cancelled,
                error => Refusal::Store(error),
            })
        }
        Err(error) => Err(Refusal::Store(error)),
    }
}

/// The other names that find this operation: the branch and the directory.
fn aliases(entry: &WorktreeEntry, original: &Path, requested_name: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let directory = original.file_name().map(|name| name.to_string_lossy().into_owned());
    for alias in entry.branch.iter().cloned().chain(directory) {
        if alias != requested_name && !alias.is_empty() && !aliases.contains(&alias) {
            aliases.push(alias);
        }
    }
    aliases
}

/// The mounts strictly inside the checkout at `spelled`, re-spelled under
/// it, for the choice between moving and deleting in place. Native tables
/// spell real paths, so the checkout is resolved the same way to find them.
/// An unreadable table is no mounts here: deletion still checks each
/// directory before entering it, and the question warned.
fn nested_mounts(mounts: &dyn MountInfo, spelled: &Path) -> Vec<Mount> {
    let Ok(table) = mounts.mounts() else { return Vec::new() };
    let real = biscuit_file::canonicalize_simplified(spelled).unwrap_or_else(|_| spelled.to_path_buf());
    let mut nested: Vec<Mount> = table
        .iter()
        .filter_map(|mount| {
            let rest = mount.mount_point.strip_prefix(&real).ok().or_else(|| mount.mount_point.strip_prefix(spelled).ok())?;
            (!rest.as_os_str().is_empty()).then(|| Mount { mount_point: spelled.join(rest), ..mount.clone() })
        })
        .collect();
    nested.sort_by_key(|mount| mount.mount_point.components().count());
    nested.dedup();
    nested
}

/// One saved operation being carried forward, from its approval or a later
/// run's resumption to its finish.
pub(crate) struct Operation<'a> {
    pub base: &'a Path,
    pub env: &'a Env<'a>,
    pub pending: PendingRemoval,
    pub finished: Finished,
    /// The approval was given in this run, so a conditional branch step was
    /// just assessed and is not rechecked.
    pub fresh: bool,
}

impl Operation<'_> {
    fn record(&self) -> &CompactRecord {
        self.pending.record()
    }

    /// Runs every step after `from`, which a resumed operation reads from
    /// the paths and Git rather than from the record's marker. An approved
    /// unlock the marker does not show as done is made in any case (it is a
    /// no-op once the lock is gone). Only a checkout directory is placed.
    /// `plan` is the move to make; `shown` is the dirty paths the question
    /// listed ([`late::shown`]), `None` when no question's list reached
    /// this run.
    pub(crate) fn run(
        mut self,
        from: Progress,
        plan: Option<MovePlan>,
        shown: Option<&[PathBuf]>,
        seams: &mut Seams<'_>,
    ) -> Result<Removal, Crashed> {
        let target = self.record().target;
        if target == ApprovedTarget::Absent {
            self.finished.checkout = CheckoutResult::AlreadyRemoved;
        }
        if self.record().progress < Progress::Unlocked {
            if let Err(stopped) = self.unlock() {
                return Ok(self.stop(stopped));
            }
            seams.at(Boundary::Unlocked)?;
        }
        if matches!(target, ApprovedTarget::Checkout { .. }) && from < Progress::Placed {
            match self.place(plan, seams) {
                Ok(boundary) => seams.at(boundary)?,
                Err(Ended::Refused(refusal)) => return Ok(self.close(refusal)),
                Err(Ended::Stopped(stopped)) => return Ok(self.stop(stopped)),
            }
        }
        let root = self.record().deletion_path.as_path().to_path_buf();
        if from < Progress::ContentRemoved && target.checkout_identity().is_some() {
            self.finished.late_changes = late::unseen(self.base, &root, shown);
        }
        if from < Progress::ContentRemoved {
            let deleted = match target {
                ApprovedTarget::Checkout { .. } => self.delete(&root, seams),
                // The real checkout first, so the link is the last thing
                // that finds the operation.
                ApprovedTarget::OwnLink { link, .. } => match if absent(&root) { Deleted::Done } else { self.delete(&root, seams) } {
                    Deleted::Done => self.remove_bound(link),
                    other => other,
                },
                ApprovedTarget::Entry { identity } => self.remove_bound(identity),
                ApprovedTarget::Absent => Deleted::Done,
            };
            match deleted {
                Deleted::Done => {
                    if let Err(error) = self.pending.advance(Progress::ContentRemoved) {
                        return Ok(self.stop(not_recorded(error)));
                    }
                }
                Deleted::Kept(result) => {
                    self.finished.checkout = result;
                    return Ok(Removal::Finished(Box::new(self.finished)));
                }
                Deleted::Replaced(refusal) => return Ok(self.close(refusal)),
            }
            seams.at(Boundary::ContentRemoved)?;
        }
        if from < Progress::Unregistered {
            if let Err(stopped) = self.unregister() {
                return Ok(self.stop(stopped));
            }
            seams.at(Boundary::Unregistered)?;
        }
        self.branch_steps(seams)
    }

    fn unlock(&mut self) -> Result<(), String> {
        let original = self.record().original.as_path().to_path_buf();
        let admin = self.record().repository.admin_dir.as_path().to_path_buf();
        if self.record().unlock_needed && availability::inspect(&admin.join("locked")).is_ok() {
            unlock(self.base, &original, &admin)?;
        }
        self.pending.advance(Progress::Unlocked).map_err(not_recorded)
    }

    /// Moves the checkout aside, or marks it for deletion in place.
    fn place(&mut self, plan: Option<MovePlan>, seams: &mut Seams<'_>) -> Result<Boundary, Ended> {
        let original = self.record().original.as_path().to_path_buf();
        let moved = match (self.record().deletion_mode, plan) {
            (DeletionMode::InPlace, _) | (DeletionMode::Moved, None) => false,
            (DeletionMode::Moved, Some(plan)) => {
                let mounts = nested_mounts(self.env.mounts, &original);
                match place_with(self.base, &plan, &mounts, &seams.moves) {
                    Ok(Placement::Moved) => true,
                    Ok(Placement::InPlace(_)) => false,
                    Err(error) if error.nothing_changed() => return Err(Ended::Refused(Refusal::Move(error))),
                    Err(error) => return Err(Ended::Stopped(format!("{} could not be moved aside: {error}", original.display()))),
                }
            }
        };
        if !moved {
            self.pending.mark_in_place().map_err(|error| Ended::Stopped(not_recorded(error)))?;
        }
        self.pending.advance(Progress::Placed).map_err(|error| Ended::Stopped(not_recorded(error)))?;
        Ok(if moved { Boundary::Moved } else { Boundary::MarkedInPlace })
    }

    /// Deletes the placed checkout, or an own link's real checkout.
    fn delete(&mut self, root: &Path, seams: &mut Seams<'_>) -> Deleted {
        let identity = self.record().target.checkout_identity().expect("only a checkout directory is deleted");
        let holders = UsageHolders(self.env.usage);
        let env = delete::Env { clock: self.env.clock, faults: self.env.faults, mounts: self.env.mounts, holders: &holders, cancelled: self.env.cancelled };
        let mut progress = Reported { notify: self.env.notify, hook: &mut *seams.hook, deleted: None };
        let deletion = delete::delete_tree(delete::Target { root, identity: &identity }, &env, &mut progress);
        self.finished.entries = progress.deleted;
        match deletion {
            Deletion::Done => Deleted::Done,
            Deletion::RetainedMounts(mounts) => Deleted::Kept(CheckoutResult::RetainedMounts(mounts)),
            Deletion::Permanent { failures, retained_mounts } => Deleted::Kept(CheckoutResult::Permanent { failures, retained_mounts }),
            Deletion::Cancelled { waiting_on } => Deleted::Kept(CheckoutResult::Cancelled { waiting_on }),
            Deletion::RootReplaced => Deleted::Replaced(Refusal::Replaced(format!(
                "{} is no longer the worktree that was approved for removal, so it was not deleted",
                root.display()
            ))),
        }
    }

    /// Removes the entry at the original path as itself while it is still
    /// the approved `identity`; one that changed since is kept.
    fn remove_bound(&self, identity: FileIdentity) -> Deleted {
        let original = self.record().original.as_path();
        let failed = |cause: io::Error| {
            Deleted::Kept(CheckoutResult::Permanent { failures: vec![Failure { path: original.to_path_buf(), cause }], retained_mounts: Vec::new() })
        };
        match rename::identity(original) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Deleted::Done,
            Err(error) => failed(error),
            Ok(found) if found != identity => Deleted::Replaced(Refusal::Replaced(format!(
                "{} changed after the question, so it was kept",
                original.display()
            ))),
            Ok(_) => match remove_entry(original) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => failed(error),
                _ => Deleted::Done,
            },
        }
    }

    /// Removes Git's registration of the deleted checkout, then its copy
    /// record. Git's record is gone already after a crash just past this.
    fn unregister(&mut self) -> Result<(), String> {
        let record = self.record();
        let original = record.original.as_path().to_path_buf();
        let registered = match record.deletion_mode {
            DeletionMode::Moved => record.deletion_path.as_path().to_path_buf(),
            DeletionMode::InPlace => original.clone(),
        };
        let admin = record.repository.admin_dir.as_path().to_path_buf();
        let admin_identity = record.repository.admin_dir_identity;
        if record.deletion_mode == DeletionMode::Moved {
            match rename::identity(&original) {
                Ok(_) => self.finished.original_kept = Some(original.clone()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => self.finished.original_kept = Some(original.clone()),
            }
        }
        if !absent(&admin) {
            if rename::identity(&admin).ok() != Some(admin_identity) {
                return Err(format!("Git's record {} is no longer this worktree's, so it was kept", admin.display()));
            }
            remove_registration(self.base, &registered, &admin)?;
        }
        if let Some(warning) = crate::copy_record::delete_for(self.base, &original) {
            self.finished.warnings.push(format!("could not delete the copy record: {warning}"));
        }
        self.pending.advance(Progress::Unregistered).map_err(not_recorded)
    }

    fn branch_steps(mut self, seams: &mut Seams<'_>) -> Result<Removal, Crashed> {
        let steps = self.record().branch_steps.clone();
        let remote_deleted = steps.iter().any(|step| matches!(step.action, BranchAction::Remote { .. }));
        for (index, step) in steps.iter().enumerate() {
            if step.state != StepState::Pending {
                self.finished.branches.push(BranchResult::recorded(step));
                continue;
            }
            let outcome = branches::run(self.base, step, self.env.revalidate, self.fresh, remote_deleted);
            let state = if outcome.is_done() { StepState::Done } else { StepState::Kept };
            self.finished.branches.push(BranchResult { target: branches::target(&step.action), outcome });
            if let Err(error) = self.pending.finish_step(index, state) {
                self.finished.warnings.push(not_recorded(error));
                return Ok(Removal::Finished(Box::new(self.finished)));
            }
            seams.at(Boundary::BranchAction(index))?;
        }
        seams.at(Boundary::BranchesDone)?;
        let Self { pending, mut finished, .. } = self;
        if let Err(error) = pending.finish() {
            finished.warnings.push(format!("the finished removal's record could not be deleted: {error}"));
        }
        Ok(Removal::Finished(Box::new(finished)))
    }

    /// Ends the run with the record kept for the next `wt remove`.
    fn stop(mut self, sentence: String) -> Removal {
        self.finished.checkout = CheckoutResult::Stopped(sentence);
        Removal::Finished(Box::new(self.finished))
    }

    /// Ends an operation whose checkout is not to be deleted any more: the
    /// record goes, so a later run starts from a fresh question.
    fn close(self, refusal: Refusal) -> Removal {
        let _ = self.pending.finish();
        Removal::Refused(Box::new(refusal))
    }
}

/// How deletion of a placed checkout ended.
enum Deleted {
    Done,
    /// Something is left; the record is kept for a later run.
    Kept(CheckoutResult),
    /// The root is not the approved checkout; the operation is closed.
    Replaced(Refusal),
}

/// How placement ended a run early.
enum Ended {
    Refused(Refusal),
    Stopped(String),
}

fn not_recorded(error: StoreError) -> String {
    format!("the removal's progress could not be recorded: {error}")
}

/// Passes deletion's progress to the hook and the user.
struct Reported<'r, 'h> {
    notify: &'r dyn Notify,
    hook: &'r mut Hook<'h>,
    deleted: Option<u64>,
}

impl delete::Progress for Reported<'_, '_> {
    fn batch(&mut self, deleted: u64) {
        (self.hook)(Boundary::DeletionBatch(deleted));
    }

    fn waiting(&mut self, wait: &Wait) {
        self.notify.waiting(wait);
    }

    fn finished(&mut self, deleted: u64) {
        self.deleted = Some(deleted);
    }
}

/// `git worktree unlock` of the worktree registered at `registered`. Never
/// by hand (N7).
fn unlock(base: &Path, registered: &Path, admin: &Path) -> Result<(), String> {
    let output = crate::git::git_from_output(base, base, &[OsStr::new("worktree"), OsStr::new("unlock"), registered.as_os_str()])
        .map_err(|error| error.to_string())?;
    if !output.status.success() && availability::inspect(&admin.join("locked")).is_ok() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("{} could not be unlocked: {stderr}", registered.display()));
    }
    Ok(())
}

/// Plain `git worktree remove` of the worktree registered at `registered`,
/// whose checkout is gone, after confirming `admin` is still its record.
/// Git deletes the record, its `modules/` included.
fn remove_registration(base: &Path, registered: &Path, admin: &Path) -> Result<(), String> {
    match read_back_reference(admin) {
        Ok(named) if same_location(&named, &registered.join(".git")) => {}
        Ok(named) => return Err(format!("Git's record of {} now names {}, so it was kept", registered.display(), named.display())),
        Err(BackReferenceError::Missing) if absent(admin) => return Ok(()),
        Err(error) => return Err(format!("Git's record of {} could not be read: {error}", registered.display())),
    }
    if !absent(registered) {
        return Err(format!("{} still exists, so Git's record of it was kept", registered.display()));
    }
    let output = crate::git::git_from_output(base, base, &[OsStr::new("worktree"), OsStr::new("remove"), registered.as_os_str()])
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Git's record of {} could not be removed: {stderr}", registered.display()));
    }
    Ok(())
}

/// Removes the entry at `path` as itself: a link (a junction or other
/// reparse point on Windows) is never followed, and a file is unlinked.
fn remove_entry(path: &Path) -> io::Result<()> {
    match availability::inspect(path)? {
        // Never deleted as an entry; Git's record of it is then kept.
        EntryKind::Directory => Ok(()),
        EntryKind::Link => remove_link(path),
        EntryKind::File => std::fs::remove_file(path),
    }
}

fn remove_link(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
        // A junction or directory symlink reports no `is_dir`, and deleting
        // it as a file is denied; it is removed as a directory entry, which
        // never touches its target.
        let directory = std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0);
        if directory {
            return std::fs::remove_dir(path);
        }
    }
    std::fs::remove_file(path)
}

fn absent(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

impl fmt::Display for CheckoutResult {
    /// One plain sentence about what is left, for a run that did not finish.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Removed => f.write_str("the worktree was removed"),
            Self::AlreadyRemoved => f.write_str("the worktree was already gone; Git's record of it was removed"),
            Self::RetainedMounts(mounts) => {
                let points: Vec<String> = mounts.iter().map(|mount| mount.mount_point.display().to_string()).collect();
                write!(f, "{} is still mounted inside the worktree; unmount it, then run wt remove again", points.join(", "))
            }
            Self::Permanent { failures, .. } => {
                let sentences: Vec<String> = failures.iter().map(ToString::to_string).collect();
                f.write_str(&sentences.join("; "))
            }
            Self::Cancelled { .. } => f.write_str("the removal was cancelled; run wt remove again to finish it"),
            Self::Stopped(sentence) => f.write_str(sentence),
        }
    }
}
