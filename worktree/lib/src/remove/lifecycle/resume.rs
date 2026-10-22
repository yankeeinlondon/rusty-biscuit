//! Picking up a pending removal: what an interrupted or stopped operation
//! left, read from the filesystem and Git rather than from its saved phase,
//! and the retry that continues it under fresh consent.
//!
//! A pending record describes what happened; it never authorizes a deletion.
//! [`resume`] re-collects everything an approval binds (a fresh inventory of
//! what survives, the consent fingerprint, the index, the branch tip) and
//! runs the same checked steps as a first attempt. A recreated original, a
//! replaced quarantine, or a checkout Git can no longer read is never cleaned
//! up automatically: [`inspect`] says what is where, and the caller explains
//! the manual way out.

use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::Path;
use std::sync::Arc;

use sniff::filesystem::query::NativeString;

use super::{
    Approval, Attempt, BranchActions, Boundary, Crashed, Environment, Hook, Presence, Refusal, Removal, ResumeConsent, Tree,
    check_usage, confirm_unchanged, consent_state, event, explain_deletions, now_ms, preflight_runs, presence, recheck,
    registration_state, report_slow_startup, retained_entries,
};
use crate::availability::{self, EntryKind};
use crate::remove::admin_entry::{AdminEntry, common_git_dir, same_location};
use crate::remove::missing::{MissingCheckout, MissingRefusal, bind_index, remove_missing_record};
use crate::remove::observation::{Drain, Observation, ObservationError, Targets};
use crate::remove::observation::inventory::DeletionInventory;
use crate::remove::outcome::{
    JournalSummary, OperationReport, OriginalState, QuarantineReport,
    QuarantineState, RegistrationState, RemovalOutcome, Step, StepResult,
};
use crate::remove::quarantine::association::{MoveRefusal, verify_quarantined};
use crate::remove::quarantine::rename;
use crate::remove::recovery::OperationId;
use crate::remove::inventory::bind_excluded;
use crate::remove::recovery::journal::{EntryState, summarize};
use crate::remove::recovery::record::{BranchBinding, RecoveryRecord, RepositoryBinding};
use crate::remove::recovery::space::NativeSpace;
use crate::remove::recovery::store::{RecoveryStore, StoreError};
use crate::remove::usage::OwnedHandles;
use crate::worktree::{WorktreeEntry, parse_worktree_list};

/// A pending operation as it stands now.
#[derive(Debug)]
pub struct Inspection {
    pub id: OperationId,
    pub record: RecoveryRecord,
    pub original: Presence,
    pub quarantine: Presence,
    pub registration: RegistrationState,
    /// Every attempt's journal, the current one's unconfirmed entries
    /// resolved against the quarantine; `None` when the quarantine is not the
    /// moved checkout, so nothing in it could be checked.
    pub journal: Option<JournalSummary>,
    /// The journal's last line was cut short by a crash.
    pub torn_tail: bool,
    /// What is left in the quarantine, relative to it, when it is the moved
    /// checkout and could be listed.
    pub retained: Option<Vec<NativeString>>,
    pub resolution: Resolution,
}

/// What a retry can do with a pending operation, decided from what is at
/// both paths and in Git now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Nothing moved: the checkout is at its original path, registered
    /// there. [`release_unstarted`] closes the record so a new removal can
    /// start.
    NotStarted,
    /// The checkout was renamed but Git's record still names the original
    /// path: the process stopped between the two. [`release_unstarted`]
    /// moves it back.
    MovedUnassociated,
    /// The checkout is in the quarantine and Git's worktree there; a retry
    /// deletes what survives after fresh consent.
    Quarantined,
    /// The quarantine is the moved checkout, but its `.git` link is gone, so
    /// Git cannot read what is left. Nothing is repaired; the remaining
    /// entries and the worktree's record are kept for manual recovery.
    PartialCheckout,
    /// The moved checkout's content is gone and Git still registers the
    /// quarantine; a retry removes that record after re-checking its index.
    ContentRemoved,
    /// The checkout and its registration are gone; a retry runs the branch
    /// actions approved again, then closes the record.
    CheckoutRemoved,
    /// Something that is not the checkout is at the original path. Neither
    /// tree is cleaned up until the user resolves it.
    OriginalRecreated,
    /// Anything else, with the reason; everything is kept.
    Unresumable(String),
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => f.write_str("nothing was moved yet"),
            Self::MovedUnassociated => f.write_str("the checkout was moved aside, but Git still records its original path"),
            Self::Quarantined => f.write_str("the checkout is moved aside and can be deleted after a fresh check"),
            Self::PartialCheckout => f.write_str("what is left of the moved checkout no longer has its .git link"),
            Self::ContentRemoved => f.write_str("the checkout's files are gone and only Git's record of it remains"),
            Self::CheckoutRemoved => f.write_str("the checkout is removed and only branch actions remain"),
            Self::OriginalRecreated => f.write_str("something new is at the worktree's original path"),
            Self::Unresumable(reason) => f.write_str(reason),
        }
    }
}

/// Inspects pending operation `id` of the repository at `base`.
///
/// ## Errors
///
/// A [`StoreError`]: the record or journal cannot be read or validated, it
/// belongs to another repository, or the operation already completed.
pub fn inspect(base: &Path, id: &OperationId) -> Result<Inspection, StoreError> {
    inspect_in(&open_store(base)?, id)
}

fn open_store(base: &Path) -> Result<RecoveryStore, StoreError> {
    let common = common_git_dir(base).map_err(|error| StoreError::Io(io::Error::other(error.to_string())))?;
    RecoveryStore::open(&common)
}

pub(super) fn inspect_in(store: &RecoveryStore, id: &OperationId) -> Result<Inspection, StoreError> {
    let record = store.read_record(id)?;
    let original_path = record.original.as_path();
    let quarantine_path = record.quarantine.as_path();
    let identity = record.checkout_identity;
    let original = presence(original_path, &identity);
    let quarantine = presence(quarantine_path, &identity);
    let (expected, admin_replaced) = current_binding(&record)?;
    let registration = if admin_replaced {
        RegistrationState::Unknown
    } else {
        registration_state(record.repository.admin_dir.as_path(), original_path, quarantine_path)
    };

    let handle = match quarantine {
        Presence::Checkout => crate::remove::observation::inventory::open_root(quarantine_path, Some(&identity)).ok().map(|(dir, _)| dir),
        _ => None,
    };
    let checkable = matches!(quarantine, Presence::Absent) || handle.is_some();
    let recovered = store.recover(id, &expected, |relative| match &handle {
        Some(dir) => match dir.symlink_metadata(relative) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        },
        // Absent: nothing of it is left. Otherwise nothing was checked, and
        // the summary is dropped below.
        None => Ok(!matches!(quarantine, Presence::Absent)),
    })?;
    let journal = checkable.then(|| summarize(&recovered.entries));
    let retained = handle.is_some().then(|| retained_entries(quarantine_path, &identity).ok()).flatten();

    let resolution = resolve(&original, &quarantine, registration, admin_replaced, quarantine_path);
    Ok(Inspection { id: id.clone(), record, original, quarantine, registration, journal, torn_tail: recovered.torn_tail, retained, resolution })
}

fn resolve(original: &Presence, quarantine: &Presence, registration: RegistrationState, admin_replaced: bool, quarantine_path: &Path) -> Resolution {
    use Presence::{Absent, Checkout, Other, Uninspectable};
    use RegistrationState::{AtOriginal, AtQuarantine, Removed};
    if admin_replaced {
        return Resolution::Unresumable("Git's record of this worktree now belongs to another worktree".into());
    }
    match (original, quarantine, registration) {
        (Other, _, _) => Resolution::OriginalRecreated,
        (Uninspectable(error), _, _) => Resolution::Unresumable(format!("the original path can't be inspected: {error}")),
        (_, Uninspectable(error), _) => Resolution::Unresumable(format!("the moved checkout's path can't be inspected: {error}")),
        (_, Other, _) => Resolution::Unresumable("something other than the moved checkout is at its path".into()),
        (Checkout, Absent, AtOriginal) => Resolution::NotStarted,
        (Absent, Checkout, AtOriginal) => Resolution::MovedUnassociated,
        (Absent, Checkout, AtQuarantine) if !matches!(availability::inspect(&quarantine_path.join(".git")), Ok(EntryKind::File)) => {
            Resolution::PartialCheckout
        }
        (Absent, Checkout, AtQuarantine) => Resolution::Quarantined,
        (Absent, Absent, AtQuarantine) => Resolution::ContentRemoved,
        (Absent, Absent, Removed) => Resolution::CheckoutRemoved,
        (original, quarantine, registration) => Resolution::Unresumable(format!(
            "the checkout's locations ({original:?} at the original path, {quarantine:?} at the quarantine) and Git's \
            record ({registration:?}) do not describe any step of a removal"
        )),
    }
}

/// The repository binding to check the record against, read fresh: the
/// common directory's identity, and the admin directory's while it exists.
/// The flag is set when a directory with another identity has the admin
/// directory's name (Git reused it for a new worktree); the record's own
/// identity is then used, so the record can still be read.
fn current_binding(record: &RecoveryRecord) -> Result<(RepositoryBinding, bool), StoreError> {
    let common_dir_identity = rename::identity(record.repository.common_dir.as_path())?;
    let admin = record.repository.admin_dir.as_path();
    let (admin_dir_identity, replaced) = match rename::identity(admin) {
        Ok(identity) if identity == record.repository.admin_dir_identity => (identity, false),
        Ok(_) => (record.repository.admin_dir_identity, true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (record.repository.admin_dir_identity, false),
        Err(error) => return Err(error.into()),
    };
    let binding = RepositoryBinding {
        common_dir: record.repository.common_dir.clone(),
        common_dir_identity,
        admin_dir: record.repository.admin_dir.clone(),
        admin_dir_identity,
    };
    Ok((binding, replaced))
}

/// Closes pending operation `id` when it changed nothing
/// ([`Resolution::NotStarted`]), first moving the checkout back when only
/// the rename happened ([`Resolution::MovedUnassociated`]), so a new removal
/// can start from the original path.
///
/// ## Errors
///
/// [`Refusal::Recovery`] for any other resolution; [`Refusal::Store`] when
/// the record or its lock cannot be used; [`Refusal::Changed`] when moving
/// back did not restore the checkout and its registration.
pub fn release_unstarted(base: &Path, id: &OperationId) -> Result<(), Refusal> {
    let store = open_store(base).map_err(Refusal::Store)?;
    let record = store.read_record(id).map_err(Refusal::Store)?;
    let lock = store.lock(&record.repository).map_err(Refusal::Store)?;
    let inspection = inspect_in(&store, id).map_err(Refusal::Store)?;
    let original = record.original.as_path();
    let quarantine = record.quarantine.as_path();
    match inspection.resolution {
        Resolution::NotStarted => {}
        Resolution::MovedUnassociated => {
            // No replacement: a directory that appeared at the original
            // since the inspection stays, and this fails.
            rename::rename_no_replace(quarantine, original)
                .map_err(|error| Refusal::Changed(format!("the checkout could not be moved back: {error}")))?;
            let admin = record.repository.admin_dir.as_path();
            if presence(original, &record.checkout_identity) != Presence::Checkout
                || registration_state(admin, original, quarantine) != RegistrationState::AtOriginal
            {
                return Err(Refusal::Changed("the checkout was moved back, but could not be verified at its original path".into()));
            }
        }
        _ => return Err(Refusal::Recovery(Box::new(inspection))),
    }
    let (expected, _) = current_binding(&record).map_err(Refusal::Store)?;
    let recovered = store.recover(id, &expected, |_| Ok(true)).map_err(Refusal::Store)?;
    let operation = store.reopen(lock, id, &expected, Arc::new(recovered.inventory), &NativeSpace).map_err(Refusal::Store)?;
    operation.discard().map_err(Refusal::Store)
}

/// Consent to remove a registration whose checkout is gone
/// ([`Resolution::ContentRemoved`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordApproval {
    /// [`inspect_missing`](crate::remove::missing::inspect_missing) of the
    /// quarantine's registration, as shown to the user.
    pub inspected: MissingCheckout,
    /// Consent to discard the staged changes `inspected` lists.
    pub discard_staged: bool,
}

/// A retry of pending operation `id`, with approvals collected again.
#[derive(Debug, Clone, Copy)]
pub struct ResumeRequest<'a> {
    pub base: &'a Path,
    pub id: &'a OperationId,
    /// For [`Resolution::Quarantined`]: consent for what survives, from
    /// [`resume_consent`]'s `inventory` and `fingerprint`.
    pub approval: Option<&'a Approval>,
    /// For [`Resolution::ContentRemoved`].
    pub record: Option<&'a RecordApproval>,
    pub branches: &'a BranchActions,
}

/// The consent a retry of pending operation `id` asks for, read from its
/// moved checkout: every attempt's journal is read and validated first, and
/// a dirty entry leaves consent only when its own deletion by this operation
/// explains it (see [`ResumeConsent::excluded`]). [`resume`] repeats this
/// under the operation lock and refuses unless the approval still matches.
///
/// ## Errors
///
/// [`Refusal::Store`] when the record or a journal cannot be read or
/// validated (the recovery refusal, which enables no exclusion);
/// [`Refusal::Inventory`] when the moved checkout cannot be opened as
/// itself; [`Refusal::Git`] as for [`consent_state`].
pub fn resume_consent(base: &Path, id: &OperationId) -> Result<ResumeConsent, Refusal> {
    let store = open_store(base).map_err(Refusal::Store)?;
    let record = store.read_record(id).map_err(Refusal::Store)?;
    let (expected, _) = current_binding(&record).map_err(Refusal::Store)?;
    resume_consent_in(&store, &record, &expected, base)
}

fn resume_consent_in(store: &RecoveryStore, record: &RecoveryRecord, expected: &RepositoryBinding, base: &Path) -> Result<ResumeConsent, Refusal> {
    let quarantine = record.quarantine.as_path();
    let (dir, _) = crate::remove::observation::inventory::open_root(quarantine, Some(&record.checkout_identity)).map_err(Refusal::Inventory)?;
    let present = |relative: &Path| match dir.symlink_metadata(relative) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    };
    let recovered = store.recover(&record.operation_id, expected, present).map_err(Refusal::Store)?;
    let branch = match &record.branch {
        BranchBinding::Named(branch) => Some(branch.as_str()),
        BranchBinding::Detached => None,
    };
    let (raw, raw_fingerprint) = consent_state(base, quarantine, branch).map_err(Refusal::Git)?;
    let deleted: HashSet<&Path> =
        recovered.entries.iter().filter(|entry| entry.state == EntryState::Deleted).map(|entry| entry.path.as_path()).collect();
    let (inventory, excluded) = explain_deletions(&raw, &deleted, |relative| matches!(present(relative), Ok(false)));
    let fingerprint = bind_excluded(&raw_fingerprint, &excluded);
    Ok(ResumeConsent { raw, inventory, excluded, fingerprint })
}

/// Continues pending operation `id` from what [`inspect`] finds, under
/// fresh approvals: [`Resolution::Quarantined`] deletes what survives and
/// unregisters, [`Resolution::ContentRemoved`] removes the registration, and
/// [`Resolution::CheckoutRemoved`] runs the branch actions. Any other
/// resolution refuses with [`Refusal::Recovery`].
///
/// For a quarantine, [`resume_consent`]'s calculation runs again under the
/// operation lock before anything more is deleted; unless the request's
/// approval matches it, the retry refuses with [`Refusal::Changed`].
pub fn resume(request: &ResumeRequest<'_>, env: &Environment<'_>) -> Removal {
    match resume_with(request, env, &mut |_| false) {
        Ok(removal) => removal,
        Err(Crashed) => unreachable!("only a test hook stops the lifecycle"),
    }
}

pub(crate) fn resume_with(request: &ResumeRequest<'_>, env: &Environment<'_>, hook: &mut Hook<'_>) -> Result<Removal, Crashed> {
    let refuse = |refusal: Refusal| Ok(Removal::Refused(Box::new(refusal)));
    let store = match open_store(request.base) {
        Ok(store) => store,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let record = match store.read_record(request.id) {
        Ok(record) => record,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let lock = match store.lock(&record.repository) {
        Ok(lock) => lock,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let inspection = match inspect_in(&store, request.id) {
        Ok(inspection) => inspection,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let (expected, _) = match current_binding(&record) {
        Ok(binding) => binding,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let resumed = Resumed { request, env, store: &store, record: &record, expected };
    match inspection.resolution {
        Resolution::Quarantined => resumed.deletion(lock, hook),
        Resolution::ContentRemoved => resumed.registration(lock, hook),
        Resolution::CheckoutRemoved => resumed.branches(lock, hook),
        _ => refuse(Refusal::Recovery(Box::new(inspection))),
    }
}

struct Resumed<'r, 'a> {
    request: &'r ResumeRequest<'a>,
    env: &'r Environment<'a>,
    store: &'r RecoveryStore,
    record: &'r RecoveryRecord,
    expected: RepositoryBinding,
}

impl<'r, 'a> Resumed<'r, 'a> {
    fn original(&self) -> &Path {
        self.record.original.as_path()
    }

    fn quarantine(&self) -> &Path {
        self.record.quarantine.as_path()
    }

    fn admin(&self) -> &Path {
        self.record.repository.admin_dir.as_path()
    }

    fn report(&self) -> OperationReport {
        let mut report = OperationReport::new(Some(self.record.operation_id.clone()), RemovalOutcome::Removed, self.record.original.clone());
        report.original_state = OriginalState::Moved;
        report.timeline.push(event(Step::RecordCreated, StepResult::Succeeded, Some("resumed".into())));
        report
    }

    /// Reopens the operation with `inventory` and wraps it for the shared
    /// steps, carrying the report an earlier attempt saved into `report`.
    fn attempt(
        &self,
        lock: crate::remove::recovery::lock::OperationLock,
        inventory: Arc<DeletionInventory>,
        report: OperationReport,
    ) -> Result<Attempt<'r>, Refusal>
    where
        'a: 'r,
    {
        let id = &self.record.operation_id;
        let earlier = self.store.read_report(id);
        let operation = self.store.reopen(lock, id, &self.expected, inventory, self.env.space).map_err(Refusal::Store)?;
        let mut attempt = Attempt::new(
            self.request.base,
            self.env,
            operation,
            report,
            self.request.branches,
            self.original(),
            self.quarantine(),
            self.record.checkout_identity,
            self.admin(),
        );
        match earlier {
            Ok(Some(earlier)) => attempt.report.continue_from(earlier),
            // The earlier attempt crashed before saving one.
            Ok(None) => {}
            Err(error) => attempt.warnings.push(format!("the earlier attempt's report could not be read, so it is not carried: {error}")),
        }
        Ok(attempt)
    }

    /// The inventory of a retry that deletes nothing: empty, since the
    /// quarantine is gone. An earlier attempt's unconfirmed entry is then
    /// never taken as still present (see
    /// [`Operation::journal_entries`](crate::remove::recovery::store::Operation::journal_entries)).
    fn nothing_to_delete(&self) -> Arc<DeletionInventory> {
        Arc::new(DeletionInventory::empty(self.record.checkout_identity))
    }

    /// [`Resolution::Quarantined`]: the first attempt's steps from
    /// observation on, against the quarantine and a fresh inventory.
    fn deletion(self, lock: crate::remove::recovery::lock::OperationLock, hook: &mut Hook<'_>) -> Result<Removal, Crashed>
    where
        'a: 'r,
    {
        let refuse = |refusal: Refusal| Ok(Removal::Refused(Box::new(refusal)));
        let Some(approval) = self.request.approval else {
            return refuse(Refusal::ConsentRequired);
        };
        if approval.inventory.needs_consent() && !approval.discard_files {
            return refuse(Refusal::ConsentRequired);
        }
        let (base, quarantine, identity) = (self.request.base, self.quarantine(), self.record.checkout_identity);
        let entry = match verify_quarantined(base, quarantine, &identity, self.admin()) {
            Ok(entry) => entry,
            Err(refusal) => return refuse(Refusal::Ineligible(refusal)),
        };
        let Some(targets) = Targets::new(self.original(), quarantine, self.admin()) else {
            return refuse(Refusal::Ineligible(MoveRefusal::Unsupported("its quarantine is not a sibling".into())));
        };
        let mut observation = match (self.env.observe)(targets, true, self.env.interrupted) {
            Ok(observation) => observation,
            Err(ObservationError::Interrupted) => return refuse(Refusal::Interrupted),
            Err(error) => return refuse(Refusal::Observation(error)),
        };
        report_slow_startup(&observation, self.env);
        let inventory = match DeletionInventory::capture(quarantine, &approval.inventory.consent_sensitive_paths()) {
            Ok(inventory) => inventory,
            Err(refusal) => return refuse(Refusal::Inventory(refusal)),
        };
        let index = Arc::new(inventory.tree_index());
        let mut own = OwnedHandles::default();
        observation.register_handles(&mut own);
        let discovered_at = now_ms();
        let clearance = match check_usage(self.env.usage, quarantine, &own, Some(&index)) {
            Ok(clearance) => clearance,
            Err(refusal) => return refuse(Refusal::Usage(refusal)),
        };
        // Under the operation lock: no other attempt can add a deletion
        // result between this and the deletion it approves.
        let current = || resume_consent_in(self.store, self.record, &self.expected, base).map(|consent| (consent.inventory, consent.fingerprint));
        if let Err(refusal) = recheck(base, &entry, approval, current) {
            return refuse(refusal);
        }
        if let Err(refusal) = confirm_unchanged(&inventory, &checked_drain(&mut observation, self.env.interrupted), quarantine, Tree::AtQuarantine, true) {
            return refuse(refusal);
        }
        let binding = match bind_index(base, &AdminEntry { dir: self.admin().to_path_buf() }, approval.tip.clone()) {
            Ok(binding) => binding,
            Err(refusal) => return refuse(Refusal::Index(refusal)),
        };
        let inventory = Arc::new(inventory);
        let mut report = self.report();
        report.registration = RegistrationState::AtQuarantine;
        report.quarantine = Some(QuarantineReport {
            path: self.record.quarantine.clone(),
            identity,
            state: QuarantineState::Present,
            retained_entries: inventory.entries().len() as u64,
            retained: Vec::new(),
        });
        if let Some(search) = &clearance.search_recheck {
            report.timeline.push(event(Step::Discovery, StepResult::Succeeded, Some(search.summary(true))));
        }
        report.discovery.extend(preflight_runs(discovered_at, self.record.quarantine.clone(), clearance));
        let mut attempt = match self.attempt(lock, Arc::clone(&inventory), report) {
            Ok(attempt) => attempt,
            Err(refusal) => return refuse(refusal),
        };
        attempt.observed_by(&observation);
        if hook(Boundary::Begun) {
            return Err(Crashed);
        }
        // The last look before deleting; this attempt has changed nothing
        // yet, and the operation stays pending as it was.
        if let Err(refusal) = confirm_unchanged(&inventory, &checked_drain(&mut observation, self.env.interrupted), quarantine, Tree::AtQuarantine, false) {
            return refuse(refusal);
        }
        attempt.delete(observation, &binding, hook)
    }

    /// [`Resolution::ContentRemoved`]: the registration of the empty
    /// quarantine, removed like a missing directory's after its index is
    /// checked against the consent just given.
    fn registration(self, lock: crate::remove::recovery::lock::OperationLock, hook: &mut Hook<'_>) -> Result<Removal, Crashed>
    where
        'a: 'r,
    {
        let refuse = |refusal: Refusal| Ok(Removal::Refused(Box::new(refusal)));
        let Some(approval) = self.request.record else {
            return refuse(Refusal::ConsentRequired);
        };
        if approval.inspected.needs_consent() && !approval.discard_staged {
            return refuse(Refusal::ConsentRequired);
        }
        let entry = match listed_at(self.request.base, self.quarantine()) {
            Ok(entry) => entry,
            Err(refusal) => return refuse(refusal),
        };
        let mut report = self.report();
        report.registration = RegistrationState::AtQuarantine;
        let mut attempt = match self.attempt(lock, self.nothing_to_delete(), report) {
            Ok(attempt) => attempt,
            Err(refusal) => return refuse(refusal),
        };
        if hook(Boundary::Begun) {
            return Err(Crashed);
        }
        // `remove_missing_record` checks the quarantine's path; the original
        // is checked here, since a recreation there is never cleaned up.
        if presence(self.original(), &self.record.checkout_identity) != Presence::Absent {
            return refuse(Refusal::Changed("something now exists at the worktree's original path".into()));
        }
        match remove_missing_record(self.request.base, &entry, &approval.inspected) {
            Ok(warning) => attempt.warnings.extend(warning.map(|warning| format!("could not delete the copy record: {warning}"))),
            Err(MissingRefusal::RecordRemoval(error)) => return Ok(attempt.stop(RemovalOutcome::CleanupFailed, error)),
            // Checked before Git ran: nothing changed.
            Err(refusal) => return refuse(Refusal::Index(refusal)),
        }
        attempt.report.quarantine = Some(QuarantineReport {
            path: self.record.quarantine.clone(),
            identity: self.record.checkout_identity,
            state: QuarantineState::Removed,
            retained_entries: 0,
            retained: Vec::new(),
        });
        attempt.unregistered(hook)
    }

    /// [`Resolution::CheckoutRemoved`]: the branch actions approved again.
    fn branches(self, lock: crate::remove::recovery::lock::OperationLock, hook: &mut Hook<'_>) -> Result<Removal, Crashed>
    where
        'a: 'r,
    {
        let mut report = self.report();
        report.quarantine = Some(QuarantineReport {
            path: self.record.quarantine.clone(),
            identity: self.record.checkout_identity,
            state: QuarantineState::Removed,
            retained_entries: 0,
            retained: Vec::new(),
        });
        let attempt = match self.attempt(lock, self.nothing_to_delete(), report) {
            Ok(attempt) => attempt,
            Err(refusal) => return Ok(Removal::Refused(Box::new(refusal))),
        };
        if hook(Boundary::Begun) {
            return Err(Crashed);
        }
        attempt.unregistered(hook)
    }
}

/// Git's one registration at `path`.
fn listed_at(base: &Path, path: &Path) -> Result<WorktreeEntry, Refusal> {
    let listing = crate::git::worktree_list_from(base, base).map_err(Refusal::Git)?;
    let found: Vec<WorktreeEntry> = parse_worktree_list(&listing).into_iter().filter(|entry| same_location(&entry.path, path)).collect();
    match <[WorktreeEntry; 1]>::try_from(found) {
        Ok([entry]) => Ok(entry),
        Err(found) => Err(Refusal::Changed(format!("Git's worktree list shows the moved checkout {} times", found.len()))),
    }
}

/// A drain for a resumed attempt's checks before deletion. The checkout was
/// moved by an earlier attempt, so an interrupt here is left to the deletion
/// engine, which stops at its first drain with the report any interrupt after
/// the move gets; refusing would say that nothing was moved. The events are
/// still checked.
fn checked_drain(observation: &mut Observation, interrupted: &dyn Fn() -> bool) -> Drain {
    Drain { interrupted: false, ..observation.drain(interrupted) }
}
