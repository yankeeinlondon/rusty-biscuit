//! The removal lifecycle against real repositories: refusals before the
//! move, stops after it, crash injection at each phase boundary (in process,
//! and one real process kill), and retry by the original name.
//!
//! Policy cases use the injected observer and an injected usage source, so
//! they never depend on host timing or on what else the host runs; the
//! `native_*` tests use this platform's real observer.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use sniff::filesystem::query::{
    Evidence, EvidenceKind, FileIdentity, MatchBasis, Mechanism, NativeString, PathUsageError, PathUsageReport,
    ProcessRecord,
};

use super::*;
use crate::git::recorder;
use crate::remove::missing::inspect_missing;
use crate::remove::observation::injected::Injector;
use crate::remove::observation::{EventKind, Root};
use crate::remove::outcome::BranchState;
use crate::remove::recovery::store::{JOURNAL_FILE, RECORD_FILE, REPORT_FILE};
use crate::remove::test_support::TestRepo;
use crate::worktree::parse_worktree_list;

const BRANCH: &str = "feat/gone";

/// A repository with a linked worktree `gone` holding a dirty tracked file,
/// an untracked file, and ignored build output in two directories, plus,
/// with [`fixture_with_neighbor`], a second worktree that must survive.
struct Fixture {
    repo: TestRepo,
    gone: PathBuf,
    keep: Option<PathBuf>,
}

fn fixture() -> Fixture {
    let repo = TestRepo::new();
    // Every worktree reads the common `info/exclude`; a file write, not a
    // commit, which keeps Windows setup inside the time budget.
    let common = crate::remove::admin_entry::common_git_dir(&repo.path()).unwrap();
    fs::create_dir_all(common.join("info")).unwrap();
    fs::write(common.join("info").join("exclude"), "target/\n").unwrap();
    let gone = repo.add_worktree(BRANCH, "gone", "main");
    fs::write(gone.join("README.md"), "dirty\n").unwrap();
    fs::write(gone.join("notes.txt"), "untracked\n").unwrap();
    for directory in ["target/debug", "target/release"] {
        fs::create_dir_all(gone.join(directory)).unwrap();
        for file in ["a.o", "b.o"] {
            fs::write(gone.join(directory).join(file), "object").unwrap();
        }
    }
    Fixture { repo, gone, keep: None }
}

fn fixture_with_neighbor() -> Fixture {
    let mut fixture = fixture();
    fixture.keep = Some(fixture.repo.add_worktree("feat/keep", "keep", "main"));
    fixture
}

impl Fixture {
    fn base(&self) -> PathBuf {
        self.repo.path()
    }

    fn entry_at(&self, path: &Path) -> WorktreeEntry {
        let listing = self.repo.git(&["worktree", "list", "--porcelain"]);
        parse_worktree_list(&listing).into_iter().find(|entry| same_location(&entry.path, path)).expect("listed")
    }

    fn entry(&self) -> WorktreeEntry {
        self.entry_at(&self.gone)
    }

    /// What the user approves from a report of the checkout at `checkout`.
    fn approval_at(&self, checkout: &Path) -> Approval {
        let (inventory, fingerprint) = consent_state(&self.base(), checkout, Some(BRANCH)).unwrap();
        Approval { inventory, fingerprint, tip: self.repo.sha(&format!("refs/heads/{BRANCH}")), discard_files: true }
    }

    fn approval(&self) -> Approval {
        self.approval_at(&self.gone)
    }

    fn delete_branch(&self) -> BranchActions {
        BranchActions {
            local: Some(LocalBranchDeletion { name: BRANCH.into(), expected_tip: self.repo.sha(&format!("refs/heads/{BRANCH}")) }),
            remote: None,
        }
    }

    fn branch_exists(&self) -> bool {
        self.repo.try_git(&["rev-parse", "--verify", &format!("refs/heads/{BRANCH}")]).is_ok()
    }

    fn listed_at(&self, path: &Path) -> usize {
        let listing = self.repo.git(&["worktree", "list", "--porcelain"]);
        parse_worktree_list(&listing).iter().filter(|entry| same_location(&entry.path, path)).count()
    }

    fn store(&self) -> RecoveryStore {
        RecoveryStore::open(&crate::remove::admin_entry::common_git_dir(&self.base()).unwrap()).unwrap()
    }

    /// The one pending operation.
    fn pending(&self) -> OperationId {
        let pending = self.store().pending().unwrap();
        let [only] = pending.as_slice() else { panic!("expected one pending operation, found {pending:?}") };
        only.id.clone()
    }

    fn no_pending(&self) -> bool {
        self.store().pending().unwrap().is_empty()
    }

    fn quarantine_of(&self, id: &OperationId) -> PathBuf {
        self.store().read_record(id).unwrap().quarantine.as_path().to_path_buf()
    }

    /// Whether the checkout is untouched at its original path.
    fn untouched(&self) -> bool {
        self.gone.join("README.md").is_file()
            && self.gone.join("target/debug/a.o").is_file()
            && self.listed_at(&self.gone) == 1
            && self.branch_exists()
    }
}

/// A usage source answering every query with `processes`, counting queries.
struct Usage {
    outcome: &'static str,
    processes: Vec<ProcessRecord>,
    /// The shared budget ran out (partial discovery).
    exhausted: bool,
    /// Queries after this many fail.
    fail_after: Option<usize>,
    queries: AtomicUsize,
}

impl Usage {
    fn clear() -> Self {
        Self { outcome: "usable", exhausted: false, processes: Vec::new(), fail_after: None, queries: AtomicUsize::new(0) }
    }
}

impl UsageSource for Usage {
    fn query(&self, path: &Path) -> Result<PathUsageReport, PathUsageError> {
        let earlier = self.queries.fetch_add(1, Ordering::SeqCst);
        if self.fail_after.is_some_and(|after| earlier >= after) {
            return Err(PathUsageError::RootIdentity { path: path.to_path_buf(), source: io::Error::other("injected") });
        }
        let mut report: PathUsageReport = serde_json::from_value(json!({
            "schema_version": 1,
            "target": {
                "requested": {"display": path},
                "resolved": {"display": path},
                "kind": "directory",
                "identity": {"kind": "unix", "device": "7", "inode": "3"},
                "recursive": true
            },
            "outcome": self.outcome,
            "started_at": "2026-10-12T00:00:00Z",
            "finished_at": "2026-10-12T00:00:01Z",
            "budget_ms": 2000,
            "elapsed_us": 1000,
            "budget_exhausted": self.exhausted,
            "processes": [],
            "coverage": [],
            "limitations": []
        }))
        .unwrap();
        report.processes = self.processes.clone();
        Ok(report)
    }
}

/// Another process with an open file in the checkout.
fn holder(path: &Path) -> ProcessRecord {
    let file = path.join(".gitnexus/index.db");
    ProcessRecord {
        pid: 8124,
        creation_token: Some(77),
        start_time: None,
        identity_uncertain: false,
        name: Some(NativeString::from("node")),
        executable: Some(NativeString::from("/usr/bin/node")),
        user: None,
        evidence: vec![Evidence {
            kind: EvidenceKind::OpenHandle,
            mechanism: Mechanism::OpenHandles,
            matched_paths: vec![NativeString::from(file.as_path())],
            observed_path: Some(NativeString::from(file.as_path())),
            match_basis: MatchBasis::Identity,
            identity: Some(FileIdentity::Unix { device: 7, inode: 9 }),
            descriptor: Some(12),
            watch: None,
            access: None,
            event_only: None,
        }],
    }
}

/// Injected observers; the last one started is reachable through
/// [`Observers::injector`].
#[derive(Default)]
struct Observers {
    last: Mutex<Option<Injector>>,
    started: AtomicUsize,
}

impl Observers {
    fn start(&self, targets: Targets, moved: bool) -> Result<Observation, ObservationError> {
        self.started.fetch_add(1, Ordering::SeqCst);
        let (observation, injector) = if moved {
            Observation::injected_at_quarantine(targets, ObserverOptions::default())
        } else {
            Observation::injected(targets, ObserverOptions::default())
        };
        *self.last.lock().unwrap() = Some(injector);
        Ok(observation)
    }

    fn injector(&self) -> Injector {
        self.last.lock().unwrap().clone().expect("an observer started")
    }
}

/// Runs `body` with an environment over `usage`, injected observers, and
/// `interrupted`.
fn with_env<R>(usage: &Usage, observers: &Observers, interrupted: &dyn Fn() -> bool, body: impl FnOnce(&Environment<'_>) -> R) -> R {
    let observe = |targets, moved| observers.start(targets, moved);
    body(&Environment { usage, interrupted, observe: &observe })
}

fn run(fixture: &Fixture, approval: &Approval, branches: &BranchActions, usage: &Usage, observers: &Observers, hook: &mut Hook<'_>) -> Result<Removal, Crashed> {
    let entry = fixture.entry();
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval, branches };
    with_env(usage, observers, &|| false, |env| run_with(&request, env, hook))
}

fn run_plain(fixture: &Fixture) -> Removal {
    run(fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &Observers::default(), &mut |_| false).unwrap()
}

fn refused(removal: Removal) -> Refusal {
    match removal {
        Removal::Refused(refusal) => *refusal,
        Removal::Finished(finished) => panic!("expected a refusal, got {:?}", finished.report),
    }
}

fn finished(removal: Removal) -> Finished {
    match removal {
        Removal::Finished(finished) => *finished,
        Removal::Refused(refusal) => panic!("expected the operation to run, got {refusal:?}"),
    }
}

fn saved_report(finished: &Finished) -> OperationReport {
    serde_json::from_slice(&fs::read(&finished.report_path).unwrap()).unwrap()
}

/// Asserts a completed removal: both locations and the registration gone,
/// the other worktree untouched, and the record closed.
fn assert_removed(fixture: &Fixture, finished: &Finished) {
    assert_eq!(finished.exit_class(), ExitClass::Success, "{:?}", finished.report);
    assert_eq!(finished.report.outcome, RemovalOutcome::Removed);
    assert_eq!(finished.report.registration, RegistrationState::Removed);
    assert!(!fixture.gone.exists());
    let quarantine = finished.report.quarantine.as_ref().expect("moved");
    assert_eq!(quarantine.state, QuarantineState::Removed);
    assert!(!quarantine.path.as_path().exists());
    assert_eq!(fixture.listed_at(&fixture.gone) + fixture.listed_at(quarantine.path.as_path()), 0);
    if let Some(keep) = &fixture.keep {
        assert_eq!(fixture.listed_at(keep), 1, "the other worktree is untouched");
        assert!(keep.join("README.md").is_file());
    }
    assert!(fixture.no_pending(), "the record is closed");
    let dir = finished.report_path.parent().unwrap();
    assert!(!dir.join(JOURNAL_FILE).exists(), "the journal is deleted after completion");
    assert_eq!(saved_report(finished), finished.report, "the saved report is the one returned");
}

// ---- A first attempt ------------------------------------------------------

/// Acceptance items 2 and 3 at library level, with the real observer: the
/// dirty and ignored content is deleted, only this worktree is unregistered,
/// the branch goes after the checkout, and the record is completed.
#[test]
fn native_a_removal_runs_end_to_end_and_closes_its_record() {
    let fixture = fixture_with_neighbor();
    let copy_record = crate::copy_record::record_path(&fixture.base(), &fixture.gone).unwrap();
    fs::create_dir_all(copy_record.parent().unwrap()).unwrap();
    fs::write(&copy_record, "{}").unwrap();
    let usage = Usage::clear();
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    assert!(!approval.inventory.ignored.is_empty(), "the build output is ignored, so no consent covers it");
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };

    recorder::start_recording();
    let removal = remove_checkout(&request, &Environment { usage: &usage, interrupted: &|| false, observe: &native_observation });
    let calls = recorder::finish_recording();

    let finished = finished(removal);
    assert_removed(&fixture, &finished);
    assert!(finished.warnings.is_empty(), "{:?}", finished.warnings);
    assert!(!fixture.branch_exists(), "the branch is deleted after the checkout");
    assert_eq!(finished.report.branches, [BranchOutcome { target: BranchTarget::Local { name: BRANCH.into() }, state: BranchState::Deleted }]);
    assert!(!copy_record.exists(), "the original's copy record is deleted after verification");
    assert_eq!(usage.queries.load(Ordering::SeqCst), 1, "one decisive query, no diagnostics");
    assert_eq!(finished.report.discovery.len(), 1);
    assert_eq!(finished.report.discovery[0].purpose, DiscoveryPurpose::Preflight);
    assert!(finished.report.deleted.iter().any(|path| path.as_path().ends_with("target/debug/a.o")));
    assert!(finished.report.journal.deleted.files >= 6, "{:?}", finished.report.journal);
    let forbidden = ["--force", "-f", "prune", "repair", "move", "unlock"];
    assert!(!calls.iter().flatten().any(|arg| forbidden.contains(&arg.as_str())), "{calls:?}");
    let completed = fixture.store().read_record(finished.report.operation_id.as_ref().unwrap()).unwrap();
    assert_eq!(completed.completed_phase, Phase::Completed);
}

/// Partial, unavailable, and unsupported discovery take the move-then-delete
/// path and keep their coverage in the report.
fn assert_incomplete_discovery_removes(outcome: &'static str, exhausted: bool) {
    let fixture = fixture();
    let usage = Usage { outcome, exhausted, ..Usage::clear() };
    let finished = finished(run(&fixture, &fixture.approval(), &BranchActions::default(), &usage, &Observers::default(), &mut |_| false).unwrap());
    assert_removed(&fixture, &finished);
    assert!(fixture.branch_exists(), "no branch action was requested");
    match &finished.report.discovery[0].result {
        DiscoveryResult::Report(report) => {
            assert_eq!(serde_json::to_value(report.outcome).unwrap(), json!(outcome));
            assert_eq!(report.budget_exhausted, exhausted);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn partial_discovery_still_removes_and_is_kept_in_the_report() {
    assert_incomplete_discovery_removes("usable", true);
}

#[test]
fn unavailable_discovery_still_removes_and_is_kept_in_the_report() {
    assert_incomplete_discovery_removes("unavailable", false);
}

#[test]
fn unsupported_discovery_still_removes_and_is_kept_in_the_report() {
    assert_incomplete_discovery_removes("unsupported", false);
}

/// Acceptance item 1: usage refuses before anything moves; consent to
/// discard does not override it.
#[test]
fn observed_usage_refuses_before_anything_moves() {
    let fixture = fixture();
    let usage = Usage { processes: vec![holder(&fixture.gone)], ..Usage::clear() };
    let refusal = refused(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &usage, &Observers::default(), &mut |_| false).unwrap());
    match &refusal {
        Refusal::Usage(UsageRefusal::InUse(in_use)) => assert_eq!(in_use.holders[0].pid, 8124),
        other => panic!("{other:?}"),
    }
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(fixture.untouched());
    assert!(fixture.no_pending(), "no record is created before the decisive query clears");
}

/// A query that could not verify the checkout refuses.
#[test]
fn an_unverified_checkout_refuses() {
    let fixture = fixture();
    let usage = Usage { fail_after: Some(0), ..Usage::clear() };
    let refusal = refused(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &usage, &Observers::default(), &mut |_| false).unwrap());
    assert!(matches!(refusal, Refusal::Usage(UsageRefusal::Unverified(_))), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(fixture.untouched());
}

#[test]
fn files_that_need_consent_refuse_without_it() {
    let fixture = fixture();
    let approval = Approval { discard_files: false, ..fixture.approval() };
    let refusal = refused(run(&fixture, &approval, &BranchActions::default(), &Usage::clear(), &Observers::default(), &mut |_| false).unwrap());
    assert!(matches!(refusal, Refusal::ConsentRequired), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(fixture.untouched());
}

/// The consent fingerprint and the tip are rechecked after discovery.
#[test]
fn files_changed_after_approval_refuse() {
    let fixture = fixture();
    let approval = fixture.approval();
    fs::write(fixture.gone.join("README.md"), "changed after the report\n").unwrap();
    let refusal = refused(run(&fixture, &approval, &BranchActions::default(), &Usage::clear(), &Observers::default(), &mut |_| false).unwrap());
    assert!(matches!(&refusal, Refusal::Changed(why) if why.contains("files")), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert_eq!(fs::read_to_string(fixture.gone.join("README.md")).unwrap(), "changed after the report\n");
    assert!(fixture.no_pending());
}

#[test]
fn a_tip_moved_after_approval_refuses() {
    let fixture = fixture();
    let approval = fixture.approval();
    let other = fixture.repo.commit("later.txt");
    fixture.repo.git(&["update-ref", &format!("refs/heads/{BRANCH}"), &other]);
    let refusal = refused(run(&fixture, &approval, &BranchActions::default(), &Usage::clear(), &Observers::default(), &mut |_| false).unwrap());
    assert!(matches!(&refusal, Refusal::Changed(why) if why.contains("HEAD moved")), "{refusal:?}");
    assert!(fixture.gone.join("README.md").is_file());
}

/// Observation that cannot be established is an environment limit (exit
/// 4), before anything moves.
#[test]
fn unavailable_observation_is_an_environment_limit() {
    let fixture = fixture();
    let usage = Usage::clear();
    let observe = |_: Targets, _: bool| Err(ObservationError::Unsupported { reason: "injected" });
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), BranchActions::default());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let refusal = refused(remove_checkout(&request, &Environment { usage: &usage, interrupted: &|| false, observe: &observe }));
    assert!(matches!(refusal, Refusal::Observation(_)), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Blocked);
    assert_eq!(usage.queries.load(Ordering::SeqCst), 0, "no discovery without observation");
    assert!(fixture.untouched());
}

#[test]
fn a_locked_worktree_refuses() {
    let fixture = fixture();
    fixture.repo.git(&["worktree", "lock", "--reason", "in use", &fixture.gone.to_string_lossy()]);
    let refusal = refused(run_plain(&fixture));
    assert!(matches!(&refusal, Refusal::Ineligible(MoveRefusal::Locked { .. })), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(fixture.untouched());
}

/// A change between the record's creation and the move refuses (exit 3),
/// and the record of an operation that changed nothing is discarded.
#[test]
fn a_change_just_before_the_move_refuses_and_discards_the_record() {
    let fixture = fixture();
    let observers = Observers::default();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Begun {
            fs::write(fixture.gone.join("late.txt"), "late").unwrap();
            observers.injector().event(Root::Tree, "late.txt", EventKind::Created);
        }
        false
    };
    let refusal = refused(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &observers, &mut hook).unwrap());
    assert!(matches!(&refusal, Refusal::Inventory(InventoryRefusal::Changed { .. })), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(fixture.untouched() && fixture.gone.join("late.txt").is_file());
    assert!(fixture.no_pending());
    let store_dirs: Vec<_> = fs::read_dir(fixture.store().root()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(store_dirs, ["locks"], "the operation directory is gone");
}

/// An admin event other than the move's own `gitdir` rewrite stops the
/// removal after the move: exit 1, both paths and the registration in their
/// actual state, the branch kept, and the record pending.
#[test]
fn unexplained_activity_during_the_move_stops_after_it() {
    let fixture = fixture();
    let observers = Observers::default();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Moved {
            observers.injector().event(Root::Admin, "index", EventKind::Modified);
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &observers, &mut hook).unwrap());
    assert_eq!(finished.report.outcome, RemovalOutcome::ActivityDetected);
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert_eq!(finished.report.activity[0].location, ActivityLocation::Admin);
    assert_eq!(finished.report.original_state, OriginalState::Moved);
    assert_eq!(finished.report.registration, RegistrationState::AtQuarantine);
    let quarantine = finished.report.quarantine.as_ref().unwrap();
    assert_eq!(quarantine.state, QuarantineState::Present);
    assert!(quarantine.path.as_path().join("README.md").is_file(), "nothing was deleted");
    assert_eq!(finished.report.journal.deleted.total(), 0);
    assert_eq!(finished.report.branches[0].state, BranchState::NotAttempted);
    assert!(fixture.branch_exists());
    assert_eq!(saved_report(&finished), finished.report);
    assert_eq!(fixture.pending(), *finished.report.operation_id.as_ref().unwrap());
}

/// Acceptance item 3 at lifecycle level: a retained-cwd style write into
/// the quarantine during deletion stops it, keeps the new file and the
/// rest, reports both paths and the deletions so far, and runs no branch
/// action.
#[test]
fn a_write_into_the_quarantine_during_deletion_stops_with_an_honest_report() {
    let fixture = fixture();
    let observers = Observers::default();
    let quarantine = std::cell::RefCell::new(None::<PathBuf>);
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Moved {
            *quarantine.borrow_mut() = Some(fixture.quarantine_of(&fixture.pending()));
        }
        if boundary == Boundary::DeletionBatch(2) {
            let quarantine = quarantine.borrow().clone().unwrap();
            fs::write(quarantine.join("written-late.txt"), "new work").unwrap();
            observers.injector().event(Root::Tree, "written-late.txt", EventKind::Created);
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &observers, &mut hook).unwrap());
    let quarantine = quarantine.into_inner().unwrap();
    assert_eq!(finished.report.outcome, RemovalOutcome::ActivityDetected);
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert_eq!(fs::read_to_string(quarantine.join("written-late.txt")).unwrap(), "new work");
    assert!(finished.report.journal.deleted.total() >= 1, "batch 1 was deleted before the write");
    assert_eq!(finished.report.deleted.len() as u64, finished.report.journal.deleted.total());
    let reported = finished.report.quarantine.as_ref().unwrap();
    assert_eq!(reported.state, QuarantineState::Present);
    assert!(reported.retained.iter().any(|path| path.as_path() == Path::new("written-late.txt")));
    assert_eq!(reported.retained_entries as usize, retained_entries(&quarantine, &reported.identity).unwrap().len());
    assert!(finished.report.notes.contains(&crate::remove::outcome::ReportNote::ConcurrentWriteLimit));
    assert_eq!(finished.report.registration, RegistrationState::AtQuarantine);
    assert!(fixture.branch_exists(), "no branch operation");
    assert_eq!(finished.report.branches[0].state, BranchState::NotAttempted);
    // Diagnostics ask who uses the quarantine, a validated real directory.
    assert!(finished.report.discovery.iter().any(|run| run.purpose == DiscoveryPurpose::Diagnostic
        && run.path.as_path() == quarantine.as_path()));
}

/// Acceptance item 3's other half: recreation at the original path stops
/// the removal; the recreated tree is never touched or deleted.
#[test]
fn recreation_at_the_original_path_stops_and_is_left_alone() {
    let fixture = fixture();
    let observers = Observers::default();
    let usage = Usage::clear();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::DeletionBatch(1) {
            fs::create_dir(&fixture.gone).unwrap();
            fs::write(fixture.gone.join("index.db"), "recreated").unwrap();
            observers.injector().event(Root::Parent, "gone", EventKind::Created);
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &usage, &observers, &mut hook).unwrap());
    assert_eq!(finished.report.outcome, RemovalOutcome::ActivityDetected);
    assert_eq!(finished.report.original_state, OriginalState::Recreated);
    assert_eq!(fs::read_to_string(fixture.gone.join("index.db")).unwrap(), "recreated");
    assert!(fixture.branch_exists());
    let diagnostics: Vec<_> = finished.report.discovery.iter().filter(|run| run.purpose == DiscoveryPurpose::Diagnostic).collect();
    assert!(diagnostics.iter().any(|run| same_location(run.path.as_path(), &fixture.gone)), "{diagnostics:?}");
}

/// Lost coverage right after the move stops as unverified state, and a
/// diagnostic query that fails is recorded without changing the stop.
#[test]
fn lost_coverage_after_the_move_stops_and_a_failed_diagnostic_is_recorded() {
    let fixture = fixture();
    let observers = Observers::default();
    let usage = Usage { fail_after: Some(1), ..Usage::clear() };
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Moved {
            observers.injector().overflow();
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &usage, &observers, &mut hook).unwrap());
    assert_eq!(finished.report.outcome, RemovalOutcome::StateUnverified);
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert_eq!(finished.report.activity.last().unwrap().kind, ActivityKind::CoverageLost);
    let diagnostic = finished.report.discovery.iter().find(|run| run.purpose == DiscoveryPurpose::Diagnostic).expect("a diagnostic query");
    assert!(matches!(&diagnostic.result, DiscoveryResult::Failed { kind, .. } if kind == "root_identity"), "{diagnostic:?}");
    assert!(fixture.branch_exists());
}

/// An interrupt stops at the next entry boundary; the operation stays
/// pending and a retry finishes it.
#[test]
fn an_interrupted_removal_is_resumed() {
    let fixture = fixture();
    let observers = Observers::default();
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let usage = Usage::clear();
    let finished = finished(with_env(&usage, &observers, &|| true, |env| remove_checkout(&request, env)));
    assert_eq!(finished.report.outcome, RemovalOutcome::CleanupFailed);
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert!(fixture.branch_exists());

    let id = fixture.pending();
    let quarantine = fixture.quarantine_of(&id);
    let fresh = fixture.approval_at(&quarantine);
    let resumed = finished_resume(&fixture, &id, Some(&fresh), None, &fixture.delete_branch(), &observers);
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
}

#[test]
fn a_failed_branch_deletion_keeps_the_record_and_a_retry_needs_fresh_approval() {
    let fixture = fixture();
    let stale = BranchActions {
        local: Some(LocalBranchDeletion { name: BRANCH.into(), expected_tip: "0".repeat(40) }),
        remote: Some(RemoteBranchDeletion {
            remote: "origin".into(),
            destination: BRANCH.into(),
            endpoint: "https://example.invalid/repo.git".into(),
            expected_sha: "0".repeat(40),
        }),
    };
    let finished = finished(run(&fixture, &fixture.approval(), &stale, &Usage::clear(), &Observers::default(), &mut |_| false).unwrap());
    assert_eq!(finished.report.outcome, RemovalOutcome::Removed, "the checkout is removed");
    assert!(finished.report.branch_failed());
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert!(matches!(finished.report.branches[0].state, BranchState::Failed { .. }));
    assert_eq!(finished.report.branches[1].state, BranchState::NotAttempted, "nothing after a failed step runs");
    assert!(fixture.branch_exists());
    assert!(!fixture.gone.exists());

    // Retry by the original name finds the pending record, though Git no
    // longer registers anything by that name.
    let id = match resolve_target(&fixture.base(), BRANCH).unwrap() {
        Target::Pending(target) => target.id,
        other => panic!("{other:?}"),
    };
    assert_eq!(inspect(&fixture.base(), &id).unwrap().resolution, Resolution::CheckoutRemoved);
    // The saved approval is never reused: an empty set of actions deletes
    // nothing.
    let resumed = finished_resume(&fixture, &id, None, None, &BranchActions::default(), &Observers::default());
    assert_removed(&fixture, &resumed);
    assert!(fixture.branch_exists(), "no branch deletion was approved again");
}

// ---- Crash injection at each phase boundary --------------------------------

fn crash_at(fixture: &Fixture, observers: &Observers, at: Boundary) -> OperationId {
    let crashed = run(fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), observers, &mut |boundary| boundary == at);
    assert!(crashed.is_err(), "the hook crashed the run at {at:?}");
    fixture.pending()
}

fn finished_resume(
    fixture: &Fixture,
    id: &OperationId,
    approval: Option<&Approval>,
    record: Option<&RecordApproval>,
    branches: &BranchActions,
    observers: &Observers,
) -> Finished {
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id, approval, record, branches };
    finished(with_env(&Usage::clear(), observers, &|| false, |env| resume(&request, env)))
}

#[test]
fn a_crash_after_the_record_leaves_nothing_moved_and_blocks_a_new_run() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Begun);
    assert!(fixture.untouched());
    let inspection = inspect(&fixture.base(), &id).unwrap();
    assert_eq!(inspection.resolution, Resolution::NotStarted);
    assert_eq!((inspection.original.clone(), inspection.quarantine.clone()), (Presence::Checkout, Presence::Absent));

    let refusal = refused(run_plain(&fixture));
    assert!(matches!(&refusal, Refusal::Store(StoreError::Pending(pending)) if *pending == id), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(fixture.untouched());
    // The name resolves to the pending operation: the live checkout is the
    // same one.
    assert!(matches!(resolve_target(&fixture.base(), "gone").unwrap(), Target::Pending(target) if target.id == id));
}

#[test]
fn a_released_unstarted_record_lets_a_new_removal_run() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Begun);
    release_unstarted(&fixture.base(), &id).unwrap();
    assert!(fixture.no_pending() && fixture.untouched());
    assert!(matches!(resolve_target(&fixture.base(), "gone").unwrap(), Target::Live(_)));
    assert_removed(&fixture, &finished(run_plain(&fixture)));
}

/// The process stopped between the rename and Git's record update; the
/// retry moves the checkout back rather than deleting anything.
#[test]
fn a_crash_between_the_rename_and_the_association_is_moved_back() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Begun);
    let quarantine = fixture.quarantine_of(&id);
    rename::rename_no_replace(&fixture.gone, &quarantine).unwrap();
    assert_eq!(inspect(&fixture.base(), &id).unwrap().resolution, Resolution::MovedUnassociated);
    let refusal = refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| {
        resume(&ResumeRequest { base: &fixture.base(), id: &id, approval: None, record: None, branches: &BranchActions::default() }, env)
    }));
    assert!(matches!(&refusal, Refusal::Recovery(inspection) if inspection.resolution == Resolution::MovedUnassociated));

    release_unstarted(&fixture.base(), &id).unwrap();
    assert!(fixture.untouched() && !quarantine.exists());
    assert!(fixture.no_pending());
}

/// Acceptance items 6 and 10: after the move Git no longer registers the
/// original name; the retry finds the record by it, takes a fresh
/// inventory and fresh consent, and finishes.
#[test]
fn a_crash_after_the_move_is_resumed_by_the_original_name() {
    let fixture = fixture();
    let observers = Observers::default();
    let id = crash_at(&fixture, &observers, Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    assert_eq!(fixture.listed_at(&fixture.gone), 0);
    assert_eq!(fixture.listed_at(&quarantine), 1);
    let inspection = inspect(&fixture.base(), &id).unwrap();
    assert_eq!(inspection.resolution, Resolution::Quarantined);
    assert_eq!(inspection.journal.unwrap().deleted.total(), 0);
    assert!(inspection.retained.unwrap().iter().any(|path| path.as_path() == Path::new("README.md")));

    for name in ["gone", BRANCH, &quarantine.file_name().unwrap().to_string_lossy()] {
        assert!(matches!(resolve_target(&fixture.base(), name).unwrap(), Target::Pending(target) if target.id == id), "{name}");
    }
    // The original approval is bound to the original path; the retry must
    // be approved again from the quarantine.
    let fresh = fixture.approval_at(&quarantine);
    let resumed = finished_resume(&fixture, &id, Some(&fresh), None, &fixture.delete_branch(), &observers);
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
    assert!(observers.started.load(Ordering::SeqCst) >= 2, "the retry observes again");
}

/// A retry never deletes content its fresh consent does not cover.
#[test]
fn new_content_in_the_quarantine_needs_fresh_consent() {
    let fixture = fixture();
    let observers = Observers::default();
    let id = crash_at(&fixture, &observers, Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    let before = fixture.approval_at(&quarantine);
    fs::write(quarantine.join("new-work.txt"), "written after the crash").unwrap();

    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&before), record: None, branches: &BranchActions::default() };
    let refusal = refused(with_env(&Usage::clear(), &observers, &|| false, |env| resume(&request, env)));
    assert!(matches!(&refusal, Refusal::Changed(_)), "{refusal:?}");
    assert_eq!(fs::read_to_string(quarantine.join("new-work.txt")).unwrap(), "written after the crash");
    assert_eq!(fixture.pending(), id);

    let without_consent = Approval { discard_files: false, ..fixture.approval_at(&quarantine) };
    let request = ResumeRequest { approval: Some(&without_consent), ..request };
    assert!(matches!(refused(with_env(&Usage::clear(), &observers, &|| false, |env| resume(&request, env))), Refusal::ConsentRequired));
    assert!(quarantine.join("new-work.txt").is_file());
}

/// Usage of the quarantine refuses the retry, as it would a first attempt.
#[test]
fn usage_of_the_quarantine_refuses_the_retry() {
    let fixture = fixture();
    let observers = Observers::default();
    let id = crash_at(&fixture, &observers, Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    let fresh = fixture.approval_at(&quarantine);
    let usage = Usage { processes: vec![holder(&quarantine)], ..Usage::clear() };
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&fresh), record: None, branches: &BranchActions::default() };
    let refusal = refused(with_env(&usage, &observers, &|| false, |env| resume(&request, env)));
    assert!(matches!(refusal, Refusal::Usage(UsageRefusal::InUse(_))), "{refusal:?}");
    assert!(quarantine.join("README.md").is_file());
}

#[test]
fn a_replaced_quarantine_is_never_resumed() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    let aside = fixture.base().parent().unwrap().join("aside");
    fs::rename(&quarantine, &aside).unwrap();
    fs::create_dir(&quarantine).unwrap();
    fs::write(quarantine.join("stranger.txt"), "not the checkout").unwrap();

    let inspection = inspect(&fixture.base(), &id).unwrap();
    assert!(matches!(&inspection.resolution, Resolution::Unresumable(_)), "{:?}", inspection.resolution);
    assert_eq!(inspection.journal, None, "nothing in a stranger's directory is checked");
    let fresh = fixture.approval_at(&aside);
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&fresh), record: None, branches: &BranchActions::default() };
    let refusal = refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env)));
    assert!(matches!(refusal, Refusal::Recovery(_)));
    assert!(quarantine.join("stranger.txt").is_file() && aside.join("README.md").is_file());
}

/// A partial checkout whose `.git` link is gone is reported, never
/// repaired or cleaned up.
#[test]
fn a_partial_checkout_without_its_git_link_is_kept() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    fs::remove_file(quarantine.join(".git")).unwrap();
    let inspection = inspect(&fixture.base(), &id).unwrap();
    assert_eq!(inspection.resolution, Resolution::PartialCheckout);
    assert!(inspection.retained.unwrap().iter().any(|path| path.as_path() == Path::new("README.md")));
    let refusal = refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| {
        resume(&ResumeRequest { base: &fixture.base(), id: &id, approval: None, record: None, branches: &BranchActions::default() }, env)
    }));
    assert!(matches!(refusal, Refusal::Recovery(_)));
    assert!(quarantine.join("README.md").is_file());
    assert!(!quarantine.join(".git").exists(), "nothing was repaired");
}

/// A crash after the content is gone resumes as the missing-directory
/// record removal, re-checking the index; an unlink whose result was never
/// persisted is reported as uncertain, not as deleted.
#[test]
fn a_crash_after_the_content_is_removed_resumes_with_the_record_and_uncertain_entries() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::ContentRemoved);
    let quarantine = fixture.quarantine_of(&id);
    assert!(!quarantine.exists() && fixture.listed_at(&quarantine) == 1);
    // The crash window between the last unlink and its result: drop the
    // final results line.
    let journal = fixture.store().operation_dir(&id).join(JOURNAL_FILE);
    let text = fs::read_to_string(&journal).unwrap();
    let kept: Vec<&str> = text.lines().collect();
    fs::write(&journal, format!("{}\n", kept[..kept.len() - 1].join("\n"))).unwrap();

    let inspection = inspect(&fixture.base(), &id).unwrap();
    assert_eq!(inspection.resolution, Resolution::ContentRemoved);
    let summary = inspection.journal.unwrap();
    assert!(summary.uncertain.total() >= 1 && summary.deleted.total() >= 1, "{summary:?}");

    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: None, record: None, branches: &BranchActions::default() };
    assert!(matches!(refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env))), Refusal::ConsentRequired));

    let inspected = inspect_missing(&base, &fixture.entry_at(&quarantine)).unwrap();
    let record = RecordApproval { inspected, discard_staged: true };
    let resumed = finished_resume(&fixture, &id, None, Some(&record), &fixture.delete_branch(), &Observers::default());
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
}

#[test]
fn a_crash_after_unregistering_runs_only_the_branch_actions_on_retry() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Unregistered);
    assert_eq!(inspect(&fixture.base(), &id).unwrap().resolution, Resolution::CheckoutRemoved);
    let resumed = finished_resume(&fixture, &id, None, None, &fixture.delete_branch(), &Observers::default());
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
}

/// Completion is the record's last write; a crash before it leaves a record
/// whose retry finds nothing left to delete, and a completed record never
/// authorizes another removal.
#[test]
fn a_completed_record_never_authorizes_a_second_removal() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::BranchesDone);
    assert!(!fixture.branch_exists());
    assert_eq!(inspect(&fixture.base(), &id).unwrap().resolution, Resolution::CheckoutRemoved);
    let resumed = finished_resume(&fixture, &id, None, None, &BranchActions::default(), &Observers::default());
    assert_removed(&fixture, &resumed);

    assert!(matches!(inspect(&fixture.base(), &id), Err(StoreError::Completed(_))));
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: None, record: None, branches: &BranchActions::default() };
    let refusal = refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env)));
    assert!(matches!(refusal, Refusal::Store(StoreError::Completed(_))), "{refusal:?}");
    assert!(matches!(resolve_target(&base, BRANCH), Err(ResolveError::NotFound(_))));
}

/// A stale lock file (its holder crashed) never blocks; a held lock does.
#[test]
fn only_a_held_lock_blocks_a_retry() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Unregistered);
    let store = fixture.store();
    let record = store.read_record(&id).unwrap();
    let held = store.lock(&record.repository).unwrap();
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: None, record: None, branches: &BranchActions::default() };
    let refusal = refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env)));
    assert!(matches!(refusal, Refusal::Store(StoreError::Lock(LockError::Busy))), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Blocked);
    drop(held);
    assert_removed(&fixture, &finished_resume(&fixture, &id, None, None, &BranchActions::default(), &Observers::default()));
}

#[test]
fn an_unreadable_record_blocks_a_new_removal() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Begun);
    fs::write(fixture.store().operation_dir(&id).join(RECORD_FILE), "{ not json").unwrap();
    let refusal = refused(run_plain(&fixture));
    assert!(matches!(refusal, Refusal::Store(StoreError::Record { .. }) | Refusal::Store(StoreError::Storage { .. })), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(fixture.untouched());
}

/// A record or journal that cannot be written after the move stops further
/// destructive work and reports the last durable phase.
#[cfg(unix)]
#[test]
fn an_unwritable_record_stops_before_deleting() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = fixture();
    let operation_dir = std::cell::RefCell::new(None::<PathBuf>);
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Moved {
            let dir = fixture.store().operation_dir(&fixture.pending());
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();
            *operation_dir.borrow_mut() = Some(dir);
        }
        false
    };
    let removal = run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &Observers::default(), &mut hook).unwrap();
    let dir = operation_dir.into_inner().unwrap();
    if fs::write(dir.join("probe"), "x").is_ok() {
        // Running as root: permissions cannot make the write fail.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        return;
    }
    let finished = finished(removal);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(finished.report.outcome, RemovalOutcome::CleanupFailed);
    assert_eq!(finished.report.journal.deleted.total(), 0, "nothing was deleted");
    assert!(finished.warnings.iter().any(|warning| warning.contains("report could not be saved")), "{:?}", finished.warnings);
    let id = finished.report.operation_id.clone().unwrap();
    assert_eq!(fixture.store().read_record(&id).unwrap().completed_phase, Phase::Associated, "the last durable phase");
    assert!(fixture.quarantine_of(&id).join("README.md").is_file());
}

// ---- Retry resolution -----------------------------------------------------

/// A new worktree at the original path is ambiguous with the pending
/// removal, and the pending one refuses while the original is recreated.
#[test]
fn a_new_worktree_at_the_original_path_is_ambiguous_and_never_cleaned_up() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    fixture.repo.git(&["worktree", "add", "-q", "-b", "feat/new", &fixture.gone.to_string_lossy(), "main"]);

    match resolve_target(&fixture.base(), "gone").unwrap() {
        Target::Ambiguous(candidates) => {
            assert_eq!(candidates.len(), 2);
            assert!(candidates.iter().any(|candidate| matches!(candidate, Candidate::Live(entry) if entry.branch.as_deref() == Some("feat/new"))));
            assert!(candidates.iter().any(|candidate| matches!(candidate, Candidate::Pending { id: pending, .. } if *pending == id)));
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(resolve_target(&fixture.base(), "feat/new").unwrap(), Target::Live(_)));

    assert_eq!(inspect(&fixture.base(), &id).unwrap().resolution, Resolution::OriginalRecreated);
    let fresh = fixture.approval_at(&quarantine);
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&fresh), record: None, branches: &BranchActions::default() };
    assert!(matches!(refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env))), Refusal::Recovery(_)));
    assert!(fixture.gone.join("README.md").is_file() && quarantine.join("README.md").is_file());
}

/// The branch reused by a new worktree after the checkout was removed is
/// ambiguous with the pending removal (its branch actions are unfinished).
#[test]
fn a_reused_branch_is_ambiguous_with_the_pending_removal() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Unregistered);
    let again = fixture.base().parent().unwrap().join("again");
    fixture.repo.git(&["worktree", "add", "-q", &again.to_string_lossy(), BRANCH]);
    match resolve_target(&fixture.base(), BRANCH).unwrap() {
        Target::Ambiguous(candidates) => {
            assert!(candidates.iter().any(|candidate| matches!(candidate, Candidate::Live(entry) if same_location(&entry.path, &again))));
            assert!(candidates.iter().any(|candidate| matches!(candidate, Candidate::Pending { id: pending, .. } if *pending == id)));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn names_that_match_nothing_are_not_found_and_base_is_never_pending() {
    let fixture = fixture();
    assert!(matches!(resolve_target(&fixture.base(), "nothing-here"), Err(ResolveError::NotFound(_))));
    assert!(matches!(resolve_target(&fixture.base(), "base").unwrap(), Target::Live(entry) if entry.is_main));
    assert!(matches!(resolve_target(&fixture.base(), "gone").unwrap(), Target::Live(entry) if same_location(&entry.path, &fixture.gone)));
    assert!(!fixture.store().root().join("locks").exists() || fixture.no_pending());
}

// ---- Real process termination ----------------------------------------------

const CHILD_BASE: &str = "WT_LIFECYCLE_CHILD_BASE";
const CHILD_TARGET: &str = "WT_LIFECYCLE_CHILD_TARGET";
const CHILD_READY: &str = "wt-lifecycle-child: blocked";

/// Run by [`native_a_killed_removal_is_found_and_resumed_by_its_original_name`]
/// in its own process: a removal with the real observer that blocks before
/// deletion batch 2 until the parent kills it.
#[test]
#[ignore = "subprocess fixture invoked by the process termination test"]
fn lifecycle_child_blocks_mid_deletion() {
    use std::io::Write;
    let (Ok(base), Ok(target)) = (std::env::var(CHILD_BASE), std::env::var(CHILD_TARGET)) else { return };
    let (base, target) = (PathBuf::from(base), PathBuf::from(target));
    let listing = crate::git::git_from(&base, &base, &["worktree", "list", "--porcelain"]).unwrap();
    let entry = parse_worktree_list(&listing).into_iter().find(|entry| same_location(&entry.path, &target)).unwrap();
    let (inventory, fingerprint) = consent_state(&base, &target, Some(BRANCH)).unwrap();
    let tip = crate::git::git_from(&base, &base, &["rev-parse", &format!("refs/heads/{BRANCH}")]).unwrap();
    let approval = Approval { inventory, fingerprint, tip: tip.clone(), discard_files: true };
    let branches = BranchActions { local: Some(LocalBranchDeletion { name: BRANCH.into(), expected_tip: tip }), remote: None };
    let request = RemovalRequest { base: &base, entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let usage = Usage::clear();
    let env = Environment { usage: &usage, interrupted: &|| false, observe: &native_observation };
    let _ = run_with(&request, &env, &mut |boundary| {
        if boundary == Boundary::DeletionBatch(2) {
            println!("{CHILD_READY}");
            std::io::stdout().flush().unwrap();
            std::thread::sleep(Duration::from_secs(120));
        }
        false
    });
    panic!("the parent should have killed this process");
}

/// Acceptance items 6 and 11 with a real process kill mid-deletion: the
/// lock dies with the process, the record is found by the original name,
/// the journal keeps what was deleted, and the retry finishes.
#[test]
fn native_a_killed_removal_is_found_and_resumed_by_its_original_name() {
    let fixture = fixture();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["remove::lifecycle::tests::lifecycle_child_blocks_mid_deletion", "--exact", "--ignored", "--nocapture"])
        .env(CHILD_BASE, fixture.base())
        .env(CHILD_TARGET, &fixture.gone)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if line.contains(CHILD_READY) {
                let _ = sender.send(());
            }
        }
    });
    let ready = receiver.recv_timeout(Duration::from_secs(60));
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
    ready.expect("the child reached deletion batch 2");

    let id = match resolve_target(&fixture.base(), "gone").unwrap() {
        Target::Pending(target) => target.id,
        other => panic!("{other:?}"),
    };
    let inspection = inspect(&fixture.base(), &id).unwrap();
    assert_eq!(inspection.resolution, Resolution::Quarantined);
    let summary = inspection.journal.unwrap();
    assert!(summary.deleted.total() >= 1 && summary.uncertain.total() == 0, "{summary:?}");
    assert!(fixture.branch_exists());

    let quarantine = inspection.record.quarantine.as_path().to_path_buf();
    let fresh = fixture.approval_at(&quarantine);
    let request = ResumeRequest { base: &fixture.base(), id: &id, approval: Some(&fresh), record: None, branches: &fixture.delete_branch() };
    let usage = Usage::clear();
    let resumed = finished(resume(&request, &Environment { usage: &usage, interrupted: &|| false, observe: &native_observation }));
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
    let archived = fixture.store().operation_dir(&id).join("journal-1.jsonl");
    assert!(!archived.exists(), "completion deletes the earlier journal too");
    assert!(fixture.store().operation_dir(&id).join(REPORT_FILE).is_file());
}
