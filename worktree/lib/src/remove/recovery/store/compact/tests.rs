use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::json;
use sniff::filesystem::query::{FileIdentity, PathUsageReport};
use tempfile::TempDir;

use super::fault::{self, Fault};
use super::*;
use crate::remove::recovery::compact::{BranchAction, BranchStep, DeletionMode, StepApproval, UnboundTarget};
use crate::remove::recovery::record::{BranchBinding, IntendedAction, Phase, QUARANTINE_PREFIX, RecoveryRecord};
use crate::remove::recovery::store::{JOURNAL_FILE, LockError};
use crate::remove::test_support::ManualClock;

struct Fixture {
    _dir: TempDir,
    common: PathBuf,
    work: PathBuf,
    store: RecoveryStore,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let common = dir.path().join("repo.git");
    fs::create_dir_all(common.join("worktrees")).unwrap();
    let work = dir.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let store = RecoveryStore::open(&common).unwrap();
    Fixture { _dir: dir, common, work, store }
}

fn identity(inode: u64) -> FileIdentity {
    FileIdentity::Unix { device: 7, inode }
}

fn binding(f: &Fixture, admin: &str) -> RepositoryBinding {
    RepositoryBinding {
        common_dir: f.common.as_path().into(),
        common_dir_identity: identity(1),
        admin_dir: f.common.join("worktrees").join(admin).as_path().into(),
        admin_dir_identity: identity(100 + admin.len() as u64),
    }
}

fn approval(f: &Fixture, admin: &str) -> CompactRecord {
    CompactRecord {
        record_version: COMPACT_RECORD_VERSION,
        operation_id: OperationId::generate().unwrap(),
        created_at_ms: 1,
        requested_name: "fix/path".into(),
        aliases: vec![admin.into()],
        original: f.work.join(admin).as_path().into(),
        deletion_path: f.work.join(format!("{QUARANTINE_PREFIX}{admin}-a91c")).as_path().into(),
        deletion_mode: DeletionMode::Moved,
        checkout_identity: identity(3),
        repository: binding(f, admin),
        unlock_needed: false,
        branch_steps: vec![BranchStep {
            action: BranchAction::Local { name: "fix/path".into(), expected_tip: "a".repeat(40) },
            approval: StepApproval::Conditional,
            state: StepState::Pending,
        }],
        progress: Progress::Approved,
    }
}

fn approve(f: &Fixture, record: CompactRecord) -> PendingRemoval {
    let lock = f.store.lock(&record.repository).unwrap();
    f.store.approve(lock, record).unwrap()
}

fn resume(f: &Fixture, id: &OperationId, admin: &str) -> Result<PendingRemoval, StoreError> {
    let expected = binding(f, admin);
    let lock = f.store.lock(&expected).unwrap();
    f.store.resume(lock, id, &expected)
}

fn names(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect()
}

fn store_names(f: &Fixture) -> BTreeSet<String> {
    names(f.store.root()).into_iter().filter(|name| name != LOCKS_DIR).collect()
}

fn discovery(path: &Path) -> PathUsageReport {
    serde_json::from_value(json!({
        "schema_version": 1,
        "target": {
            "requested": {"display": path},
            "resolved": {"display": path},
            "kind": "directory",
            "identity": {"kind": "unix", "device": "7", "inode": "3"},
            "recursive": true
        },
        "outcome": "unsupported",
        "started_at": "2026-10-12T00:00:00Z",
        "finished_at": "2026-10-12T00:00:00Z",
        "budget_ms": 2000,
        "elapsed_us": 5,
        "budget_exhausted": false,
        "processes": [],
        "coverage": [],
        "limitations": []
    }))
    .unwrap()
}

fn legacy_record(f: &Fixture, admin: &str, phase: Phase) -> RecoveryRecord {
    let original = f.work.join(admin);
    RecoveryRecord {
        record_version: RECORD_VERSION,
        operation_id: OperationId::generate().unwrap(),
        created_at_ms: 1,
        requested_name: "fix/path".into(),
        aliases: vec![],
        original: original.as_path().into(),
        quarantine: f.work.join(format!("{QUARANTINE_PREFIX}{admin}-a91c")).as_path().into(),
        checkout_identity: identity(3),
        repository: binding(f, admin),
        branch: BranchBinding::Named("fix/path".into()),
        approved_tip: "a".repeat(40),
        consent_fingerprint: "b".repeat(64),
        discovery: discovery(&original),
        intended_actions: vec![
            IntendedAction::RemoveCheckout,
            IntendedAction::DeleteLocalBranch { name: "fix/path".into() },
            IntendedAction::DeleteRemoteBranch { remote: "origin".into(), name: "fix/path".into() },
        ],
        completed_phase: phase,
    }
}

/// Writes a legacy operation as an earlier `wt` left it: its record, a
/// report of version 6, a journal and an archived one, all owner-only.
fn save_legacy(f: &Fixture, record: &RecoveryRecord) -> PathBuf {
    let dir = f.store.operation_dir(&record.operation_id);
    storage::create_private_dir(&dir).unwrap();
    storage::write_durable(&dir.join(RECORD_FILE), &record.to_json()).unwrap();
    let report = json!({"report_version": 6, "operation_id": record.operation_id, "outcome": "activity_detected"});
    storage::write_durable(&dir.join(REPORT_FILE), &serde_json::to_vec(&report).unwrap()).unwrap();
    storage::write_durable(&dir.join(JOURNAL_FILE), b"{\"torn").unwrap();
    storage::write_durable(&dir.join("journal-1.jsonl"), b"not a journal").unwrap();
    dir
}

const LEGACY_FILES: [&str; 4] = ["journal-1.jsonl", "journal.jsonl", "record.json", "report.json"];

fn set_of(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn an_approval_is_saved_alone_owner_only_and_reads_back() {
    let f = fixture();
    let record = approval(&f, "wt1");
    let pending = approve(&f, record.clone());
    let dir = f.store.operation_dir(pending.id());
    assert_eq!(names(&dir), set_of(&[RECORD_FILE]));
    assert_eq!(f.store.read_stored(pending.id()).unwrap(), Stored::Compact(Box::new(record.clone())));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(fs::metadata(dir.join(RECORD_FILE)).unwrap().permissions().mode() & 0o777, 0o600);
    }
    // A few kilobytes, whatever the checkout's size (R25).
    assert!(fs::metadata(dir.join(RECORD_FILE)).unwrap().len() < 4096);
}

#[test]
fn an_approval_that_cannot_be_saved_leaves_no_operation_and_says_nothing_was_deleted() {
    for fault_kind in [Fault::Fail, Fault::Crash] {
        let f = fixture();
        let _injected = fault::inject(move |step| (step == SaveStep::Staged).then_some(fault_kind));
        let lock = f.store.lock(&binding(&f, "wt1")).unwrap();
        let error = f.store.approve(lock, approval(&f, "wt1")).unwrap_err();
        assert!(matches!(error, StoreError::NotSaved(_)), "{fault_kind:?}: {error:?}");
        assert!(error.to_string().contains("nothing was deleted"), "{error}");
        assert!(f.store.operations().unwrap().is_empty(), "{fault_kind:?}");
        // A crash may leave its hidden staging directory, which names no
        // operation and blocks nothing.
        assert!(store_names(&f).iter().all(|name| name.starts_with('.')), "{fault_kind:?}: {:?}", store_names(&f));
        drop(_injected);
        approve(&f, approval(&f, "wt1"));
    }
}

#[cfg(unix)]
#[test]
fn an_unwritable_store_is_an_approval_not_saved() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    let lock = f.store.lock(&binding(&f, "wt1")).unwrap();
    fs::set_permissions(f.store.root(), fs::Permissions::from_mode(0o500)).unwrap();
    let result = f.store.approve(lock, approval(&f, "wt1"));
    fs::set_permissions(f.store.root(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(result, Err(StoreError::NotSaved(_))), "{result:?}");
    assert!(f.store.operations().unwrap().is_empty());
}

#[test]
fn a_stored_operation_of_either_format_blocks_a_new_approval_of_the_same_worktree_only() {
    let f = fixture();
    let first = approve(&f, approval(&f, "wt1"));
    let id = first.id().clone();
    drop(first);
    let lock = f.store.lock(&binding(&f, "wt1")).unwrap();
    assert!(matches!(f.store.approve(lock, approval(&f, "wt1")), Err(StoreError::Pending(pending)) if pending == id));
    approve(&f, approval(&f, "wt2"));

    let legacy = legacy_record(&f, "wt3", Phase::Moved);
    save_legacy(&f, &legacy);
    let lock = f.store.lock(&binding(&f, "wt3")).unwrap();
    assert!(matches!(f.store.approve(lock, approval(&f, "wt3")), Err(StoreError::Pending(pending)) if pending == legacy.operation_id));
}

#[test]
fn an_unreadable_record_blocks_every_new_approval() {
    let f = fixture();
    let other = approve(&f, approval(&f, "wt1"));
    let path = f.store.operation_dir(other.id()).join(RECORD_FILE);
    drop(other);
    fs::write(&path, b"{").unwrap();
    let lock = f.store.lock(&binding(&f, "wt2")).unwrap();
    assert!(matches!(f.store.approve(lock, approval(&f, "wt2")), Err(StoreError::Record { .. })));
}

#[test]
fn approve_needs_the_worktrees_own_lock_and_an_unstarted_record() {
    let f = fixture();
    let lock = f.store.lock(&binding(&f, "wt2")).unwrap();
    assert!(matches!(f.store.approve(lock, approval(&f, "wt1")), Err(StoreError::WrongLock)));
    let mut started = approval(&f, "wt1");
    started.progress = Progress::Placed;
    let lock = f.store.lock(&binding(&f, "wt1")).unwrap();
    assert!(matches!(f.store.approve(lock, started), Err(StoreError::PhaseOrder)));
    assert!(f.store.operations().unwrap().is_empty());
}

#[test]
fn progress_and_branch_steps_persist_and_only_move_forward() {
    let f = fixture();
    let mut pending = approve(&f, approval(&f, "wt1"));
    let id = pending.id().clone();
    assert!(matches!(pending.finish_step(0, StepState::Done), Err(StoreError::PhaseOrder)), "before unregistering");
    pending.advance(Progress::Placed).unwrap();
    pending.advance(Progress::Unlocked).unwrap();
    assert_eq!(pending.record().progress, Progress::Placed, "never backward");
    pending.advance(Progress::Unregistered).unwrap();
    pending.finish_step(0, StepState::Done).unwrap();
    assert!(matches!(pending.finish_step(0, StepState::Kept), Err(StoreError::PhaseOrder)), "a finished step stays");
    assert!(matches!(pending.finish_step(1, StepState::Done), Err(StoreError::PhaseOrder)), "no such step");
    drop(pending);
    let Stored::Compact(saved) = f.store.read_stored(&id).unwrap() else { panic!("compact") };
    assert_eq!((saved.progress, saved.branch_steps[0].state), (Progress::Unregistered, StepState::Done));
}

#[test]
fn a_finished_operation_leaves_nothing_in_the_store() {
    let f = fixture();
    let mut pending = approve(&f, approval(&f, "wt1"));
    pending.advance(Progress::Unregistered).unwrap();
    pending.finish_step(0, StepState::Done).unwrap();
    pending.finish().unwrap();
    assert!(store_names(&f).is_empty(), "{:?}", store_names(&f));
    // The same worktree can be approved again.
    approve(&f, approval(&f, "wt1"));
}

/// A clock that releases a held lock at its `release_at`th sleep.
struct Releasing {
    held: Mutex<Option<OperationLock>>,
    release_at: usize,
    sleeps: Mutex<usize>,
}

impl Clock for Releasing {
    fn sleep(&self, _duration: Duration, _cancelled: &dyn Fn() -> bool) -> Slept {
        let mut sleeps = self.sleeps.lock().unwrap();
        *sleeps += 1;
        if *sleeps == self.release_at {
            self.held.lock().unwrap().take();
        }
        Slept::Elapsed
    }
}

#[test]
fn a_waiter_takes_the_lock_once_released_and_then_reads_what_its_owner_left() {
    let f = fixture();
    let expected = binding(&f, "wt1");
    let owner = approve(&f, approval(&f, "wt1"));
    let id = owner.id().clone();
    // The owner finishes and releases while the waiter backs off.
    owner.finish().unwrap();
    let clock = Releasing { held: Mutex::new(Some(f.store.lock(&expected).unwrap())), release_at: 3, sleeps: Mutex::new(0) };
    let lock = f.store.lock_waiting(&expected, &clock, &|| false).unwrap();
    assert_eq!(*clock.sleeps.lock().unwrap(), 3);
    assert!(f.store.operations().unwrap().is_empty(), "the owner's completion is what the waiter finds");
    assert!(matches!(f.store.read_stored(&id), Err(StoreError::Storage { .. })));
    drop(lock);

    // An owner that exited mid-operation leaves its record to the waiter.
    let mut owner = approve(&f, approval(&f, "wt1"));
    owner.advance(Progress::Placed).unwrap();
    let id = owner.id().clone();
    let clock = Releasing { held: Mutex::new(Some(owner.into_lock())), release_at: 1, sleeps: Mutex::new(0) };
    let lock = f.store.lock_waiting(&expected, &clock, &|| false).unwrap();
    let resumed = f.store.resume(lock, &id, &expected).unwrap();
    assert_eq!(resumed.record().progress, Progress::Placed);
}

#[test]
fn a_lock_wait_backs_off_and_ends_on_cancellation_without_the_lock() {
    let f = fixture();
    let expected = binding(&f, "wt1");
    let held = f.store.lock(&expected).unwrap();
    let clock = ManualClock::cancelled_after(3);
    assert!(matches!(f.store.lock_waiting(&expected, &clock, &|| false), Err(StoreError::Cancelled)));
    assert_eq!(clock.slept(), [50, 100, 200].map(Duration::from_millis));
    assert!(matches!(f.store.lock(&expected), Err(StoreError::Lock(LockError::Busy))), "still the holder's");
    let already = ManualClock::new();
    assert!(matches!(f.store.lock_waiting(&expected, &already, &|| true), Err(StoreError::Cancelled)));
    drop(held);
}

#[test]
fn a_stale_lock_file_and_a_crashed_owners_record_are_taken_over_at_once() {
    let f = fixture();
    let expected = binding(&f, "wt1");
    let crashed = approve(&f, approval(&f, "wt1"));
    let id = crashed.id().clone();
    // The process died: its lock file stays, unheld.
    drop(crashed);
    let clock = ManualClock::new();
    let lock = f.store.lock_waiting(&expected, &clock, &|| false).unwrap();
    assert!(clock.slept().is_empty());
    assert_eq!(f.store.resume(lock, &id, &expected).unwrap().record().progress, Progress::Approved);
}

#[test]
fn resume_refuses_a_replaced_worktree_record_or_repository_and_changes_nothing() {
    let f = fixture();
    let pending = approve(&f, approval(&f, "wt1"));
    let id = pending.id().clone();
    let dir = f.store.operation_dir(&id);
    drop(pending);
    let before = fs::read(dir.join(RECORD_FILE)).unwrap();
    let edits: [fn(&mut RepositoryBinding); 2] =
        [|b| b.admin_dir_identity = identity(999), |b| b.common_dir_identity = identity(998)];
    for edit in edits {
        let mut replaced = binding(&f, "wt1");
        edit(&mut replaced);
        let lock = f.store.lock(&replaced).unwrap();
        let error = f.store.resume(lock, &id, &replaced).unwrap_err();
        assert!(matches!(error, StoreError::Record { reason: RecordInvalid::IdentityMismatch, .. }), "{error:?}");
    }
    let lock = f.store.lock(&binding(&f, "wt2")).unwrap();
    assert!(matches!(f.store.resume(lock, &id, &binding(&f, "wt1")), Err(StoreError::WrongLock)));
    assert_eq!(fs::read(dir.join(RECORD_FILE)).unwrap(), before);
}

#[test]
fn corrupt_foreign_and_unknown_version_records_are_explained_and_kept() {
    let f = fixture();
    let pending = approve(&f, approval(&f, "wt1"));
    let id = pending.id().clone();
    let path = f.store.operation_dir(&id).join(RECORD_FILE);
    drop(pending);
    let doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let with = |key: &str, value: serde_json::Value| {
        let mut doc = doc.clone();
        doc[key] = value;
        serde_json::to_vec(&doc).unwrap()
    };
    let cells: [(&str, Vec<u8>, &str); 4] = [
        ("corrupt", b"{\"record_version\": 2,".to_vec(), "malformed"),
        ("unknown version", with("record_version", json!(3)), "version 3"),
        ("another operation's", with("operation_id", json!("ffffffffffffffffffffffffffffffff")), "operation id"),
        ("malformed field", with("progress", json!("completed")), "malformed"),
    ];
    for (cell, bytes, explained) in cells {
        fs::write(&path, &bytes).unwrap();
        let error = resume(&f, &id, "wt1").unwrap_err();
        assert!(matches!(error, StoreError::Record { .. }), "{cell}: {error:?}");
        let message = error.to_string();
        assert!(message.contains(id.as_str()) && message.contains(explained), "{cell}: {message}");
        assert_eq!(fs::read(&path).unwrap(), bytes, "{cell}: the record is kept as it was");
    }
}

#[test]
fn resuming_a_compact_record_deletes_leftover_journals_and_reports_without_reading_them() {
    let f = fixture();
    let pending = approve(&f, approval(&f, "wt1"));
    let id = pending.id().clone();
    let dir = f.store.operation_dir(&id);
    drop(pending);
    fs::write(dir.join(JOURNAL_FILE), b"\x00 unreadable").unwrap();
    fs::write(dir.join(REPORT_FILE), b"{").unwrap();
    let resumed = resume(&f, &id, "wt1").unwrap();
    assert_eq!(names(&dir), set_of(&[RECORD_FILE]));
    assert_eq!(resumed.record().operation_id, id);
}

#[test]
fn a_legacy_operation_migrates_to_a_compact_record_and_drops_its_journals_and_report() {
    let f = fixture();
    let legacy = legacy_record(&f, "wt1", Phase::Deleting);
    let dir = save_legacy(&f, &legacy);
    // The moved checkout's `.git` file was partly deleted; migration never
    // reads the checkout, so nothing about it needs repair first.
    let quarantine = legacy.quarantine.as_path().to_path_buf();
    fs::create_dir_all(&quarantine).unwrap();
    fs::write(quarantine.join(".git"), b"gitdir: ").unwrap();

    let resumed = resume(&f, &legacy.operation_id, "wt1").unwrap();
    let record = resumed.record().clone();
    assert_eq!((record.record_version, record.progress, record.deletion_mode), (2, Progress::Placed, DeletionMode::Moved));
    assert_eq!(record.deletion_path, legacy.quarantine);
    assert_eq!(
        record.branch_steps.iter().map(|step| (&step.action, step.state)).collect::<Vec<_>>(),
        [
            (&BranchAction::Local { name: "fix/path".into(), expected_tip: "a".repeat(40) }, StepState::Pending),
            (&BranchAction::Unbound { target: UnboundTarget::Remote { remote: "origin".into(), name: "fix/path".into() } }, StepState::Kept),
        ]
    );
    assert_eq!(names(&dir), set_of(&[RECORD_FILE]));
    assert_eq!(f.store.read_stored(&legacy.operation_id).unwrap(), Stored::Compact(Box::new(record)));
    assert_eq!(fs::read(quarantine.join(".git")).unwrap(), b"gitdir: ");
}

#[test]
fn a_migration_stopped_before_publishing_leaves_the_legacy_files_and_can_run_again() {
    for fault_kind in [Fault::Crash, Fault::Fail] {
        let f = fixture();
        let legacy = legacy_record(&f, "wt1", Phase::Moved);
        let dir = save_legacy(&f, &legacy);
        let before: Vec<Vec<u8>> = LEGACY_FILES.iter().map(|name| fs::read(dir.join(name)).unwrap()).collect();
        {
            let _injected = fault::inject(move |step| (step == SaveStep::Migrated).then_some(fault_kind));
            assert!(resume(&f, &legacy.operation_id, "wt1").is_err(), "{fault_kind:?}");
        }
        let after: Vec<Vec<u8>> = LEGACY_FILES.iter().map(|name| fs::read(dir.join(name)).unwrap()).collect();
        assert_eq!(after, before, "{fault_kind:?}: the originals are intact");
        assert!(matches!(f.store.read_stored(&legacy.operation_id).unwrap(), Stored::Legacy(_)));

        resume(&f, &legacy.operation_id, "wt1").unwrap();
        assert_eq!(names(&dir), set_of(&[RECORD_FILE]), "{fault_kind:?}");
    }
}

#[test]
fn a_migration_stopped_after_publishing_resumes_from_the_compact_record() {
    let f = fixture();
    let legacy = legacy_record(&f, "wt1", Phase::ContentRemoved);
    let dir = save_legacy(&f, &legacy);
    {
        let _injected = fault::inject(|step| (step == SaveStep::MigrationPublished).then_some(Fault::Crash));
        assert!(resume(&f, &legacy.operation_id, "wt1").is_err());
    }
    assert!(names(&dir).contains(JOURNAL_FILE), "the crash came before the leftovers went");
    let Stored::Compact(published) = f.store.read_stored(&legacy.operation_id).unwrap() else { panic!("compact") };
    assert_eq!(published.progress, Progress::ContentRemoved);
    let resumed = resume(&f, &legacy.operation_id, "wt1").unwrap();
    assert_eq!(resumed.record(), &*published);
    assert_eq!(names(&dir), set_of(&[RECORD_FILE]));
}

#[test]
fn a_legacy_operation_migrates_without_a_readable_journal_or_report() {
    let f = fixture();
    let legacy = legacy_record(&f, "wt1", Phase::Associated);
    let dir = save_legacy(&f, &legacy);
    fs::remove_file(dir.join(JOURNAL_FILE)).unwrap();
    fs::write(dir.join(REPORT_FILE), b"\xff\xfe").unwrap();
    assert_eq!(resume(&f, &legacy.operation_id, "wt1").unwrap().record().progress, Progress::Placed);
}

#[test]
fn a_legacy_operation_of_a_replaced_worktree_record_is_never_migrated() {
    let f = fixture();
    let legacy = legacy_record(&f, "wt1", Phase::Prepared);
    let dir = save_legacy(&f, &legacy);
    let mut replaced = binding(&f, "wt1");
    replaced.admin_dir_identity = identity(999);
    let lock = f.store.lock(&replaced).unwrap();
    let error = f.store.resume(lock, &legacy.operation_id, &replaced).unwrap_err();
    assert!(matches!(error, StoreError::Record { reason: RecordInvalid::IdentityMismatch, .. }), "{error:?}");
    assert_eq!(names(&dir), set_of(&LEGACY_FILES));
    assert!(matches!(f.store.read_stored(&legacy.operation_id).unwrap(), Stored::Legacy(_)));
}

#[test]
fn a_completed_legacy_operation_is_deleted_rather_than_resumed() {
    let f = fixture();
    let legacy = legacy_record(&f, "wt1", Phase::Completed);
    save_legacy(&f, &legacy);
    let error = resume(&f, &legacy.operation_id, "wt1").unwrap_err();
    assert!(matches!(&error, StoreError::Completed(id) if *id == legacy.operation_id), "{error:?}");
    assert!(store_names(&f).is_empty(), "{:?}", store_names(&f));
}
