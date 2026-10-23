//! One contention-aware removal of a linked checkout, from the decisive usage
//! query to the closed recovery record.
//!
//! [`remove_checkout`] runs the order `2026-10-05-graceful-file-contention`
//! decides (its decision 8), each step through the module that owns it:
//!
//! 1. [`plan_move`] fixes the quarantine and refuses ineligible topologies
//!    (locked, submodules, links) before anything else;
//! 2. activity [`observation`](super::observation) starts and the
//!    [`DeletionInventory`] is captured, so no window exists in which a new
//!    writer is seen by neither;
//! 3. the decisive [`check_usage`] runs, excluding only the observer's own
//!    registered handles (and rechecking once when only Windows Search
//!    held files);
//! 4. the approval's bindings are read again (the branch tip and the consent
//!    fingerprint), the inventory is confirmed against everything observed
//!    so far, and the index is bound ([`bind_index`]);
//! 5. [`RecoveryStore::begin`] makes the record and the inventory durable,
//!    and one last drain refuses anything that changed since;
//! 6. [`move_to_quarantine`], then the move's own events are explained
//!    (the original moved away, the quarantine moved in, the rewritten admin
//!    `gitdir`) before [`delete_content`](super::observation::deletion::delete_content)
//!    and [`unregister_quarantine`];
//! 7. only then the original's copy record and the branch actions, and
//!    [`Operation::complete`] persists completion before the journal goes.
//!
//! Nothing is moved before step 6, so every failure up to there is a
//! [`Refusal`] that changed nothing (exit 3 or 4, or 1 for an operational
//! error or an interrupt seen just before the move). From the move on, the operation always ends as [`Finished`]: a
//! saved report, and a pending record whenever the checkout or a branch
//! action is unfinished. [`resume`] picks such a record up again, and
//! [`resolve_target`] finds it by the name the user typed.

mod resolve;
mod resume;
#[cfg(test)]
mod tests;

pub use resolve::{Candidate, PendingRemovals, PendingTarget, ResolveError, Target, resolve_target, resolve_target_in};
pub use resume::{Inspection, RecordApproval, Resolution, ResumeRequest, inspect, release_unstarted, resume, resume_consent};

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use sniff::filesystem::query::{FileIdentity, NativeString};

use crate::availability::{self, EntryKind};
use crate::error::WorktreeError;
use crate::worktree::WorktreeEntry;

use super::admin_entry::{BackReferenceError, read_back_reference, same_location};
use super::included::classify_included;
use super::inventory::{Inventory, collect_inventory};
use super::missing::{IndexBinding, MissingRefusal, bind_index, recorded_head};
use super::observation::deletion::{DeletionOptions, Stop, delete_content_with, own_move};
use super::observation::inventory::{DeletionInventory, InventoryRefusal};
use super::observation::{DiagnosticsHandle, Drain, EventKind, Observation, ObservationError, ObserverOptions, Scope, Targets};
use super::outcome::{
    ActivityKind, ActivityLocation, BranchOutcome, BranchState, BranchTarget, DiscoveryPurpose, DiscoveryResult,
    DiscoveryRun, ObservedActivity, OperationReport, OriginalState, QuarantineReport, QuarantineState, RegistrationState,
    RemovalOutcome, ReportNote, Step, StepResult, TimelineEvent,
};
use super::quarantine::association::{MoveError, MoveErrorClass, MovePlan, MoveRefusal, move_to_quarantine, plan_move};
use super::quarantine::association::CheckoutLocation;
use super::quarantine::rename;
use super::quarantine::unregister::{Unregistered, unregister_quarantine};
use super::recovery::OperationId;
use super::recovery::journal::{EntryState, summarize};
use super::recovery::lock::LockError;
use super::recovery::record::{BranchBinding, IntendedAction, Phase, RECORD_VERSION, RecoveryRecord, RepositoryBinding};
use super::recovery::store::{BookkeepingWarning, Operation, RecoveryStore, StoreError};
use super::remote::delete_remote_branch;
use super::remove_local_branch;
use super::usage::{OwnedHandles, UsageClearance, UsageRefusal, UsageSource, check_usage};

/// What the user approved, as the report showed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    /// The checkout's files, with its include assessment; on resume,
    /// [`ResumeConsent::inventory`].
    pub inventory: Inventory,
    /// [`Inventory::fingerprint`] of `inventory` (see [`consent_state`]); on
    /// resume, [`ResumeConsent::fingerprint`].
    pub fingerprint: String,
    /// The commit the checkout's branch (or detached HEAD) was approved at.
    pub tip: String,
    /// Consent to discard what `inventory` says needs it.
    pub discard_files: bool,
}

/// The consent-relevant state of the checkout at `checkout` now: its files
/// and include assessment, and their fingerprint. Runs `git status`, which
/// may rewrite the worktree's index.
///
/// ## Errors
///
/// Git's failure, or [`WorktreeError::Io`] when a file consent would cover
/// cannot be read.
pub fn consent_state(base: &Path, checkout: &Path, branch: Option<&str>) -> Result<(Inventory, String), WorktreeError> {
    let mut inventory = collect_inventory(base, checkout)?;
    inventory.included = classify_included(base, checkout, branch)?;
    let fingerprint = inventory.fingerprint(base, checkout)?;
    Ok((inventory, fingerprint))
}

/// The consent state of a pending removal's moved checkout: what
/// [`consent_state`] collects there, less the dirty entries the operation's
/// own confirmed deletions explain. Built by [`resume_consent`]; a retry's
/// [`Approval`] carries `inventory` and `fingerprint`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeConsent {
    /// Everything `git status` lists in the moved checkout.
    pub raw: Inventory,
    /// `raw` without `excluded`: what the report lists and consent covers.
    pub inventory: Inventory,
    /// Each an unstaged deletion (porcelain ` D`) of a path that some
    /// attempt's journal records as `Deleted`, inspected absent now.
    pub excluded: Vec<PathBuf>,
    /// [`Inventory::fingerprint`] of `raw`, bound to `excluded`, so a retry's
    /// approval and the recheck under the operation lock compare both views.
    pub fingerprint: String,
}

/// `raw` split into what consent covers and what this operation's confirmed
/// deletions explain. `deleted` holds every journal path with a durable
/// `Deleted` result; `absent` inspects a quarantine-relative path without
/// following links, and an inspection error is not absence.
///
/// Paths are compared by component, which is exact and case-sensitive but
/// lets Git's `/` match a Windows journal's `\`; a deleted directory
/// explains no path beneath it.
fn explain_deletions(raw: &Inventory, deleted: &HashSet<&Path>, absent: impl Fn(&Path) -> bool) -> (Inventory, Vec<PathBuf>) {
    let mut inventory = raw.clone();
    let mut excluded = Vec::new();
    inventory.dirty.retain(|entry| {
        let explained = entry.status == " D" && deleted.get(entry.path.as_path()).is_some_and(|journaled| absent(journaled));
        if explained {
            excluded.push(entry.path.clone());
        }
        !explained
    });
    (inventory, excluded)
}

/// Deletes the local branch `name` while its tip is still `expected_tip`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBranchDeletion {
    pub name: String,
    pub expected_tip: String,
}

/// Deletes branch `destination` on `remote` through `endpoint`, leased to
/// `expected_sha`; see [`delete_remote_branch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBranchDeletion {
    pub remote: String,
    pub destination: String,
    pub endpoint: String,
    pub expected_sha: String,
}

/// The approved branch actions, run only after the checkout's removal is
/// verified, local before remote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchActions {
    pub local: Option<LocalBranchDeletion>,
    pub remote: Option<RemoteBranchDeletion>,
}

/// One removal of the live, linked worktree `entry`.
#[derive(Debug, Clone, Copy)]
pub struct RemovalRequest<'a> {
    pub base: &'a Path,
    pub entry: &'a WorktreeEntry,
    /// The name the user typed; saved so a retry finds the operation by it.
    pub requested_name: &'a str,
    pub approval: &'a Approval,
    pub branches: &'a BranchActions,
}

/// How an observer is started: `moved` says the tree is already at the
/// quarantine; the predicate is [`Environment::interrupted`].
pub type StartObservation<'a> = dyn Fn(Targets, bool, &dyn Fn() -> bool) -> Result<Observation, ObservationError> + 'a;

/// What the lifecycle asks of its surroundings. Tests inject each.
pub struct Environment<'a> {
    pub usage: &'a dyn UsageSource,
    /// Polled once before the move, where it refuses with nothing changed,
    /// at every deletion entry boundary, where it stops the removal, and by
    /// every wait for change delivery, which it ends at once: before the
    /// move that refuses, after it the removal stops.
    pub interrupted: &'a dyn Fn() -> bool,
    pub observe: &'a StartObservation<'a>,
    /// Called at most once per attempt, after observation starts and before
    /// the move (on a resumed attempt, before its further deletion), when
    /// the startup ([`ObservationDiagnostics::startup_ms`]) took longer than
    /// [`SLOW_STARTUP`]. The removal continues.
    ///
    /// [`ObservationDiagnostics::startup_ms`]: super::observation::ObservationDiagnostics::startup_ms
    pub slow_startup: &'a dyn Fn(Duration),
}

/// The observation startup above which [`Environment::slow_startup`] is told.
pub const SLOW_STARTUP: Duration = Duration::from_secs(1);

/// Tells [`Environment::slow_startup`] when `observation` took longer than
/// [`SLOW_STARTUP`] to start.
fn report_slow_startup(observation: &Observation, env: &Environment<'_>) {
    let startup = observation.diagnostics().startup_ms.map(Duration::from_millis);
    if let Some(startup) = startup.filter(|startup| *startup > SLOW_STARTUP) {
        (env.slow_startup)(startup);
    }
}

/// [`StartObservation`] with this platform's native adapter.
pub fn native_observation(targets: Targets, moved: bool, interrupted: &dyn Fn() -> bool) -> Result<Observation, ObservationError> {
    if moved {
        Observation::start_at_quarantine(targets, ObserverOptions::default(), interrupted)
    } else {
        Observation::start(targets, ObserverOptions::default(), interrupted)
    }
}

/// How the CLI exits for an ended removal: `wt`'s exit codes 0, 1, 3, and 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitClass {
    /// 0: verified removal with every branch action done.
    Success,
    /// 1: a move happened, the state is uncertain, a branch action failed,
    /// or an operational error that changed nothing.
    Failure,
    /// 3: refused before anything changed; a fresh run or a fix to the
    /// worktree is needed.
    Refused,
    /// 4: this environment cannot run the lifecycle; nothing changed.
    Blocked,
}

/// Why nothing was moved or deleted. Earlier preparation (a repaired link)
/// is the caller's to report.
#[derive(Debug, thiserror::Error)]
pub enum Refusal {
    #[error(transparent)]
    Usage(UsageRefusal),
    /// A topology or state the move does not support.
    #[error(transparent)]
    Ineligible(MoveRefusal),
    #[error(transparent)]
    Inventory(InventoryRefusal),
    #[error(transparent)]
    Observation(ObservationError),
    /// Approved state (tip, files, index, Git's record) changed.
    #[error("{0}")]
    Changed(String),
    /// The checkout holds content that needs consent, and none was given.
    #[error("the worktree holds files that need consent to discard, and none was given")]
    ConsentRequired,
    #[error(transparent)]
    Index(MissingRefusal),
    #[error(transparent)]
    Store(StoreError),
    /// The move failed and the checkout and its registration are where they
    /// were ([`MoveError::nothing_changed`]).
    #[error(transparent)]
    Move(MoveError),
    #[error(transparent)]
    Git(WorktreeError),
    /// A pending operation that [`resume`] cannot continue; nothing is
    /// cleaned up automatically.
    #[error("removal {} cannot be resumed: {}", .0.id, .0.resolution)]
    Recovery(Box<Inspection>),
    /// [`Environment::interrupted`] held before the move, or before a
    /// resumed attempt's deletion.
    #[error("the removal was interrupted before it changed anything")]
    Interrupted,
}

impl Refusal {
    pub fn exit_class(&self) -> ExitClass {
        match self {
            Self::Inventory(InventoryRefusal::Coverage(_)) | Self::Observation(_) => ExitClass::Blocked,
            Self::Store(error) => match error {
                StoreError::Storage { .. } | StoreError::Io(_) | StoreError::JournalWrite(_) => ExitClass::Blocked,
                StoreError::Lock(LockError::Busy | LockError::Storage(_) | LockError::Io(_)) => ExitClass::Blocked,
                StoreError::Pending(_) | StoreError::Record { .. } | StoreError::Journal { .. } | StoreError::InventoryMismatch => {
                    ExitClass::Refused
                }
                StoreError::WrongLock | StoreError::PhaseOrder | StoreError::Completed(_) => ExitClass::Failure,
            },
            Self::Move(error) => match error.class() {
                MoveErrorClass::Refused => ExitClass::Refused,
                MoveErrorClass::BlockedByEnvironment => ExitClass::Blocked,
                MoveErrorClass::Failed => ExitClass::Failure,
            },
            Self::Git(_) | Self::Interrupted => ExitClass::Failure,
            Self::Usage(_)
            | Self::Ineligible(_)
            | Self::Inventory(_)
            | Self::Changed(_)
            | Self::ConsentRequired
            | Self::Index(_)
            | Self::Recovery(_) => ExitClass::Refused,
        }
    }
}

/// An operation that reached its end after the move, or resumed after one.
#[derive(Debug)]
pub struct Finished {
    /// Also saved at `report_path`, unless `warnings` says it could not be.
    pub report: OperationReport,
    pub report_path: PathBuf,
    /// Bookkeeping that failed without changing the outcome.
    pub warnings: Vec<String>,
}

impl Finished {
    pub fn exit_class(&self) -> ExitClass {
        if self.report.outcome == RemovalOutcome::Removed && !self.report.branch_failed() && self.report.errors.is_empty() {
            ExitClass::Success
        } else {
            ExitClass::Failure
        }
    }
}

/// How one removal attempt ended.
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

/// Removes the live, linked worktree `request.entry` through the whole
/// lifecycle (see the module docs).
pub fn remove_checkout(request: &RemovalRequest<'_>, env: &Environment<'_>) -> Removal {
    match run_with(request, env, &mut |_| false) {
        Ok(removal) => removal,
        Err(Crashed) => unreachable!("only a test hook stops the lifecycle"),
    }
}

/// A point the lifecycle passes, for crash-injection tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Boundary {
    /// The record is durable; nothing has moved.
    Begun,
    /// Moved and associated; nothing deleted.
    Moved,
    /// About to journal deletion batch `n` (batches before it are durable).
    DeletionBatch(u64),
    ContentRemoved,
    Unregistered,
    BranchesDone,
}

/// A test hook asked the lifecycle to stop at a [`Boundary`] as if the
/// process had died there: nothing more is written.
#[derive(Debug)]
pub(crate) struct Crashed;

/// Returns `true` to crash at the boundary. A deletion-batch boundary cannot
/// crash in-process; a hook blocks there for a real process kill instead.
pub(crate) type Hook<'h> = dyn FnMut(Boundary) -> bool + 'h;

pub(crate) fn run_with(request: &RemovalRequest<'_>, env: &Environment<'_>, hook: &mut Hook<'_>) -> Result<Removal, Crashed> {
    let refuse = |refusal: Refusal| Ok(Removal::Refused(Box::new(refusal)));
    let RemovalRequest { base, entry, requested_name, approval, branches } = *request;
    if approval.inventory.needs_consent() && !approval.discard_files {
        return refuse(Refusal::ConsentRequired);
    }
    let id = match OperationId::generate() {
        Ok(id) => id,
        Err(error) => return refuse(Refusal::Git(error)),
    };
    let plan = match plan_move(base, entry, &id) {
        Ok(plan) => plan,
        Err(refusal) => return refuse(Refusal::Ineligible(refusal)),
    };
    let store = match RecoveryStore::open(&plan.common_dir) {
        Ok(store) => store,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    // `begin` refuses these too; asking first spares the usage query.
    if let Err(error) = no_pending(&store, &plan.admin.dir) {
        return refuse(Refusal::Store(error));
    }
    let Some(targets) = Targets::new(&plan.original, &plan.quarantine, &plan.admin.dir) else {
        return refuse(Refusal::Ineligible(MoveRefusal::Unsupported("its quarantine is not a sibling".into())));
    };
    let mut timeline = Vec::new();
    let mut observation = match (env.observe)(targets.clone(), false, env.interrupted) {
        Ok(observation) => observation,
        Err(ObservationError::Interrupted) => return refuse(Refusal::Interrupted),
        Err(error) => return refuse(Refusal::Observation(error)),
    };
    report_slow_startup(&observation, env);
    timeline.push(event(Step::ObservationStarted, StepResult::Succeeded, None));
    let inventory = match DeletionInventory::capture(&plan.original, &approval.inventory.consent_sensitive_paths()) {
        Ok(inventory) => inventory,
        Err(refusal) => return refuse(Refusal::Inventory(refusal)),
    };
    timeline.push(event(Step::InventoryCaptured, StepResult::Succeeded, Some(format!("{} entries", inventory.entries().len()))));

    let mut own = OwnedHandles::default();
    observation.register_handles(&mut own);
    let discovered_at = now_ms();
    let clearance = match check_usage(env.usage, &plan.original, &own) {
        Ok(clearance) => clearance,
        Err(refusal) => return refuse(Refusal::Usage(refusal)),
    };
    let search_recheck = clearance.search_recheck.as_ref().map(|search| search.summary(true));
    timeline.push(event(Step::Discovery, StepResult::Succeeded, search_recheck));

    // After discovery and observation setup: the approval must still
    // describe the checkout.
    let current = || consent_state(base, &entry.path, entry.branch.as_deref()).map_err(Refusal::Git);
    let mut early_warnings = match recheck(base, entry, approval, current) {
        Ok(warnings) => warnings,
        Err(refusal) => return refuse(refusal),
    };
    // Rechecking ran `git status`, which may rewrite the index; the binding
    // is taken after it, so admin events up to here are its own.
    let mut early_activity =
        match confirm_unchanged(&inventory, &observation.drain(env.interrupted), &plan.original, Tree::AtOriginal, &targets, true) {
            Ok(activity) => activity,
            Err(refusal) => return refuse(refusal),
        };
    let binding = match bind_index(base, &plan.admin, approval.tip.clone()) {
        Ok(binding) => binding,
        Err(refusal) => return refuse(Refusal::Index(refusal)),
    };

    let record = match new_record(&plan, &id, requested_name, entry, approval, branches, clearance.report.clone()) {
        Ok(record) => record,
        Err(error) => return refuse(Refusal::Git(WorktreeError::Io(error))),
    };
    let lock = match store.lock(&record.repository) {
        Ok(lock) => lock,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let inventory = Arc::new(inventory);
    let operation = match store.begin(lock, record, Arc::clone(&inventory)) {
        Ok(operation) => operation,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    timeline.push(event(Step::RecordCreated, StepResult::Succeeded, None));
    if hook(Boundary::Begun) {
        return Err(Crashed);
    }

    if (env.interrupted)() {
        return Ok(discard(operation, Refusal::Interrupted));
    }
    // The last look before the move; anything since the binding is reported.
    match confirm_unchanged(&inventory, &observation.drain(env.interrupted), &plan.original, Tree::AtOriginal, &targets, false) {
        Ok(activity) => early_activity.extend(activity),
        Err(refusal) => return Ok(discard(operation, refusal)),
    }
    let moved = {
        let observation = RefCell::new(&mut observation);
        move_to_quarantine(base, &plan, &|| unexpected_before_rollback(&mut observation.borrow_mut(), env.interrupted))
    };

    let mut report = OperationReport::new(Some(id), RemovalOutcome::Removed, NativeString::from(plan.original.as_path()));
    report.timeline = timeline;
    report.discovery.extend(preflight_runs(discovered_at, NativeString::from(plan.original.as_path()), clearance));
    let mut attempt = Attempt::new(base, env, operation, report, branches, &plan.original, &plan.quarantine, plan.identity, &plan.admin.dir);
    attempt.observed_by(&observation);
    for observed in early_activity {
        attempt.report.push_activity(observed);
    }
    attempt.warnings.append(&mut early_warnings);
    match moved {
        Ok(()) => {}
        Err(error) if error.nothing_changed() => return Ok(discard(attempt.operation, Refusal::Move(error))),
        Err(error) => {
            attempt.step(Step::Moved, StepResult::Failed, Some(error.to_string()));
            if let MoveError::Stranded { state, .. } = &error
                && state.checkout == CheckoutLocation::Quarantine
                && let Err(write) = attempt.operation.advance_to(Phase::Moved)
            {
                attempt.warnings.push(format!("the recovery record could not be updated: {write}"));
            }
            return Ok(attempt.stop(RemovalOutcome::CleanupFailed, error));
        }
    }
    observation.moved_to_quarantine(env.interrupted);
    attempt.step(Step::Moved, StepResult::Succeeded, None);
    attempt.step(Step::AssociationUpdated, StepResult::Succeeded, None);
    attempt.report.original_state = OriginalState::Moved;
    attempt.report.registration = RegistrationState::AtQuarantine;
    attempt.report.quarantine = Some(QuarantineReport {
        path: NativeString::from(plan.quarantine.as_path()),
        identity: plan.identity,
        state: QuarantineState::Present,
        retained_entries: inventory.entries().len() as u64,
        retained: Vec::new(),
    });
    if let Err(error) = attempt.operation.advance(Phase::Associated) {
        return Ok(attempt.stop(RemovalOutcome::CleanupFailed, format!("the recovery record could not be updated: {error}")));
    }
    if hook(Boundary::Moved) {
        return Err(Crashed);
    }
    // The move's own events: explained here, since the deletion engine
    // treats every admin event as activity. The rest is reported.
    let own = observation.drain(env.interrupted);
    if own.interrupted {
        return Ok(attempt.stop(Stop::Interrupted.outcome(), Stop::Interrupted));
    }
    for observed in unexplained_move_events(&plan, &own) {
        attempt.report.push_activity(observed);
    }
    attempt.delete(observation, &binding, hook)
}

/// The report's runs for one preflight check that started at
/// `discovered_at`: the decisive query, preceded by the first one when only
/// Windows Search held files and the check rechecked.
fn preflight_runs(discovered_at: u64, path: NativeString, clearance: UsageClearance) -> Vec<DiscoveryRun> {
    let mut runs = Vec::new();
    let mut at_ms = discovered_at;
    if let Some(recheck) = clearance.search_recheck {
        runs.push(DiscoveryRun {
            at_ms,
            purpose: DiscoveryPurpose::WindowsSearchHold,
            path: path.clone(),
            result: DiscoveryResult::Report(Box::new(recheck.first)),
        });
        at_ms = ms(recheck.rechecked_at);
    }
    runs.push(DiscoveryRun { at_ms, purpose: DiscoveryPurpose::Preflight, path, result: DiscoveryResult::Report(Box::new(clearance.report)) });
    runs
}

/// Refuses while any operation of the worktree record `admin_dir` is
/// pending, or any record cannot be read (it might be one).
fn no_pending(store: &RecoveryStore, admin_dir: &Path) -> Result<(), StoreError> {
    for pending in store.pending()? {
        match pending.record {
            Ok(record) if record.repository.admin_name() == admin_dir.file_name() => return Err(StoreError::Pending(pending.id)),
            Ok(_) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// The approval's bindings read fresh: the branch tip, and the consented
/// inventory and fingerprint `current` collects ([`consent_state`] of the
/// original, or [`resume_consent`] on resume). The user agreed to the
/// removal, so a difference is a warning, not a refusal; an approved branch
/// deletion still compares the approved tip and fails safely if it moved.
fn recheck(
    base: &Path,
    entry: &WorktreeEntry,
    approval: &Approval,
    current: impl FnOnce() -> Result<(Inventory, String), Refusal>,
) -> Result<Vec<String>, Refusal> {
    let mut warnings = Vec::new();
    let tip = recorded_head(base, entry).map_err(Refusal::Git)?;
    if tip != approval.tip {
        warnings.push(format!("the worktree's HEAD moved from {} to {tip} after you agreed", approval.tip));
    }
    let (inventory, fingerprint) = current()?;
    if inventory.dirty != approval.inventory.dirty || fingerprint != approval.fingerprint {
        warnings.push("the worktree's files or staged changes changed after you agreed".into());
    }
    Ok(warnings)
}

/// Where the observed tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tree {
    AtOriginal,
    AtQuarantine,
}

/// The consistency check before a mutation, as warnings: every tree path
/// `drain` names is compared with `inventory` (the tree is at `root`), and
/// anything at the other location, an admin event (unless `admin_allowed`:
/// the recheck's own `git status`), and lost watching are reported. The
/// user agreed to the removal, so none of them refuses; the deletion engine
/// deletes what changed as it is. Only an interrupt, and a tree root that is
/// no longer the checkout (the move would move another directory), refuse.
fn confirm_unchanged(
    inventory: &DeletionInventory,
    drain: &Drain,
    root: &Path,
    tree: Tree,
    targets: &Targets,
    admin_allowed: bool,
) -> Result<Vec<ObservedActivity>, Refusal> {
    if drain.interrupted {
        return Err(Refusal::Interrupted);
    }
    let mut warnings = Vec::new();
    if drain.lost.is_some() {
        warnings.push(observed(ActivityLocation::Quarantine, root, ActivityKind::CoverageLost, None));
    }
    let mut tree_events = Vec::new();
    for event in &drain.events {
        let at = Some(event.received);
        match (&event.scope, tree) {
            (Scope::Admin(name), _) if !admin_allowed => {
                warnings.push(observed(ActivityLocation::Admin, &targets.admin().join(name), ActivityKind::Modified, at));
            }
            (Scope::Admin(_), _) => {}
            (Scope::Tree(_), _) | (Scope::Original(_), Tree::AtOriginal) => tree_events.push(event.clone()),
            (Scope::Quarantine(relative), Tree::AtQuarantine) => {
                let mut event = event.clone();
                event.scope = Scope::Tree(relative.clone());
                tree_events.push(event);
            }
            (Scope::Quarantine(relative), Tree::AtOriginal) => {
                warnings.push(observed(ActivityLocation::Quarantine, &targets.quarantine().join(relative), activity_kind(event.kind), at));
            }
            (Scope::Original(relative), Tree::AtQuarantine) => {
                warnings.push(observed(ActivityLocation::Original, &targets.original().join(relative), activity_kind(event.kind), at));
            }
        }
    }
    let location = if tree == Tree::AtOriginal { ActivityLocation::Original } else { ActivityLocation::Quarantine };
    match inventory.confirm(root, &Drain { events: tree_events, lost: None, interrupted: false }) {
        Ok(()) => {}
        Err(InventoryRefusal::Changed { path, .. } | InventoryRefusal::Uninspectable { path, .. }) if path == root => {
            return Err(Refusal::Changed(format!("{} is no longer the checkout that was approved", root.display())));
        }
        Err(InventoryRefusal::Changed { path, change }) => {
            use crate::remove::observation::inventory::Change;
            let kind = match change {
                Change::Appeared => ActivityKind::Created,
                Change::Vanished => ActivityKind::Vanished,
                Change::Replaced => ActivityKind::Replaced,
                Change::Modified => ActivityKind::Modified,
            };
            warnings.push(observed(location, &path, kind, None));
        }
        Err(InventoryRefusal::Uninspectable { path, .. } | InventoryRefusal::Ineligible { path, .. }) => {
            warnings.push(observed(location, &path, ActivityKind::Modified, None));
        }
        Err(InventoryRefusal::Coverage(_)) => {}
    }
    Ok(warnings)
}

/// Called by a move whose association failed, before it undoes the rename:
/// anything but the move's own events, lost coverage, or a wait the
/// interrupt ended keeps the quarantine in place.
fn unexpected_before_rollback(observation: &mut Observation, interrupted: &dyn Fn() -> bool) -> bool {
    let drain = observation.drain(interrupted);
    drain.lost.is_some()
        || drain.interrupted
        || drain.events.iter().any(|observed| match &observed.scope {
            Scope::Original(relative) | Scope::Quarantine(relative) | Scope::Tree(relative) => !relative.as_os_str().is_empty(),
            Scope::Admin(name) => name != "gitdir",
        })
}

/// The move's own events checked against what is there now: the original
/// moved away and absent, the quarantine (or the tree's root) moved in and
/// the checkout, and the admin `gitdir` holding the rewritten record.
/// Anything else is activity, and lost coverage is reported as such.
fn unexplained_move_events(plan: &MovePlan, drain: &Drain) -> Vec<ObservedActivity> {
    if drain.lost.is_some() {
        return vec![observed(ActivityLocation::Quarantine, &plan.quarantine, ActivityKind::CoverageLost, None)];
    }
    let cumulative = DeletionOptions::default().cumulative_kinds;
    let original_kinds: Vec<EventKind> = drain
        .events
        .iter()
        .filter(|observed| matches!(&observed.scope, Scope::Original(relative) if relative.as_os_str().is_empty()))
        .map(|observed| observed.kind)
        .collect();
    let moved_away = own_move(&original_kinds, cumulative) && absent(&plan.original);
    let moved_in = rename::identity(&plan.quarantine).is_ok_and(|identity| identity == plan.identity);
    let mut activity = Vec::new();
    for event in &drain.events {
        let (location, path) = match &event.scope {
            Scope::Original(relative) if relative.as_os_str().is_empty() && moved_away => continue,
            Scope::Quarantine(relative) | Scope::Tree(relative) if relative.as_os_str().is_empty() && moved_in => continue,
            Scope::Admin(name) if name == "gitdir" && plan.back_reference_rewritten() => continue,
            Scope::Original(relative) => (ActivityLocation::Original, plan.original.join(relative)),
            Scope::Quarantine(relative) | Scope::Tree(relative) => (ActivityLocation::Quarantine, plan.quarantine.join(relative)),
            Scope::Admin(name) => (ActivityLocation::Admin, plan.admin.dir.join(name)),
        };
        activity.push(observed(location, &path, activity_kind(event.kind), Some(event.received)));
    }
    activity
}

fn activity_kind(kind: EventKind) -> ActivityKind {
    match kind {
        EventKind::Created | EventKind::MovedTo => ActivityKind::Created,
        EventKind::Removed | EventKind::MovedFrom => ActivityKind::Vanished,
        EventKind::Modified | EventKind::Metadata => ActivityKind::Modified,
    }
}

fn observed(location: ActivityLocation, path: &Path, kind: ActivityKind, received: Option<SystemTime>) -> ObservedActivity {
    ObservedActivity { location, path: NativeString::from(path), kind, received_at_ms: ms(received.unwrap_or_else(SystemTime::now)) }
}

/// Ends an operation that changed nothing: its directory is deleted, so no
/// record describes a removal that never happened. A failure to delete it
/// leaves a record that [`inspect`] resolves as [`Resolution::NotStarted`].
fn discard(operation: Operation, refusal: Refusal) -> Removal {
    let _ = operation.discard();
    Removal::Refused(Box::new(refusal))
}

fn new_record(
    plan: &MovePlan,
    id: &OperationId,
    requested_name: &str,
    entry: &WorktreeEntry,
    approval: &Approval,
    branches: &BranchActions,
    discovery: sniff::filesystem::query::PathUsageReport,
) -> io::Result<RecoveryRecord> {
    let repository = RepositoryBinding {
        common_dir: plan.common_dir.as_path().into(),
        common_dir_identity: rename::identity(&plan.common_dir)?,
        admin_dir: plan.admin.dir.as_path().into(),
        admin_dir_identity: rename::identity(&plan.admin.dir)?,
    };
    let mut aliases = Vec::new();
    let directory = plan.original.file_name().map(|name| name.to_string_lossy().into_owned());
    for alias in entry.branch.iter().cloned().chain(directory) {
        if alias != requested_name && !alias.is_empty() && !aliases.contains(&alias) {
            aliases.push(alias);
        }
    }
    let mut intended_actions = vec![IntendedAction::RemoveCheckout];
    if let Some(local) = &branches.local {
        intended_actions.push(IntendedAction::DeleteLocalBranch { name: local.name.clone() });
    }
    if let Some(remote) = &branches.remote {
        intended_actions.push(IntendedAction::DeleteRemoteBranch { remote: remote.remote.clone(), name: remote.destination.clone() });
    }
    Ok(RecoveryRecord {
        record_version: RECORD_VERSION,
        operation_id: id.clone(),
        created_at_ms: now_ms(),
        requested_name: requested_name.to_string(),
        aliases,
        original: plan.original.as_path().into(),
        quarantine: plan.quarantine.as_path().into(),
        checkout_identity: plan.identity,
        repository,
        branch: entry.branch.clone().map_or(BranchBinding::Detached, BranchBinding::Named),
        approved_tip: approval.tip.clone(),
        consent_fingerprint: approval.fingerprint.clone(),
        discovery,
        intended_actions,
        completed_phase: Phase::Prepared,
    })
}

/// An operation from its move (or its resumption) on: everything a stop
/// needs to report the actual state.
struct Attempt<'a> {
    base: &'a Path,
    env: &'a Environment<'a>,
    operation: Operation,
    report: OperationReport,
    branches: &'a BranchActions,
    original: PathBuf,
    quarantine: PathBuf,
    identity: FileIdentity,
    admin_dir: PathBuf,
    warnings: Vec<String>,
    /// The report lists every journal entry, so completion may delete the
    /// journals.
    journal_reported: bool,
    /// The attempt's observer, read into the report whenever it is saved;
    /// `None` for an attempt that started none.
    observation: Option<DiagnosticsHandle>,
}

impl<'a> Attempt<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        base: &'a Path,
        env: &'a Environment<'a>,
        operation: Operation,
        report: OperationReport,
        branches: &'a BranchActions,
        original: &Path,
        quarantine: &Path,
        identity: FileIdentity,
        admin_dir: &Path,
    ) -> Self {
        Self {
            base,
            env,
            operation,
            report,
            branches,
            original: original.to_path_buf(),
            quarantine: quarantine.to_path_buf(),
            identity,
            admin_dir: admin_dir.to_path_buf(),
            warnings: Vec::new(),
            journal_reported: false,
            observation: None,
        }
    }

    fn observed_by(&mut self, observation: &Observation) {
        self.observation = Some(observation.diagnostics_handle());
    }

    fn record_observation(&mut self) {
        if let Some(observation) = &self.observation {
            self.report.observation = Some(observation.snapshot());
        }
    }

    fn step(&mut self, step: Step, result: StepResult, detail: Option<String>) {
        self.report.timeline.push(event(step, result, detail));
    }

    /// Deletes the quarantined checkout, unregisters it, then runs the
    /// branch actions.
    fn delete(mut self, mut observation: Observation, binding: &IndexBinding, hook: &mut Hook<'_>) -> Result<Removal, Crashed> {
        if let Err(error) = self.operation.advance_to(Phase::Deleting) {
            return Ok(self.stop(RemovalOutcome::CleanupFailed, format!("the recovery record could not be updated: {error}")));
        }
        let deletion = delete_content_with(
            &mut observation,
            self.operation.journal(),
            self.env.interrupted,
            DeletionOptions::default(),
            &mut |batch| {
                hook(Boundary::DeletionBatch(batch));
            },
        );
        self.report.record_deletion(&self.quarantine, &deletion);
        self.step(Step::DeletionBatch, StepResult::Succeeded, Some(format!("{} entries deleted", deletion.deleted.len())));
        if let Some(stop) = deletion.stop {
            return Ok(self.stop(stop.outcome(), stop));
        }
        self.step(Step::ContentRemoved, StepResult::Succeeded, None);
        if let Err(error) = self.operation.advance(Phase::ContentRemoved) {
            return Ok(self.stop(RemovalOutcome::CleanupFailed, format!("the recovery record could not be updated: {error}")));
        }
        if hook(Boundary::ContentRemoved) {
            return Err(Crashed);
        }
        match unregister_quarantine(self.base, &mut observation, binding, self.env.interrupted) {
            Ok(Unregistered { copy_warning, activity }) => {
                self.warnings.extend(copy_warning.map(|warning| format!("could not delete the copy record: {warning}")));
                let lost_before = self.report.activity.iter().any(|observed| observed.kind == ActivityKind::CoverageLost);
                for observed in activity {
                    // Watching is lost once; the deletion may have said so.
                    if !(lost_before && observed.kind == ActivityKind::CoverageLost) {
                        self.report.push_activity(observed);
                    }
                }
            }
            Err(error) => return Ok(self.stop(error.outcome(), error)),
        }
        drop(observation);
        self.unregistered(hook)
    }

    /// The registration is gone: records it, deletes the original's copy
    /// record, and runs the branch actions.
    fn unregistered(mut self, hook: &mut Hook<'_>) -> Result<Removal, Crashed> {
        self.step(Step::Unregistered, StepResult::Succeeded, None);
        self.report.registration = RegistrationState::Removed;
        if let Some(quarantine) = self.report.quarantine.as_mut() {
            quarantine.state = QuarantineState::Removed;
        }
        self.report.set_retained(Vec::new());
        if let Err(error) = self.operation.advance_to(Phase::Unregistered) {
            return Ok(self.stop(RemovalOutcome::CleanupFailed, format!("the recovery record could not be updated: {error}")));
        }
        // Only now: until the registration is verified gone, the copy record
        // is part of what a retry re-checks.
        if let Some(warning) = crate::copy_record::delete_for(self.base, &self.original) {
            self.warnings.push(format!("could not delete the copy record: {warning}"));
        }
        if hook(Boundary::Unregistered) {
            return Err(Crashed);
        }
        self.branch_actions(hook)
    }

    /// Local branch, then remote; a failure leaves the rest not attempted
    /// and the record pending.
    fn branch_actions(mut self, hook: &mut Hook<'_>) -> Result<Removal, Crashed> {
        let mut failed = false;
        if let Some(local) = &self.branches.local {
            let state = match remove_local_branch(self.base, &local.name, &local.expected_tip) {
                Ok(()) => BranchState::Deleted,
                Err(error) => {
                    failed = true;
                    BranchState::Failed { error: error.to_string() }
                }
            };
            self.branch_step(BranchTarget::Local { name: local.name.clone() }, state);
        }
        if let Some(remote) = &self.branches.remote {
            let target = BranchTarget::Remote { remote: remote.remote.clone(), name: remote.destination.clone() };
            let state = if failed {
                BranchState::NotAttempted
            } else {
                match delete_remote_branch(self.base, &remote.endpoint, &remote.destination, &remote.expected_sha) {
                    Ok(()) => BranchState::Deleted,
                    Err(error) => {
                        failed = true;
                        BranchState::Failed { error }
                    }
                }
            };
            self.branch_step(target, state);
        }
        self.report.outcome = RemovalOutcome::Removed;
        self.summarize_journal();
        if failed {
            self.save_report();
            return Ok(self.finished());
        }
        if let Err(error) = self.operation.advance_to(Phase::BranchActions) {
            self.warnings.push(format!("the recovery record could not be updated: {error}"));
            self.save_report();
            return Ok(self.finished());
        }
        if hook(Boundary::BranchesDone) {
            return Err(Crashed);
        }
        self.step(Step::Completed, StepResult::Succeeded, None);
        self.record_observation();
        let Self { operation, report, mut warnings, journal_reported, .. } = self;
        let report_path = operation.report_path();
        match operation.complete(&report, journal_reported) {
            Ok(bookkeeping) => warnings.extend(bookkeeping.into_iter().map(|BookkeepingWarning::JournalNotCleared { path, error }| {
                format!("could not delete the deletion journal {}: {error}", path.display())
            })),
            Err(error) => warnings.push(format!("the removal's completion could not be recorded: {error}")),
        }
        if !journal_reported {
            warnings.push("the deletion journals were kept, since the report could not list their entries".into());
        }
        Ok(Removal::Finished(Box::new(Finished { report, report_path, warnings })))
    }

    fn branch_step(&mut self, target: BranchTarget, state: BranchState) {
        let result = match &state {
            BranchState::Deleted | BranchState::Kept => StepResult::Succeeded,
            BranchState::NotAttempted => return self.report.branches.push(BranchOutcome { target, state }),
            BranchState::Failed { .. } => StepResult::Failed,
        };
        self.step(Step::BranchAction, result, None);
        self.report.branches.push(BranchOutcome { target, state });
    }

    /// Ends the operation where it stands: the actual state of both paths
    /// and of Git's registration, a diagnostic usage query of each real
    /// directory there, and the report saved beside the pending record.
    fn stop(mut self, outcome: RemovalOutcome, error: impl Display) -> Removal {
        self.report.outcome = outcome;
        self.report.errors.push(error.to_string());
        self.describe_locations();
        self.summarize_journal();
        for target in self.intended_branch_targets() {
            self.report.branches.push(BranchOutcome { target, state: BranchState::NotAttempted });
        }
        self.save_report();
        self.finished()
    }

    fn intended_branch_targets(&self) -> Vec<BranchTarget> {
        let local = self.branches.local.iter().map(|local| BranchTarget::Local { name: local.name.clone() });
        let remote = self
            .branches
            .remote
            .iter()
            .map(|remote| BranchTarget::Remote { remote: remote.remote.clone(), name: remote.destination.clone() });
        local.chain(remote).collect()
    }

    fn describe_locations(&mut self) {
        let original = presence(&self.original, &self.identity);
        self.report.original_state = match &original {
            Presence::Absent => OriginalState::Moved,
            Presence::Checkout => OriginalState::Kept,
            Presence::Other => OriginalState::Recreated,
            Presence::Uninspectable(_) => OriginalState::Unknown,
        };
        let quarantine = presence(&self.quarantine, &self.identity);
        let state = match &quarantine {
            Presence::Absent => QuarantineState::Removed,
            Presence::Checkout => QuarantineState::Present,
            Presence::Other | Presence::Uninspectable(_) => QuarantineState::Unknown,
        };
        let report = self.report.quarantine.get_or_insert_with(|| QuarantineReport {
            path: NativeString::from(self.quarantine.as_path()),
            identity: self.identity,
            state,
            retained_entries: 0,
            retained: Vec::new(),
        });
        report.state = state;
        match quarantine {
            Presence::Checkout => match retained_entries(&self.quarantine, &self.identity) {
                Ok(entries) => self.report.set_retained(entries),
                Err(error) => self.report.errors.push(format!("what remains in the quarantine could not be listed: {error}")),
            },
            _ => self.report.set_retained(Vec::new()),
        }
        self.report.registration = registration_state(&self.admin_dir, &self.original, &self.quarantine);

        // Diagnostics only: who might be using what is left. A real directory
        // is queried; a replacement link is never followed to gather this.
        for (path, at) in [(&self.original, &original), (&self.quarantine, &quarantine)] {
            if matches!(availability::inspect(path), Ok(EntryKind::Directory)) && !matches!(at, Presence::Uninspectable(_)) {
                let result = match self.env.usage.query(path) {
                    Ok(report) => DiscoveryResult::Report(Box::new(report)),
                    Err(error) => DiscoveryResult::Failed { kind: error.kind().to_string(), message: error.to_string() },
                };
                self.report.discovery.push(DiscoveryRun {
                    at_ms: now_ms(),
                    purpose: DiscoveryPurpose::Diagnostic,
                    path: NativeString::from(path.as_path()),
                    result,
                });
            }
        }
    }

    /// Fills the report's journal counts, entries, and deleted paths from
    /// every attempt's journal, asking the quarantine (through a verified
    /// handle) about the current attempt's unconfirmed entries.
    fn summarize_journal(&mut self) {
        let quarantine = presence(&self.quarantine, &self.identity);
        let handle = match quarantine {
            Presence::Checkout => super::observation::inventory::open_root(&self.quarantine, Some(&self.identity)).ok().map(|(dir, _)| dir),
            _ => None,
        };
        let entries = self.operation.journal_entries(|relative| match (&quarantine, &handle) {
            (Presence::Absent, _) => Ok(false),
            (_, Some(dir)) => match dir.symlink_metadata(relative) {
                Ok(_) => Ok(true),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            },
            _ => Err(io::Error::other("the quarantine is not the moved checkout")),
        });
        match entries {
            Ok(entries) => {
                self.report.journal = summarize(&entries);
                let deleted = entries.iter().filter(|entry| entry.state == EntryState::Deleted);
                self.report.set_deleted(deleted.map(|entry| NativeString::from(self.quarantine.join(entry.path.as_path()).as_path())).collect());
                if self.report.journal.deleted.total() > 0 && !self.report.notes.contains(&ReportNote::ConcurrentWriteLimit) {
                    self.report.notes.push(ReportNote::ConcurrentWriteLimit);
                }
                self.report.journal_entries = entries;
                self.journal_reported = true;
            }
            Err(error) => {
                self.journal_reported = false;
                self.report.errors.push(format!("the deletion journal could not be summarized: {error}"));
            }
        }
    }

    fn save_report(&mut self) {
        self.record_observation();
        if let Err(error) = self.operation.save_report(&self.report) {
            self.warnings.push(format!("the report could not be saved: {error}"));
        }
    }

    fn finished(self) -> Removal {
        let report_path = self.operation.report_path();
        Removal::Finished(Box::new(Finished { report: self.report, report_path, warnings: self.warnings }))
    }
}

/// What is at one of an operation's two paths, read without following a
/// link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presence {
    Absent,
    /// A real directory with the checkout's identity.
    Checkout,
    /// Something else: never deleted, traversed, or overwritten.
    Other,
    Uninspectable(String),
}

pub(crate) fn presence(path: &Path, identity: &FileIdentity) -> Presence {
    match availability::inspect(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Presence::Absent,
        Err(error) => Presence::Uninspectable(error.to_string()),
        Ok(EntryKind::Directory) => match rename::identity(path) {
            Ok(found) if found == *identity => Presence::Checkout,
            Ok(_) => Presence::Other,
            Err(error) => Presence::Uninspectable(error.to_string()),
        },
        Ok(_) => Presence::Other,
    }
}

/// Where the worktree record `admin_dir` points, read from the record itself.
pub(crate) fn registration_state(admin_dir: &Path, original: &Path, quarantine: &Path) -> RegistrationState {
    match read_back_reference(admin_dir) {
        Ok(named) if same_location(&named, &original.join(".git")) => RegistrationState::AtOriginal,
        Ok(named) if same_location(&named, &quarantine.join(".git")) => RegistrationState::AtQuarantine,
        Err(BackReferenceError::Missing) if absent(admin_dir) => RegistrationState::Removed,
        _ => RegistrationState::Unknown,
    }
}

/// Every entry left in the quarantine, relative to it, without following
/// links.
pub(crate) fn retained_entries(quarantine: &Path, identity: &FileIdentity) -> Result<Vec<NativeString>, InventoryRefusal> {
    let listing = DeletionInventory::capture(quarantine, &[])?;
    if listing.root() != identity {
        return Err(InventoryRefusal::Changed { path: quarantine.to_path_buf(), change: super::observation::inventory::Change::Replaced });
    }
    Ok(listing.entries().iter().map(|entry| entry.path.clone()).collect())
}

fn absent(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

fn event(step: Step, result: StepResult, detail: Option<String>) -> TimelineEvent {
    TimelineEvent { at_ms: now_ms(), step, result, detail }
}

fn now_ms() -> u64 {
    ms(SystemTime::now())
}

fn ms(at: SystemTime) -> u64 {
    at.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}
