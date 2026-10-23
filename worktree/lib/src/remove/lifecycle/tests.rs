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
use crate::remove::observation::injected::{Injector, Scripted};
use crate::remove::observation::{CoverageLost, EventKind, ObservationStage, Root};
use crate::remove::outcome::{BranchState, EntryCounts, MAX_REPORTED_DELETED};
use crate::remove::recovery::journal::EntryState;
use crate::remove::recovery::store::fault::{self as store_fault, Fault};
use crate::remove::recovery::store::{ENTRIES_FILE, JOURNAL_FILE, RECORD_FILE, REPORT_FILE, ReopenStep};
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

    /// What the user approves from a retry's report of pending operation
    /// `id`.
    fn resume_approval(&self, id: &OperationId) -> Approval {
        let ResumeConsent { inventory, fingerprint, .. } = resume_consent(&self.base(), id).unwrap();
        Approval { inventory, fingerprint, tip: self.repo.sha(&format!("refs/heads/{BRANCH}")), discard_files: true }
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
        Self {
            outcome: "usable",
            exhausted: false,
            processes: Vec::new(),
            fail_after: None,
            queries: AtomicUsize::new(0),
        }
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
/// [`Observers::injector`]. With `confirming`, each (a fresh tree only) waits
/// for delivery on a virtual clock, delivering that script.
#[derive(Default)]
struct Observers {
    last: Mutex<Option<Injector>>,
    started: AtomicUsize,
    confirming: Option<Vec<(Duration, Scripted)>>,
}

impl Observers {
    fn confirming(script: Vec<(Duration, Scripted)>) -> Self {
        Self { confirming: Some(script), ..Self::default() }
    }

    fn start(&self, targets: Targets, moved: bool) -> Result<Observation, ObservationError> {
        self.started.fetch_add(1, Ordering::SeqCst);
        let (observation, injector) = match (&self.confirming, moved) {
            (Some(script), false) => {
                let (observation, injector) = Observation::injected_confirming(targets, ObserverOptions::default());
                for (at, delivery) in script {
                    injector.at(*at, delivery.clone());
                }
                (observation, injector)
            }
            (_, true) => Observation::injected_at_quarantine(targets, ObserverOptions::default()),
            (None, false) => Observation::injected(targets, ObserverOptions::default()),
        };
        *self.last.lock().unwrap() = Some(injector);
        Ok(observation)
    }

    fn injector(&self) -> Injector {
        self.last.lock().unwrap().clone().expect("an observer started")
    }

    /// Whether the last observer started delivered its scripted interrupt.
    fn interrupted(&self) -> bool {
        self.last.lock().unwrap().as_ref().is_some_and(Injector::interrupted)
    }
}

/// Runs `body` with an environment over `usage`, injected observers, and
/// `interrupted`.
fn with_env<R>(usage: &Usage, observers: &Observers, interrupted: &dyn Fn() -> bool, body: impl FnOnce(&Environment<'_>) -> R) -> R {
    let observe = |targets, moved, _interrupted: &dyn Fn() -> bool| observers.start(targets, moved);
    body(&Environment { usage, interrupted, observe: &observe, slow_startup: &|_| {} })
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

/// The saved report with its entries read back from the entries file it
/// names. The saved JSON lists none inline, and names the file exactly when
/// there are entries.
fn saved_report(finished: &Finished) -> OperationReport {
    let bytes = fs::read(&finished.report_path).unwrap();
    let raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(raw.get("journal_entries").is_none(), "no inline entries: {raw}");
    let mut report: OperationReport = serde_json::from_slice(&bytes).unwrap();
    let entries = finished.report_path.with_file_name(ENTRIES_FILE);
    match report.entries_file.as_deref() {
        Some(name) => {
            assert_eq!(name, ENTRIES_FILE);
            report.journal_entries = crate::remove::recovery::store::read_entries(&entries);
            assert!(!report.journal_entries.is_empty(), "an entries file is named only when there are entries");
        }
        None => assert!(!entries.exists(), "no entries file beside a report that names none"),
    }
    report
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
    assert!(!dir.join("journal-1.jsonl").exists(), "earlier attempts' journals are deleted after completion");
    assert_eq!(saved_report(finished), finished.report, "the saved report is the one returned");
}

/// The fixture checkout's entries: the `.git` link, `README.md`,
/// `notes.txt`, and four object files under three directories.
const FIXTURE_ENTRIES: EntryCounts = EntryCounts { files: 7, directories: 3, links: 0 };

/// Asserts that `report` accounts for every one of `expected` entries
/// exactly once, as deleted or uncertain, across every attempt: the counts,
/// the full entry list, and the capped list of deleted paths agree.
fn assert_accounted_once(report: &OperationReport, expected: EntryCounts) {
    let journal = &report.journal;
    let accounted = EntryCounts {
        files: journal.deleted.files + journal.uncertain.files,
        directories: journal.deleted.directories + journal.uncertain.directories,
        links: journal.deleted.links + journal.uncertain.links,
    };
    assert_eq!(accounted, expected, "{journal:?}");
    assert_eq!(report.journal_entries.len() as u64, expected.total(), "{:?}", report.journal_entries);
    let paths: std::collections::BTreeSet<&Path> = report.journal_entries.iter().map(|entry| entry.path.as_path()).collect();
    assert_eq!(paths.len(), report.journal_entries.len(), "an entry is listed twice");
    let deleted = report.journal_entries.iter().filter(|entry| entry.state == EntryState::Deleted).count();
    assert_eq!(deleted as u64, journal.deleted.total());
    assert_eq!(report.deleted.len().min(MAX_REPORTED_DELETED) as u64 + report.truncation.deleted_omitted, journal.deleted.total());
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
    let removal = remove_checkout(&request, &Environment { usage: &usage, interrupted: &|| false, observe: &native_observation, slow_startup: &|_| {} });
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

/// The Git processes a clean removal starts, so a redundant read cannot come
/// back unnoticed: each costs 20–50 ms on Windows. Every listing and index
/// read here is a deliberate re-read (docs/cli/remove.md, "Deleting the
/// checkout"): the plan's checks, the recheck after discovery, the index
/// binding, the checks again just before the rename, the association
/// verified after it, and the final unregister's listing and index check.
/// Common directory, refs, branch tips, and, in a plain repository, the
/// worktree listings and the index reads without `--debug` are read
/// in-process.
#[test]
fn a_clean_removal_starts_only_its_deliberate_git_processes() {
    let fixture = fixture();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());

    let scope = crate::git::calls::CallScope::enter();
    recorder::start_recording();
    crate::questions::start_recording();
    let removal = run(&fixture, &approval, &branches, &Usage::clear(), &Observers::default(), &mut |_| false).unwrap();
    let asked = crate::questions::finish_recording();
    let calls = recorder::finish_recording();
    let started = scope.finish();

    assert_removed(&fixture, &finished(removal));
    // Each call's subcommand and options, without `-c` settings or paths.
    let commands: Vec<String> = calls
        .iter()
        .map(|args| {
            let mut kept = Vec::new();
            let mut args = args.iter();
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "-c" => {
                        args.next();
                    }
                    arg if Path::new(arg).is_absolute() => {}
                    arg => kept.push(arg),
                }
            }
            kept.join(" ")
        })
        .collect();
    assert_eq!(
        commands,
        [
            "status --porcelain=v1 -z --untracked-files=all --ignored=matching", // recheck: the consent fingerprint
            "ls-files --stage --debug -z", // bind_index: Git's reading of every entry
            "worktree list --porcelain",  // unregister: its content is gone, so Git marks it prunable
            "ls-files --stage --debug -z", //   the index still as approved
            "worktree remove",
            "branch -D feat/gone",
        ],
        "{calls:?}"
    );
    assert_eq!(started, commands.len() as u64, "every recorded call started one process");
    // Every listing and plain index read is asked at its own step and read
    // in-process (`git::worktree_list_from`, `git::staged_entries_*`) unless
    // only Git can answer it: the last listing, above.
    let asked: Vec<String> = asked.iter().map(|args| args.join(" ")).collect();
    assert_eq!(
        asked,
        [
            "worktree list --porcelain", // plan_move: listed once, not prunable
            "ls-files --stage -z",       //   no submodules in the index
            "ls-files --stage -z",       // recheck: the fingerprint's index part
            "worktree list --porcelain", // just before the rename: the plan's checks again
            "ls-files --stage -z",       //   no submodules
            "worktree list --porcelain", // after it: the association verified
            "worktree list --porcelain", // unregister: listed once at the quarantine (by Git)
        ],
        "{asked:?}"
    );
}

/// More deletions than the report names: the named list is capped, and the
/// saved entries file still lists every entry after the journal is deleted.
#[test]
fn a_completed_report_lists_every_entry_past_the_deleted_cap() {
    let fixture = fixture();
    let extra = MAX_REPORTED_DELETED + 5;
    for n in 0..extra {
        fs::write(fixture.gone.join("target/debug").join(format!("extra-{n}.o")), "object").unwrap();
    }
    let finished = finished(run_plain(&fixture));
    assert_removed(&fixture, &finished);
    let expected = EntryCounts { files: FIXTURE_ENTRIES.files + extra as u64, ..FIXTURE_ENTRIES };
    assert_eq!(finished.report.journal.deleted, expected);
    assert_eq!(finished.report.deleted.len(), MAX_REPORTED_DELETED);
    assert_eq!(finished.report.truncation.deleted_omitted, expected.total() - MAX_REPORTED_DELETED as u64);
    assert_accounted_once(&saved_report(&finished), expected);
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

/// Another process using the checkout never stops a removal the user
/// agreed to: it is named in a warning, and the report keeps the query.
#[test]
fn observed_usage_is_reported_and_the_removal_completes() {
    let fixture = fixture();
    let usage = Usage { processes: vec![holder(&fixture.gone)], ..Usage::clear() };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &usage, &Observers::default(), &mut |_| false).unwrap());
    assert_removed(&fixture, &finished);
    assert!(
        finished.warnings.iter().any(|warning| warning.contains("node (PID 8124) was using the worktree")),
        "{:?}",
        finished.warnings
    );
    assert_eq!(usage.queries.load(Ordering::SeqCst), 1, "queried once, never rechecked");
    let DiscoveryResult::Report(report) = &finished.report.discovery[0].result else { panic!("{:?}", finished.report.discovery) };
    assert_eq!(report.processes[0].pid, 8124);
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

/// The consent fingerprint and the tip are rechecked after discovery. The
/// user agreed to the removal, so a difference is a warning and the removal
/// completes.
#[test]
fn files_changed_after_approval_are_deleted_and_reported() {
    let fixture = fixture();
    let approval = fixture.approval();
    fs::write(fixture.gone.join("README.md"), "changed after the report\n").unwrap();
    let finished = finished(run(&fixture, &approval, &BranchActions::default(), &Usage::clear(), &Observers::default(), &mut |_| false).unwrap());
    assert_removed(&fixture, &finished);
    assert!(finished.warnings.iter().any(|warning| warning.contains("files")), "{:?}", finished.warnings);
}

/// A tip that moved is reported; the approved branch deletion still
/// compares the approved tip, so the moved branch is kept.
#[test]
fn a_tip_moved_after_approval_is_reported_and_the_moved_branch_kept() {
    let fixture = fixture();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let other = fixture.repo.commit("later.txt");
    fixture.repo.git(&["update-ref", &format!("refs/heads/{BRANCH}"), &other]);
    let finished = finished(run(&fixture, &approval, &branches, &Usage::clear(), &Observers::default(), &mut |_| false).unwrap());
    assert_eq!(finished.report.outcome, RemovalOutcome::Removed);
    assert!(!fixture.gone.exists());
    assert!(finished.warnings.iter().any(|warning| warning.contains("HEAD moved")), "{:?}", finished.warnings);
    assert!(finished.report.branch_failed(), "the branch moved, so its deletion fails safely");
    assert!(fixture.branch_exists());
}

/// Observation that cannot be established is an environment limit (exit
/// 4), before anything moves.
#[test]
fn unavailable_observation_is_an_environment_limit() {
    let fixture = fixture();
    let usage = Usage::clear();
    let observe = |_: Targets, _: bool, _: &dyn Fn() -> bool| Err(ObservationError::Unsupported { reason: "injected" });
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), BranchActions::default());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let refusal = refused(remove_checkout(&request, &Environment { usage: &usage, interrupted: &|| false, observe: &observe, slow_startup: &|_| {} }));
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

/// A change between the record's creation and the move is reported, and
/// the new file is deleted with the rest.
#[test]
fn a_change_just_before_the_move_is_deleted_and_reported() {
    let fixture = fixture();
    let observers = Observers::default();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Begun {
            fs::write(fixture.gone.join("late.txt"), "late").unwrap();
            observers.injector().event(Root::Tree, "late.txt", EventKind::Created);
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &observers, &mut hook).unwrap());
    assert_removed(&fixture, &finished);
    assert!(
        finished.report.activity.iter().any(|observed| observed.path.as_path().ends_with("late.txt")),
        "{:?}",
        finished.report.activity
    );
    assert!(!fixture.branch_exists());
}

/// An interrupt that arrives before the move (during the usage query, say)
/// refuses with nothing moved and the record discarded, so a Ctrl-C there
/// never leaves a pending removal behind.
#[test]
fn an_interrupt_before_the_move_refuses_and_discards_the_record() {
    let fixture = fixture();
    let observers = Observers::default();
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };

    let refusal = refused(with_env(&Usage::clear(), &observers, &|| true, |env| remove_checkout(&request, env)));

    assert!(matches!(refusal, Refusal::Interrupted), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Failure);
    assert!(fixture.untouched());
    assert!(fixture.no_pending());
}

/// An interrupt that ends observation's startup wait is an interrupt (exit
/// 1), not an environment limit, and nothing is queried or moved.
#[test]
fn an_interrupt_while_observation_starts_refuses_as_an_interrupt() {
    let fixture = fixture();
    let usage = Usage::clear();
    let observe = |_: Targets, _: bool, _: &dyn Fn() -> bool| Err(ObservationError::Interrupted);
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), BranchActions::default());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };

    let refusal = refused(remove_checkout(&request, &Environment { usage: &usage, interrupted: &|| true, observe: &observe, slow_startup: &|_| {} }));

    assert!(matches!(refusal, Refusal::Interrupted), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Failure);
    assert_eq!(usage.queries.load(Ordering::SeqCst), 0);
    assert!(fixture.untouched());
    assert!(fixture.no_pending());
}

/// The first wait for delivery polls the interrupt: one that arrives while
/// it waits refuses at once, before any record exists, instead of waiting
/// out the stall limit (which would refuse as lost coverage, exit 4).
#[test]
fn an_interrupt_during_a_wait_before_the_move_refuses_at_once() {
    let fixture = fixture();
    let observers = Observers::confirming(vec![(Duration::from_secs(2), Scripted::Interrupt)]);
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };

    let refusal = refused(with_env(&Usage::clear(), &observers, &|| observers.interrupted(), |env| remove_checkout(&request, env)));

    assert!(matches!(refusal, Refusal::Interrupted), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Failure);
    assert!(observers.injector().elapsed() < Duration::from_secs(3), "{:?}", observers.injector().elapsed());
    assert!(fixture.untouched());
    assert!(fixture.no_pending());
}

/// After the move, an interrupt that ends the wait for the move's own events
/// stops the removal as any interrupt there does: exit 1, nothing deleted,
/// and a pending record a retry continues.
#[test]
fn an_interrupt_during_a_wait_after_the_move_stops_with_a_pending_record() {
    let fixture = fixture();
    // The two checks before the move and the move's labeling are confirmed;
    // the drain of the move's own events is not.
    let observers = Observers::confirming(vec![
        (Duration::from_secs(1), Scripted::Acknowledge),
        (Duration::from_secs(2), Scripted::Acknowledge),
        (Duration::from_secs(3), Scripted::Acknowledge),
        (Duration::from_secs(4), Scripted::Interrupt),
    ]);
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };

    let stopped = finished(with_env(&Usage::clear(), &observers, &|| observers.interrupted(), |env| remove_checkout(&request, env)));

    assert_eq!(stopped.report.outcome, RemovalOutcome::CleanupFailed);
    assert_eq!(stopped.exit_class(), ExitClass::Failure);
    assert_eq!(stopped.report.journal.deleted.total(), 0, "{:?}", stopped.report.journal);
    assert!(observers.injector().elapsed() < Duration::from_secs(5), "{:?}", observers.injector().elapsed());
    let id = fixture.pending();
    assert!(fixture.quarantine_of(&id).join("README.md").is_file(), "the checkout waits at the quarantine");
    assert!(fixture.branch_exists());
}

/// An admin event other than the move's own `gitdir` rewrite is reported,
/// and the removal completes.
#[test]
fn unexplained_activity_during_the_move_is_reported() {
    let fixture = fixture();
    let observers = Observers::default();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Moved {
            observers.injector().event(Root::Admin, "index", EventKind::Modified);
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &observers, &mut hook).unwrap());
    assert_removed(&fixture, &finished);
    assert_eq!(finished.report.activity[0].location, ActivityLocation::Admin);
    assert!(!fixture.branch_exists());
}

/// A retained-cwd style write into the quarantine during deletion is
/// deleted with the rest and reported; the removal and its branch step
/// complete.
#[test]
fn a_write_into_the_quarantine_during_deletion_is_deleted_and_reported() {
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
    assert_removed(&fixture, &finished);
    assert!(!quarantine.exists());
    assert!(
        finished.report.activity.iter().any(|observed| observed.path.as_path() == quarantine.join("written-late.txt")),
        "{:?}",
        finished.report.activity
    );
    assert!(finished.report.notes.contains(&crate::remove::outcome::ReportNote::ConcurrentWriteLimit));
    assert!(!fixture.branch_exists(), "the approved branch deletion ran");
}

/// Recreation at the original path is outside what the user agreed to: it
/// is never touched, and reported, while the moved checkout and its record
/// are removed.
#[test]
fn recreation_at_the_original_path_is_reported_and_left_alone() {
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
    assert_eq!(finished.exit_class(), ExitClass::Success, "{:?}", finished.report);
    assert_eq!(finished.report.outcome, RemovalOutcome::Removed);
    assert_eq!(finished.report.registration, RegistrationState::Removed);
    assert_eq!(fs::read_to_string(fixture.gone.join("index.db")).unwrap(), "recreated");
    assert!(finished.report.activity.iter().any(|observed| observed.location == ActivityLocation::Original), "{:?}", finished.report.activity);
    assert!(!finished.report.quarantine.as_ref().unwrap().path.as_path().exists());
    assert!(fixture.no_pending());
}

/// Lost coverage right after the move is reported once, with its stage,
/// and the removal completes without watching.
#[test]
fn lost_coverage_after_the_move_is_reported_and_the_removal_completes() {
    let fixture = fixture();
    let observers = Observers::default();
    let usage = Usage::clear();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Moved {
            observers.injector().overflow();
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &usage, &observers, &mut hook).unwrap());
    assert_removed(&fixture, &finished);
    let lost: Vec<_> = finished.report.activity.iter().filter(|observed| observed.kind == ActivityKind::CoverageLost).collect();
    assert_eq!(lost.len(), 1, "{:?}", finished.report.activity);
    let loss = finished.report.observation.as_ref().and_then(|observation| observation.loss.clone()).expect("recorded");
    assert_eq!((loss.stage, loss.reason, loss.entries_since_drain), (ObservationStage::PostMove, CoverageLost::Overflow, None));
    assert!(!fixture.branch_exists());
}

/// Lost coverage during deletion is saved with the attempt's observation
/// diagnostics: the deletion stage and the entries processed since the
/// previous drain. The removal completes.
#[test]
fn a_loss_during_deletion_is_reported_with_its_stage() {
    let fixture = fixture();
    let observers = Observers::default();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::DeletionBatch(2) {
            observers.injector().overflow();
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &observers, &mut hook).unwrap());
    assert_removed(&fixture, &finished);
    let observation = finished.report.observation.clone().expect("the attempt observed");
    let loss = observation.loss.as_ref().expect("the overflow");
    assert_eq!((loss.stage, &loss.reason), (ObservationStage::Deletion, &CoverageLost::Overflow));
    // Received by the drain that ends batch 2's level, after at least that
    // batch.
    assert!(loss.entries_since_drain.is_some_and(|entries| entries >= 1), "{loss:?}");
    let saved = saved_report(&finished);
    assert_eq!(saved.report_version, crate::remove::outcome::REPORT_VERSION);
    assert_eq!(saved.observation, Some(observation), "persisted as returned");
}

/// An interrupt stops at the next entry boundary; the operation stays
/// pending and a retry finishes it.
///
/// The retry's report keeps the first attempt: its deletions alongside the
/// retry's, each counted once, and its stop (outcome, error, timeline,
/// diagnostics) as an earlier attempt.
#[test]
fn an_interrupted_removal_is_resumed() {
    let fixture = fixture();
    let observers = Observers::default();
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let usage = Usage::clear();
    // Interrupted once two batches are deleted.
    let interrupted = std::sync::atomic::AtomicBool::new(false);
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::DeletionBatch(3) {
            interrupted.store(true, Ordering::SeqCst);
        }
        false
    };
    let stop = || interrupted.load(Ordering::SeqCst);
    let finished = finished(with_env(&usage, &observers, &stop, |env| run_with(&request, env, &mut hook)).unwrap());
    assert_eq!(finished.report.outcome, RemovalOutcome::CleanupFailed);
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert!(fixture.branch_exists());
    assert_eq!(saved_report(&finished), finished.report, "the stop saves every entry so far");
    let first = finished.report;
    let deleted_first = first.journal.deleted.total();
    assert!(deleted_first >= 2 && deleted_first < FIXTURE_ENTRIES.total(), "{:?}", first.journal);

    let id = fixture.pending();
    let fresh = fixture.resume_approval(&id);
    let resumed = finished_resume(&fixture, &id, Some(&fresh), None, &fixture.delete_branch(), &observers);
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());

    let report = &resumed.report;
    assert_accounted_once(report, FIXTURE_ENTRIES);
    assert_eq!(report.journal.deleted, FIXTURE_ENTRIES, "nothing is uncertain");
    let from_first: Vec<_> = report.journal_entries.iter().filter(|entry| entry.attempt == 1).collect();
    let deleted_by_first: Vec<_> = first.journal_entries.iter().filter(|entry| entry.state == EntryState::Deleted).collect();
    assert_eq!(from_first, deleted_by_first, "the first attempt's deletions are kept as they were");
    assert!(report.journal_entries.iter().all(|entry| entry.attempt <= 2));
    let [earlier] = report.earlier_attempts.as_slice() else { panic!("{:?}", report.earlier_attempts) };
    assert_eq!(earlier.outcome, RemovalOutcome::CleanupFailed);
    assert_eq!(earlier.errors, first.errors);
    assert_eq!(earlier.timeline, first.timeline);
    assert_eq!(earlier.discovery, first.discovery);
    assert_eq!(earlier.branches, first.branches);
    assert!(report.errors.is_empty(), "the earlier stop is not this attempt's: {:?}", report.errors);
}

/// Stops a first removal once `batches` deletion batches are done, with
/// `at_batch` run at each batch boundary.
fn interrupted_after(fixture: &Fixture, batches: u64, mut at_batch: impl FnMut(u64)) -> Finished {
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let interrupted = std::sync::atomic::AtomicBool::new(false);
    let mut hook = |boundary: Boundary| {
        if let Boundary::DeletionBatch(batch) = boundary {
            at_batch(batch);
            if batch == batches + 1 {
                interrupted.store(true, Ordering::SeqCst);
            }
        }
        false
    };
    let stop = || interrupted.load(Ordering::SeqCst);
    let finished = finished(with_env(&Usage::clear(), &Observers::default(), &stop, |env| run_with(&request, env, &mut hook)).unwrap());
    assert_eq!(finished.report.outcome, RemovalOutcome::CleanupFailed);
    finished
}

/// A pending operation whose report a version 5 `wt` saved, with its
/// entries inline and no entries file, resumes: that report is read and
/// carried as the earlier attempt, and the completed entries file lists
/// every attempt's entries as the journals recorded them, never the inline
/// list.
#[test]
fn a_removal_stopped_by_a_version_5_wt_resumes_and_completes() {
    let fixture = fixture();
    let first = interrupted_after(&fixture, 2, |_| {});
    let id = fixture.pending();
    let report_path = first.report_path.clone();
    let quarantine = fixture.quarantine_of(&id);
    let deleted_by_first: Vec<_> = first.report.journal_entries.iter().filter(|entry| entry.state == EntryState::Deleted).cloned().collect();
    assert!(!deleted_by_first.is_empty());
    let original = serde_json::to_value(NativeString::from(fixture.gone.as_path())).unwrap();
    // Frozen version 5 bytes: only the operation's own id and paths are
    // filled in. Its one inline entry is one no journal planned.
    let legacy = json!({
        "report_version": 5,
        "operation_id": id.as_str(),
        "outcome": "activity_detected",
        "original": original,
        "original_state": "moved",
        "quarantine": null,
        "registration": "at_quarantine",
        "timeline": [{"at_ms": 5, "step": "moved", "result": "succeeded", "detail": null}],
        "discovery": [],
        "activity": [],
        "journal": {
            "deleted": {"files": 1, "directories": 0, "links": 0},
            "externally_absent": {"files": 0, "directories": 0, "links": 0},
            "kept": {"files": 0, "directories": 0, "links": 0},
            "planned": {"files": 0, "directories": 0, "links": 0},
            "uncertain": {"files": 0, "directories": 0, "links": 0}
        },
        "journal_entries": [{"attempt": 1, "batch": 1, "path": {"display": "only-in-version-5"}, "kind": "file", "state": "deleted"}],
        "deleted": [],
        "notes": [],
        "branches": [],
        "errors": ["stopped by version 5"],
        "truncation": {"activity_omitted": 0, "retained_omitted": 0, "deleted_omitted": 0},
        "earlier_attempts": [],
        "observation": null
    });
    // Overwriting keeps the saved file's owner-only mode.
    fs::write(&report_path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();
    fs::remove_file(report_path.with_file_name(ENTRIES_FILE)).unwrap();
    let read = fixture.store().read_report(&id).unwrap().expect("read");
    assert_eq!((read.report_version, read.journal_entries.len()), (5, 1));

    let fresh = fixture.resume_approval(&id);
    let resumed = finished_resume(&fixture, &id, Some(&fresh), None, &fixture.delete_branch(), &Observers::default());
    assert_removed(&fixture, &resumed);
    assert!(!quarantine.exists());
    let saved = saved_report(&resumed);
    assert_eq!(saved.report_version, crate::remove::outcome::REPORT_VERSION);
    assert_accounted_once(&saved, FIXTURE_ENTRIES);
    let from_first: Vec<_> = saved.journal_entries.iter().filter(|entry| entry.attempt == 1).cloned().collect();
    assert_eq!(from_first, deleted_by_first, "the first attempt's entries come from its journal");
    assert!(saved.journal_entries.iter().all(|entry| entry.path.as_path() != Path::new("only-in-version-5")));
    let [earlier] = saved.earlier_attempts.as_slice() else { panic!("{:?}", saved.earlier_attempts) };
    assert_eq!((earlier.outcome, earlier.errors.as_slice()), (RemovalOutcome::ActivityDetected, ["stopped by version 5".to_string()].as_slice()));
    assert_eq!((earlier.timeline[0].at_ms, earlier.observation.as_ref()), (5, None));
}

/// A stopped retry whose earlier report cannot be read warns, naming the
/// file, and still resumes from the journals.
#[test]
fn an_unreadable_earlier_report_is_named_and_the_retry_still_completes() {
    let fixture = fixture();
    let first = interrupted_after(&fixture, 2, |_| {});
    let id = fixture.pending();
    fs::write(&first.report_path, b"{\"report_version\":6,").unwrap();
    let fresh = fixture.resume_approval(&id);
    let resumed = finished_resume(&fixture, &id, Some(&fresh), None, &fixture.delete_branch(), &Observers::default());
    assert_removed(&fixture, &resumed);
    assert_accounted_once(&saved_report(&resumed), FIXTURE_ENTRIES);
    let path = first.report_path.display().to_string();
    assert!(
        resumed.warnings.iter().any(|warning| warning.starts_with("the earlier attempt's report could not be read") && warning.contains(&path)),
        "{:?}",
        resumed.warnings
    );
    assert!(resumed.report.earlier_attempts.is_empty());
}

/// A journal that cannot be read back when the stop is reported: the report
/// says so, lists no entries, and names no entries file, and the journal is
/// kept.
#[test]
fn a_stop_whose_journal_cannot_be_summarized_saves_no_entries_file() {
    use std::io::Write as _;
    let fixture = fixture();
    let store = fixture.store();
    let stopped = interrupted_after(&fixture, 1, |batch| {
        if batch == 2 {
            // A complete line no journal version has: replay refuses it.
            let id = fixture.pending();
            let journal = store.operation_dir(&id).join(JOURNAL_FILE);
            fs::OpenOptions::new().append(true).open(journal).unwrap().write_all(b"{\"surprise\":true}\n").unwrap();
        }
    });
    let report = &stopped.report;
    assert!(report.errors.iter().any(|error| error.starts_with("the deletion journal could not be summarized")), "{:?}", report.errors);
    assert_eq!((report.entries_file.as_deref(), report.journal_entries.len()), (None, 0));
    assert_eq!(saved_report(&stopped), *report);
    assert!(stopped.report_path.with_file_name(JOURNAL_FILE).exists());
    assert!(!stopped.report_path.with_file_name(ENTRIES_FILE).exists());
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
    let fresh = fixture.resume_approval(&id);
    let resumed = finished_resume(&fixture, &id, Some(&fresh), None, &fixture.delete_branch(), &observers);
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
    assert!(observers.started.load(Ordering::SeqCst) >= 2, "the retry observes again");
}

/// New content after a retry's approval is deleted and reported; content
/// that needs consent still needs it.
#[test]
fn new_content_in_the_quarantine_is_deleted_after_a_retry_is_approved() {
    let fixture = fixture();
    let observers = Observers::default();
    let id = crash_at(&fixture, &observers, Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    let before = fixture.resume_approval(&id);
    fs::write(quarantine.join("new-work.txt"), "written after the crash").unwrap();

    let base = fixture.base();
    let without_consent = Approval { discard_files: false, ..fixture.resume_approval(&id) };
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&without_consent), record: None, branches: &BranchActions::default() };
    assert!(matches!(refused(with_env(&Usage::clear(), &observers, &|| false, |env| resume(&request, env))), Refusal::ConsentRequired));
    assert!(quarantine.join("new-work.txt").is_file(), "no answer, nothing deleted");

    let request = ResumeRequest { approval: Some(&before), ..request };
    let resumed = finished(with_env(&Usage::clear(), &observers, &|| false, |env| resume(&request, env)));
    assert_removed(&fixture, &resumed);
    assert!(resumed.warnings.iter().any(|warning| warning.contains("files")), "{:?}", resumed.warnings);
}

/// Usage of the quarantine is reported on a retry, as on a first attempt,
/// and the retry completes.
#[test]
fn usage_of_the_quarantine_is_reported_on_the_retry() {
    let fixture = fixture();
    let observers = Observers::default();
    let id = crash_at(&fixture, &observers, Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    let fresh = fixture.resume_approval(&id);
    let usage = Usage { processes: vec![holder(&quarantine)], ..Usage::clear() };
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&fresh), record: None, branches: &BranchActions::default() };
    let resumed = finished(with_env(&usage, &observers, &|| false, |env| resume(&request, env)));
    assert_removed(&fixture, &resumed);
    assert!(resumed.warnings.iter().any(|warning| warning.contains("PID 8124")), "{:?}", resumed.warnings);
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
    // The first attempt's deletions stay confirmed, and its unconfirmed
    // unlink stays uncertain.
    let report = &resumed.report;
    assert_eq!(report.journal.deleted, summary.deleted);
    assert_eq!(report.journal.uncertain, summary.uncertain);
    assert_accounted_once(report, FIXTURE_ENTRIES);
    assert!(report.journal_entries.iter().all(|entry| entry.attempt == 1), "the retry deleted nothing");
    assert!(report.earlier_attempts.is_empty(), "the crashed attempt saved no report");
}

#[test]
fn a_crash_after_unregistering_runs_only_the_branch_actions_on_retry() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Unregistered);
    assert_eq!(inspect(&fixture.base(), &id).unwrap().resolution, Resolution::CheckoutRemoved);
    let resumed = finished_resume(&fixture, &id, None, None, &fixture.delete_branch(), &Observers::default());
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
    // The checkout's removal by the crashed attempt is still reported.
    assert_eq!(resumed.report.journal.deleted, FIXTURE_ENTRIES);
    assert_accounted_once(&resumed.report, FIXTURE_ENTRIES);
    assert_eq!(resumed.report.deleted.len() as u64, FIXTURE_ENTRIES.total());
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

// ---- A retry whose journal replacement stops part-way -----------------------

/// Runs `retry` once with `RecoveryStore::reopen` stopped at each of its
/// steps, as a crash or a failed write would stop it. Each stopped retry is
/// refused before it changes anything, the pending operation inspects
/// exactly as before, and `unchanged` holds.
fn each_stopped_reopen(fixture: &Fixture, id: &OperationId, retry: impl Fn() -> Refusal, unchanged: impl Fn()) {
    let header_only = |path: &Path| {
        let journal = fs::read(path.with_file_name(JOURNAL_FILE)).unwrap();
        fs::write(path, &journal[..=journal.iter().position(|byte| *byte == b'\n').unwrap()]).unwrap();
    };
    let summary = |inspection: Inspection| (inspection.resolution, inspection.journal, inspection.torn_tail, inspection.retained);
    let before = summary(inspect(&fixture.base(), id).unwrap());
    for (at, fault, header) in [
        (ReopenStep::Archived, Fault::Crash, false),
        (ReopenStep::Created, Fault::Crash, true),
        (ReopenStep::Initialized, Fault::Crash, false),
        (ReopenStep::Created, Fault::Fail, true),
    ] {
        let injected = store_fault::inject(move |step, path| {
            (step == at).then(|| {
                if header {
                    header_only(path);
                }
                fault
            })
        });
        let refusal = retry();
        drop(injected);
        assert!(matches!(refusal, Refusal::Store(StoreError::Io(_))), "{at:?} {fault:?}: {refusal:?}");
        assert_eq!(summary(inspect(&fixture.base(), id).unwrap()), before, "{at:?} {fault:?}");
        assert_eq!(fixture.pending(), *id);
        unchanged();
    }
}

#[test]
fn a_quarantine_retry_stopped_while_replacing_the_journal_stays_recoverable() {
    let fixture = fixture();
    let observers = Observers::default();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let entry = fixture.entry();
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    // Interrupted after some entries are deleted, so the journal holds
    // confirmed deletions: the polls are the one before the move, the first
    // batch's, and its first entry's.
    let checks = AtomicUsize::new(0);
    let interrupted = || checks.fetch_add(1, Ordering::SeqCst) >= 3;
    let stopped = finished(with_env(&Usage::clear(), &observers, &interrupted, |env| remove_checkout(&request, env)));
    assert_eq!(stopped.report.outcome, RemovalOutcome::CleanupFailed);
    let id = fixture.pending();
    let quarantine = fixture.quarantine_of(&id);
    assert!(inspect(&fixture.base(), &id).unwrap().journal.unwrap().deleted.total() >= 1);

    let fresh = fixture.resume_approval(&id);
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&fresh), record: None, branches: &branches };
    each_stopped_reopen(
        &fixture,
        &id,
        || refused(with_env(&Usage::clear(), &observers, &|| false, |env| resume(&request, env))),
        || assert!(quarantine.join("README.md").is_file(), "a stopped retry deletes nothing"),
    );
    assert_removed(&fixture, &finished_resume(&fixture, &id, Some(&fresh), None, &branches, &observers));
}

#[test]
fn a_registration_retry_stopped_while_replacing_the_journal_stays_recoverable() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::ContentRemoved);
    let quarantine = fixture.quarantine_of(&id);
    let base = fixture.base();
    let inspected = inspect_missing(&base, &fixture.entry_at(&quarantine)).unwrap();
    let record = RecordApproval { inspected, discard_staged: true };
    let request = ResumeRequest { base: &base, id: &id, approval: None, record: Some(&record), branches: &BranchActions::default() };
    each_stopped_reopen(
        &fixture,
        &id,
        || refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env))),
        || assert_eq!(fixture.listed_at(&quarantine), 1, "a stopped retry keeps the registration"),
    );
    assert_removed(&fixture, &finished_resume(&fixture, &id, None, Some(&record), &BranchActions::default(), &Observers::default()));
}

#[test]
fn a_branch_retry_stopped_while_replacing_the_journal_stays_recoverable() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Unregistered);
    let base = fixture.base();
    let branches = fixture.delete_branch();
    let request = ResumeRequest { base: &base, id: &id, approval: None, record: None, branches: &branches };
    each_stopped_reopen(
        &fixture,
        &id,
        || refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env))),
        || assert!(fixture.branch_exists(), "a stopped retry deletes no branch"),
    );
    assert_removed(&fixture, &finished_resume(&fixture, &id, None, None, &branches, &Observers::default()));
    assert!(!fixture.branch_exists());
}

#[test]
fn a_release_stopped_while_replacing_the_journal_stays_recoverable() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Begun);
    each_stopped_reopen(
        &fixture,
        &id,
        || release_unstarted(&fixture.base(), &id).unwrap_err(),
        || assert!(fixture.untouched()),
    );
    release_unstarted(&fixture.base(), &id).unwrap();
    assert!(fixture.no_pending() && fixture.untouched());
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
    let env = Environment { usage: &usage, interrupted: &|| false, observe: &native_observation, slow_startup: &|_| {} };
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

    let fresh = fixture.resume_approval(&id);
    let request = ResumeRequest { base: &fixture.base(), id: &id, approval: Some(&fresh), record: None, branches: &fixture.delete_branch() };
    let usage = Usage::clear();
    let resumed = finished(resume(&request, &Environment { usage: &usage, interrupted: &|| false, observe: &native_observation, slow_startup: &|_| {} }));
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
    let archived = fixture.store().operation_dir(&id).join("journal-1.jsonl");
    assert!(!archived.exists(), "completion deletes the earlier journal too");
    assert!(fixture.store().operation_dir(&id).join(REPORT_FILE).is_file());
}

mod resume_consent;
mod startup_warning;

// ---- Other commands leave a checkout under removal alone -------------------

/// `git status` refreshes a checkout's index, which a removal's watcher sees
/// as activity and stops for. A listing runs it in every other worktree and
/// in none a pending removal identifies: not in the quarantine Git now
/// registers, and not in an unmoved checkout whose record is durable.
#[test]
#[serial_test::serial]
fn a_listing_runs_no_git_status_in_a_checkout_under_removal() {
    for at in [Boundary::Moved, Boundary::Begun] {
        let fixture = fixture_with_neighbor();
        let id = crash_at(&fixture, &Observers::default(), at);
        let under_removal = match at {
            Boundary::Moved => fixture.quarantine_of(&id),
            _ => fixture.gone.clone(),
        };
        let stores = [crate::cache::cache_path(&fixture.base()).unwrap(), crate::fork_origin::fork_origin_path(&fixture.base()).unwrap()];

        recorder::start_recording();
        let mut list = crate::worktree::parse_worktree_state_in(&fixture.base()).unwrap();
        crate::worktree::fill_worktree_statuses(&mut list).unwrap();
        let calls = recorder::finish_recording();
        for store in &stores {
            let _ = fs::remove_file(store);
        }

        let statuses = calls.iter().filter(|args| args.iter().any(|arg| arg == "status")).count();
        assert_eq!(statuses, 2, "{at:?}: the base and the neighbor only: {calls:?}");
        let status = list.statuses.iter().find(|status| same_location(&status.entry.path, &under_removal)).expect("listed");
        assert_eq!(status.removal.as_deref(), Some(BRANCH), "{at:?}");
        assert_eq!(status.dirty, crate::worktree::DirtyStatus::Unknown, "{at:?}");
        let keep = fixture.keep.as_deref().unwrap();
        assert!(list.statuses.iter().filter(|status| !same_location(&status.entry.path, &under_removal)).all(|status| status.removal.is_none()));
        assert!(list.statuses.iter().any(|status| same_location(&status.entry.path, keep)));
    }
}

/// Lost coverage before the move, after the user agreed, is reported once
/// and the removal goes ahead without watching.
#[test]
fn lost_coverage_before_the_move_is_reported_and_the_removal_completes() {
    let fixture = fixture();
    let observers = Observers::default();
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::Begun {
            observers.injector().overflow();
        }
        false
    };
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &Usage::clear(), &observers, &mut hook).unwrap());
    assert_removed(&fixture, &finished);
    let lost = finished.report.activity.iter().filter(|observed| observed.kind == ActivityKind::CoverageLost).count();
    assert_eq!(lost, 1, "{:?}", finished.report.activity);
    assert!(!fixture.branch_exists());
}
