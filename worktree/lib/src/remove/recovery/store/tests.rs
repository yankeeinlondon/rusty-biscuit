use std::collections::BTreeSet;

use serde_json::{Value, json};
use sniff::filesystem::query::{FileIdentity, NativeString, PathUsageReport};
use tempfile::TempDir;

use super::*;
use crate::remove::observation::inventory::DeletionInventory;
use crate::remove::outcome::RemovalOutcome;
use crate::remove::recovery::journal::{EntryKind, EntryState, JournalEntry, PlannedEntry, ResultState};
use super::fault::{self, Fault};
use crate::remove::recovery::record::{BranchBinding, IntendedAction, RECORD_VERSION};

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

fn record(f: &Fixture, admin: &str) -> RecoveryRecord {
    let original = f.work.join(admin);
    RecoveryRecord {
        record_version: RECORD_VERSION,
        operation_id: OperationId::generate().unwrap(),
        created_at_ms: 1,
        requested_name: "fix/path".into(),
        aliases: vec!["fix-path".into()],
        original: original.as_path().into(),
        quarantine: f.work.join(format!(".wt-remove-{admin}-a91c")).as_path().into(),
        checkout_identity: identity(3),
        repository: binding(f, admin),
        branch: BranchBinding::Named("fix/path".into()),
        approved_tip: "a".repeat(40),
        consent_fingerprint: "b".repeat(64),
        discovery: discovery(&original),
        intended_actions: vec![IntendedAction::RemoveCheckout, IntendedAction::DeleteLocalBranch { name: "fix/path".into() }],
        completed_phase: Phase::Prepared,
    }
}

/// The inventory every test operation is approved to delete: one hashed
/// file first (so the matrix's first-occurrence edits land on it), files and
/// directories, and a link.
fn inventory_value() -> Value {
    let id = |inode: u64| json!({"kind": "unix", "device": "7", "inode": inode.to_string()});
    let file = |path: &str, inode: u64, content: Value| {
        json!({"path": {"display": path}, "identity": id(inode),
               "shape": {"file": {"size": 5, "modified": {"secs": 1_700_000_000, "nanos": 5}, "mode": 420, "content": content}}})
    };
    let dir = |path: &str, inode: u64| json!({"path": {"display": path}, "identity": id(inode), "shape": {"directory": {"mode": 493}}});
    json!({
        "root": id(3),
        "entries": [
            file("a", 10, json!({"blake3": "ab".repeat(32)})),
            dir("d", 11),
            file("d/a", 12, json!("unhashed")),
            file("d/b", 13, json!("unhashed")),
            dir("e", 14),
            file("e/x", 15, json!("unhashed")),
            {"path": {"display": "l"}, "identity": id(16), "shape": {"link": {"target": {"display": "../elsewhere"}}}},
        ]
    })
}

fn inventory() -> Arc<DeletionInventory> {
    Arc::new(serde_json::from_value(inventory_value()).unwrap())
}

fn begin(f: &Fixture, record: RecoveryRecord) -> Operation {
    let lock = f.store.lock(&record.repository).unwrap();
    f.store.begin(lock, record, inventory()).unwrap()
}

fn file(path: &str) -> PlannedEntry {
    PlannedEntry { path: NativeString::from(path), kind: EntryKind::File }
}

fn everything_present(_: &Path) -> io::Result<bool> {
    Ok(true)
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn begin_makes_an_owner_only_pending_operation_that_reads_back() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let op = begin(&f, saved.clone());
    let dir = op.dir().to_path_buf();
    assert_eq!(dir, f.common.join(STORE_DIR).join(saved.operation_id.as_str()));
    #[cfg(unix)]
    {
        assert_eq!(mode(&f.common.join(STORE_DIR)), 0o700);
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&dir.join(RECORD_FILE)), 0o600);
        assert_eq!(mode(&dir.join(JOURNAL_FILE)), 0o600);
    }
    drop(op);

    let pending = f.store.pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].record.as_ref().unwrap(), &saved);
    let recovered = f.store.recover(&saved.operation_id, &saved.repository, everything_present).unwrap();
    assert_eq!(recovered.record, saved);
    assert!(recovered.entries.is_empty());
    assert!(!recovered.torn_tail);
}

#[test]
fn a_held_lock_refuses_a_second_removal_and_a_stale_lock_file_does_not() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let held = f.store.lock(&saved.repository).unwrap();
    assert!(matches!(f.store.lock(&saved.repository), Err(StoreError::Lock(LockError::Busy))));
    drop(held);

    // The lock file remains; only a held lock means an active operation.
    let op = begin(&f, saved.clone());
    assert!(matches!(f.store.lock(&saved.repository), Err(StoreError::Lock(LockError::Busy))));
    drop(op);
    assert!(f.store.lock(&saved.repository).is_ok());
}

#[test]
fn the_lock_covers_only_the_worktree_it_was_taken_for() {
    let f = fixture();
    let lock = f.store.lock(&binding(&f, "wt1")).unwrap();
    assert!(matches!(f.store.begin(lock, record(&f, "wt2"), inventory()), Err(StoreError::WrongLock)));
    assert!(f.store.pending().unwrap().is_empty());
}

#[test]
fn a_pending_record_blocks_a_new_operation_on_the_same_worktree_only() {
    let f = fixture();
    let first = record(&f, "wt1");
    // Dropping the operation is a crash: the lock goes, the record stays.
    drop(begin(&f, first.clone()));

    let again = record(&f, "wt1");
    let lock = f.store.lock(&again.repository).unwrap();
    match f.store.begin(lock, again, inventory()) {
        Err(StoreError::Pending(id)) => assert_eq!(id, first.operation_id),
        other => panic!("expected the pending record to block: {:?}", other.map(|op| op.id().clone())),
    }
    drop(begin(&f, record(&f, "wt2")));
    assert_eq!(f.store.pending().unwrap().len(), 2);
}

#[test]
fn an_unreadable_record_blocks_every_new_operation() {
    let f = fixture();
    let first = record(&f, "wt1");
    drop(begin(&f, first.clone()));
    let path = f.store.operation_dir(&first.operation_id).join(RECORD_FILE);
    storage::write_durable(&path, b"{\"record_version\":1}").unwrap();

    let other = record(&f, "wt2");
    let lock = f.store.lock(&other.repository).unwrap();
    assert!(matches!(f.store.begin(lock, other, inventory()), Err(StoreError::Record { .. })));
    let pending = f.store.pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert!(matches!(pending[0].record, Err(StoreError::Record { .. })));
}

#[test]
fn a_staging_directory_left_by_a_crash_is_not_an_operation() {
    let f = fixture();
    let id = OperationId::generate().unwrap();
    let staging = f.store.root().join(format!(".{id}.tmp"));
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join(RECORD_FILE), b"half").unwrap();
    assert!(f.store.pending().unwrap().is_empty());
    drop(begin(&f, record(&f, "wt1")));
}

#[test]
fn advance_persists_each_phase_and_only_forward() {
    let f = fixture();
    let mut op = begin(&f, record(&f, "wt1"));
    op.advance(Phase::Moved).unwrap();
    assert_eq!(f.store.read_record(op.id()).unwrap().completed_phase, Phase::Moved);
    assert!(matches!(op.advance(Phase::Prepared), Err(StoreError::PhaseOrder)));
    assert!(matches!(op.advance(Phase::Moved), Err(StoreError::PhaseOrder)));
    assert!(matches!(op.advance(Phase::Completed), Err(StoreError::PhaseOrder)));
    assert_eq!(op.record().completed_phase, Phase::Moved);
}

#[cfg(unix)]
#[test]
fn a_failed_phase_write_keeps_the_earlier_phase_on_disk_and_in_memory() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    let mut op = begin(&f, record(&f, "wt1"));
    op.advance(Phase::Moved).unwrap();
    let dir = op.dir().to_path_buf();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();
    let result = op.advance(Phase::Associated);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(result, Err(StoreError::Io(_))), "{result:?}");
    assert_eq!(op.record().completed_phase, Phase::Moved);
    assert_eq!(f.store.read_record(op.id()).unwrap().completed_phase, Phase::Moved);
}

#[cfg(unix)]
#[test]
fn a_record_that_cannot_be_created_leaves_no_operation() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    let saved = record(&f, "wt1");
    let lock = f.store.lock(&saved.repository).unwrap();
    fs::set_permissions(f.store.root(), fs::Permissions::from_mode(0o500)).unwrap();
    let result = f.store.begin(lock, saved, inventory());
    fs::set_permissions(f.store.root(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(result, Err(StoreError::Io(_))));
    let names: Vec<_> = fs::read_dir(f.store.root()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, vec![std::ffi::OsString::from(LOCKS_DIR)]);
}

#[test]
fn completion_keeps_the_report_drops_the_journal_and_never_authorizes_again() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let mut op = begin(&f, saved.clone());
    op.journal().begin_batch(vec![file("a")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted]).unwrap();
    let dir = op.dir().to_path_buf();
    let report = OperationReport::new(Some(saved.operation_id.clone()), RemovalOutcome::Removed, saved.original.clone());

    assert_eq!(op.complete(&report, true).unwrap(), Vec::new());
    let saved_report: OperationReport = serde_json::from_slice(&fs::read(dir.join(REPORT_FILE)).unwrap()).unwrap();
    assert_eq!(saved_report, report);
    assert!(!dir.join(JOURNAL_FILE).exists());
    assert_eq!(f.store.read_record(&saved.operation_id).unwrap().completed_phase, Phase::Completed);
    assert!(f.store.pending().unwrap().is_empty());
    assert!(matches!(
        f.store.recover(&saved.operation_id, &saved.repository, everything_present),
        Err(StoreError::Completed(_))
    ));
    drop(begin(&f, record(&f, "wt1")));
}

#[test]
fn a_journal_that_cannot_be_cleared_is_a_warning_after_completion() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let op = begin(&f, saved.clone());
    let journal = op.dir().join(JOURNAL_FILE);
    fs::rename(&journal, op.dir().join("journal.moved")).unwrap();
    fs::create_dir(&journal).unwrap();
    fs::write(journal.join("keep"), b"x").unwrap();

    let report = OperationReport::new(Some(saved.operation_id.clone()), RemovalOutcome::Removed, saved.original.clone());
    let warnings = op.complete(&report, true).unwrap();
    assert!(matches!(&warnings[..], [BookkeepingWarning::JournalNotCleared { path, .. }] if *path == journal));
    assert!(f.store.pending().unwrap().is_empty());
}

#[test]
fn a_linked_operation_directory_is_never_read() {
    let f = fixture();
    let saved = record(&f, "wt1");
    drop(begin(&f, saved.clone()));
    let dir = f.store.operation_dir(&saved.operation_id);
    let elsewhere = f.common.parent().unwrap().join("elsewhere");
    fs::rename(&dir, &elsewhere).unwrap();
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&elsewhere, &dir);
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(&elsewhere, &dir);
    if linked.is_err() {
        // Windows without symlink privilege cannot build this fixture.
        return;
    }
    let pending = f.store.pending().unwrap();
    assert!(matches!(pending[0].record, Err(StoreError::Storage { problem: StorageProblem::Link, .. })));
    assert!(matches!(
        f.store.recover(&saved.operation_id, &saved.repository, everything_present),
        Err(StoreError::Storage { problem: StorageProblem::Link, .. })
    ));
}

// ---------------------------------------------------------------------------
// Input robustness matrices. Each starts from files the store itself wrote,
// applies one edit per cell, and asserts through `recover`.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    Valid,
    Invalid,
}

/// A record field: its name, pointer, wrong-type value, empty value with its
/// expected outcome (when the shape applies), and an invalid value.
type RecordField<'a> = (&'a str, &'a str, Value, Option<(Value, Expect)>, Value);

/// A journal cell: its name, the journal, and the expected outcome with the
/// recovered entry states.
type JournalCell<'a> = (String, Vec<u8>, Expect, Vec<(&'a str, EntryState)>);

fn edited(original: &Value, pointer: &str, edit: impl FnOnce(&mut Value, &str)) -> Vec<u8> {
    let mut value = original.clone();
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let parent = if parent.is_empty() { &mut value } else { value.pointer_mut(parent).unwrap() };
    edit(parent, key);
    serde_json::to_vec(&value).unwrap()
}

fn set(original: &Value, pointer: &str, replacement: Value) -> Vec<u8> {
    edited(original, pointer, |parent, key| match parent {
        Value::Object(map) => {
            map.insert(key.to_string(), replacement);
        }
        Value::Array(items) => items[key.parse::<usize>().unwrap()] = replacement,
        _ => unreachable!(),
    })
}

fn remove(original: &Value, pointer: &str) -> Vec<u8> {
    edited(original, pointer, |parent, key| {
        parent.as_object_mut().unwrap().remove(key).unwrap();
    })
}

/// Repeats `key`'s first member, with the same value, in compact JSON.
fn duplicate(original: &Value, key: &str) -> Vec<u8> {
    let text = serde_json::to_string(original).unwrap();
    let needle = format!("\"{key}\":");
    let start = text.find(&needle).unwrap_or_else(|| panic!("{key} is in the fixture"));
    let value_start = start + needle.len();
    let mut stream = serde_json::Deserializer::from_str(&text[value_start..]).into_iter::<Value>();
    stream.next().unwrap().unwrap();
    let value = &text[value_start..value_start + stream.byte_offset()];
    format!("{}{needle}{value},{}", &text[..start], &text[start..]).into_bytes()
}

#[test]
fn record_robustness_matrix() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let id = saved.operation_id.clone();
    let mut op = begin(&f, saved.clone());
    op.journal().begin_batch(vec![file("a")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted]).unwrap();
    drop(op);
    let path = f.store.operation_dir(&id).join(RECORD_FILE);
    let written = fs::read(&path).unwrap();
    let doc: Value = serde_json::from_slice(&written).unwrap();
    let other_id = "ffffffffffffffffffffffffffffffff";

    use Expect::*;
    // (field, pointer, wrong type, empty, invalid content)
    let fields: Vec<RecordField> = vec![
        ("record_version", "/record_version", json!("1"), None, json!(2)),
        ("operation_id", "/operation_id", json!(123), Some((json!(""), Invalid)), json!(other_id)),
        ("original", "/original", json!(123), Some((json!({}), Invalid)), json!({"display": "relative/wt1"})),
        ("quarantine", "/quarantine", json!(123), Some((json!({}), Invalid)), json!({"display": f.work.join("wt1-copy")})),
        (
            "checkout_identity",
            "/checkout_identity",
            json!(123),
            Some((json!({}), Invalid)),
            json!({"kind": "unix", "device": "7", "inode": "3", "extra": "1"}),
        ),
        (
            "common_dir_identity",
            "/repository/common_dir_identity",
            json!(123),
            Some((json!({}), Invalid)),
            json!({"kind": "unix", "device": "7", "inode": "999"}),
        ),
        (
            "admin_dir_identity",
            "/repository/admin_dir_identity",
            json!(123),
            Some((json!({}), Invalid)),
            json!({"kind": 0, "device": "7", "inode": "103"}),
        ),
        ("admin_dir", "/repository/admin_dir", json!(123), Some((json!({}), Invalid)), json!({"display": f.common.join("wt1")})),
        ("branch", "/branch", json!(123), Some((json!({}), Invalid)), json!({"named": ""})),
        ("approved_tip", "/approved_tip", json!(123), Some((json!(""), Invalid)), json!("not-a-commit")),
        ("consent_fingerprint", "/consent_fingerprint", json!(123), Some((json!(""), Invalid)), json!("B".repeat(64))),
        ("completed_phase", "/completed_phase", json!(123), Some((json!(""), Invalid)), json!("flying")),
        ("aliases", "/aliases", json!("fix-path"), Some((json!([]), Valid)), json!([""])),
        (
            "intended_actions",
            "/intended_actions",
            json!("remove_checkout"),
            Some((json!([]), Invalid)),
            json!(["remove_checkout", "remove_checkout"]),
        ),
    ];

    let mut cells: Vec<(String, Vec<u8>, Expect)> = vec![("control".into(), written.clone(), Valid)];
    for (field, pointer, wrong_type, empty, invalid) in fields {
        let key = pointer.rsplit('/').next().unwrap();
        cells.push((format!("{field}: absent"), remove(&doc, pointer), Invalid));
        cells.push((format!("{field}: null"), set(&doc, pointer, Value::Null), Invalid));
        cells.push((format!("{field}: wrong type"), set(&doc, pointer, wrong_type), Invalid));
        if let Some((empty, expect)) = empty {
            cells.push((format!("{field}: empty"), set(&doc, pointer, empty), expect));
        }
        cells.push((format!("{field}: duplicate key"), duplicate(&doc, key), Invalid));
        cells.push((format!("{field}: invalid content"), set(&doc, pointer, invalid), Invalid));
    }
    for (field, pointer, valid) in [("aliases", "/aliases", json!("fix-path")), ("intended_actions", "/intended_actions", json!("remove_checkout"))] {
        cells.push((format!("{field}: one element wrong type"), set(&doc, pointer, json!([valid, 123])), Invalid));
        cells.push((format!("{field}: every element wrong type"), set(&doc, pointer, json!([123])), Invalid));
    }
    // A repeated key inside a nested, internally tagged value.
    let text = String::from_utf8(serde_json::to_vec(&doc).unwrap()).unwrap();
    let inner = text.replacen("\"checkout_identity\":{", "\"checkout_identity\":{\"inode\":\"3\",", 1);
    cells.push(("checkout_identity: duplicate inner key".into(), inner.into_bytes(), Invalid));
    let mut trailing = written.clone();
    trailing.extend_from_slice(b"\n{}");
    cells.push(("document: trailing content".into(), trailing, Invalid));
    cells.push(("document: unknown field".into(), set(&doc, "/surprise", json!(true)), Invalid));
    cells.push(("document: not JSON".into(), b"record".to_vec(), Invalid));

    let mut failures = Vec::new();
    for (cell, bytes, expect) in cells {
        // A plain rewrite keeps the owner-only file; durability is not under test.
        fs::write(&path, &bytes).unwrap();
        let outcome = f.store.recover(&id, &saved.repository, everything_present);
        let ok = match (&outcome, expect) {
            (Ok(recovered), Valid) => recovered.entries.len() == 1,
            (Err(StoreError::Record { .. }), Invalid) => true,
            _ => false,
        };
        if !ok {
            failures.push(format!("{cell}: expected {expect:?}, got {:?}", outcome.map(|r| r.record.operation_id)));
        }
    }
    assert!(failures.is_empty(), "matrix cells failed:\n{}", failures.join("\n"));

    // The reasons a user is shown for the three semantic refusals.
    let reason = |bytes: Vec<u8>| {
        fs::write(&path, &bytes).unwrap();
        match f.store.recover(&id, &saved.repository, everything_present) {
            Err(StoreError::Record { reason, .. }) => reason,
            other => panic!("expected a record refusal, got {:?}", other.map(|r| r.record.operation_id)),
        }
    };
    assert_eq!(reason(set(&doc, "/record_version", json!(2))), RecordInvalid::UnsupportedVersion(2));
    assert_eq!(reason(set(&doc, "/operation_id", json!(other_id))), RecordInvalid::OperationMismatch);
    assert_eq!(
        reason(set(&doc, "/repository/common_dir_identity", json!({"kind": "unix", "device": "7", "inode": "999"}))),
        RecordInvalid::IdentityMismatch
    );
}

/// The operation a journal matrix edits: its store and record, its journal
/// file, and that journal's lines, which each cell edits one at a time.
///
/// Inventory positions: 0 `a`, 1 `d`, 2 `d/a`, 3 `d/b`, 4 `e`, 5 `e/x`, 6 `l`.
struct JournalMatrix {
    f: Fixture,
    saved: RecoveryRecord,
    path: PathBuf,
    lines: Vec<Value>,
}

/// What every unedited matrix journal recovers with only `d/b` present:
/// batch 1's results, and batch 2 cut short.
fn journal_control() -> Vec<(&'static str, EntryState)> {
    vec![("d/a", EntryState::Deleted), ("d/b", EntryState::Kept), ("e/x", EntryState::Uncertain)]
}

/// The batches of [`JournalMatrix::version_3`] as a version 2 `wt` wrote
/// them, frozen here because this build writes only version 3.
fn version_2_lines(id: &OperationId) -> Vec<Value> {
    vec![
        json!({"header": {"journal_version": 2, "operation_id": id.as_str()}}),
        json!({"inventory": inventory_value()}),
        json!({"intent": {"batch": 1, "entries": [{"path": {"display": "d/a"}, "kind": "file"}, {"path": {"display": "d/b"}, "kind": "file"}]}}),
        json!({"results": {"batch": 1, "entries": [{"path": {"display": "d/a"}, "state": "deleted"}, {"path": {"display": "d/b"}, "state": "kept"}]}}),
        json!({"intent": {"batch": 2, "entries": [{"path": {"display": "e/x"}, "kind": "file"}]}}),
    ]
}

/// A journal line's field: its name, line index, pointer, wrong-type value,
/// empty value, and named invalid contents.
type JournalField<'a> = (&'a str, usize, &'a str, Value, Value, Vec<(&'a str, Value)>);

fn compact(line: &Value) -> Vec<u8> {
    serde_json::to_vec(line).unwrap()
}

fn join_lines(lines: &[Vec<u8>]) -> Vec<u8> {
    lines.iter().flat_map(|line| line.iter().copied().chain(*b"\n")).collect()
}

impl JournalMatrix {
    /// The journal the writer leaves when batch 1 planned `d/a` and `d/b`
    /// (deleted, kept) and batch 2 planned `e/x` and was cut short.
    fn version_3() -> Self {
        let f = fixture();
        let saved = record(&f, "wt1");
        let mut op = begin(&f, saved.clone());
        op.journal().begin_batch(vec![file("d/a"), file("d/b")]).unwrap();
        op.journal().finish_batch(&[ResultState::Deleted, ResultState::Kept]).unwrap();
        op.journal().begin_batch(vec![file("e/x")]).unwrap();
        drop(op);
        let path = f.store.operation_dir(&saved.operation_id).join(JOURNAL_FILE);
        let written = fs::read(&path).unwrap();
        let lines: Vec<Value> =
            written.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()).map(|line| serde_json::from_slice(line).unwrap()).collect();
        assert_eq!(lines.len(), 5, "header, inventory, intent, results, open intent");
        assert_eq!(lines[0]["header"]["journal_version"], json!(3));
        assert_eq!(lines[1]["inventory"], inventory_value(), "the inventory is persisted as given");
        assert_eq!(lines[2], json!({"intent": {"batch": 1, "entries": [2, 3]}}));
        assert_eq!(lines[3], json!({"results": {"batch": 1, "states": ["deleted", "kept"]}}));
        assert_eq!(lines[4], json!({"intent": {"batch": 2, "entries": [5]}}));
        Self { f, saved, path, lines }
    }

    /// The same operation with its journal replaced by [`version_2_lines`].
    fn version_2() -> Self {
        let f = fixture();
        let saved = record(&f, "wt1");
        drop(begin(&f, saved.clone()));
        let path = f.store.operation_dir(&saved.operation_id).join(JOURNAL_FILE);
        let lines = version_2_lines(&saved.operation_id);
        Self { f, saved, path, lines }
    }

    fn line(&self, index: usize) -> Vec<u8> {
        compact(&self.lines[index])
    }

    /// The journal holding only the lines at `indices`, in that order.
    fn only(&self, indices: &[usize]) -> Vec<u8> {
        join_lines(&indices.iter().map(|&index| self.line(index)).collect::<Vec<_>>())
    }

    fn all(&self) -> Vec<u8> {
        self.only(&(0..self.lines.len()).collect::<Vec<_>>())
    }

    fn with_line(&self, index: usize, replacement: Vec<u8>) -> Vec<u8> {
        let mut all: Vec<Vec<u8>> = (0..self.lines.len()).map(|at| self.line(at)).collect();
        all[index] = replacement;
        join_lines(&all)
    }

    /// The whole journal and one more terminated line.
    fn appended(&self, extra: &[u8]) -> Vec<u8> {
        let mut bytes = self.all();
        bytes.extend_from_slice(extra);
        bytes.push(b'\n');
        bytes
    }

    /// The lines at `indices`, then `torn` without a newline.
    fn torn(&self, indices: &[usize], torn: &[u8]) -> Vec<u8> {
        let mut bytes = self.only(indices);
        bytes.extend_from_slice(torn);
        bytes
    }

    /// Recovers the operation with `bytes` as its journal and only `d/b`
    /// present.
    fn recover(&self, bytes: &[u8]) -> Result<Recovered, StoreError> {
        fs::write(&self.path, bytes).unwrap();
        let present = BTreeSet::from(["d/b"]);
        self.f.store.recover(&self.saved.operation_id, &self.saved.repository, |path| Ok(present.contains(path.to_str().unwrap())))
    }

    /// Why the journal `bytes` is refused.
    fn refusal(&self, bytes: &[u8]) -> JournalInvalid {
        match self.recover(bytes) {
            Err(StoreError::Journal { reason, .. }) => reason,
            other => panic!("expected a journal refusal, got {:?}", other.map(|recovered| recovered.entries)),
        }
    }

    /// The 1-based line a refused journal reports, and why.
    fn malformed_at(&self, bytes: &[u8]) -> (usize, String) {
        match self.refusal(bytes) {
            JournalInvalid::Malformed { line, reason } => (line, reason),
            other => panic!("expected a malformed line, got {other:?}"),
        }
    }

    /// A valid cell recovers exactly its states, the matrix inventory, and a
    /// torn tail when its name starts with "torn"; an invalid one is refused
    /// as a journal or inventory problem.
    fn assert_cells(&self, cells: Vec<JournalCell>) {
        let mut failures = Vec::new();
        for (cell, bytes, expect, states) in cells {
            let outcome = self.recover(&bytes);
            let ok = match (&outcome, expect) {
                (Ok(recovered), Expect::Valid) => {
                    let got: Vec<_> = recovered.entries.iter().map(|entry| (entry.path.display().into_owned(), entry.state)).collect();
                    let want: Vec<_> = states.iter().map(|(path, state)| (path.to_string(), *state)).collect();
                    got == want && recovered.torn_tail == cell.starts_with("torn") && recovered.inventory == *inventory()
                }
                (Err(StoreError::Journal { .. } | StoreError::InventoryMismatch), Expect::Invalid) => true,
                _ => false,
            };
            if !ok {
                failures.push(format!("{cell}: expected {expect:?}, got {:?}", outcome.map(|r| (r.entries, r.torn_tail))));
            }
        }
        assert!(failures.is_empty(), "matrix cells failed:\n{}", failures.join("\n"));
    }
}

/// Version 3 batch lines, the header, and the document shape, against the
/// writer's own output. Inventory fields are in
/// `journal_robustness_matrix_shared_inventory`.
#[test]
fn journal_robustness_matrix_version_3() {
    let m = JournalMatrix::version_3();
    let lines = &m.lines;
    let id = m.saved.operation_id.clone();
    let other_id = "ffffffffffffffffffffffffffffffff";

    use EntryState::*;
    use Expect::*;
    let control = journal_control();
    let mut cells: Vec<JournalCell> = vec![
        ("control: as written".into(), fs::read(&m.path).unwrap(), Valid, control.clone()),
        ("control: re-encoded".into(), m.all(), Valid, control.clone()),
        ("header and inventory only".into(), m.only(&[0, 1]), Valid, Vec::new()),
        (
            "results: closing the open batch".into(),
            m.appended(br#"{"results":{"batch":2,"states":["deleted"]}}"#),
            Valid,
            vec![("d/a", Deleted), ("d/b", Kept), ("e/x", Deleted)],
        ),
    ];

    // Positions, in the open batch 2 so they are planned alone.
    let open = |entries: Value| m.with_line(4, set(&lines[4], "/intent/entries", entries));
    let open_text = |entries: &str| m.with_line(4, format!("{{\"intent\":{{\"batch\":2,\"entries\":{entries}}}}}").into_bytes());
    cells.push(("index: zero".into(), open(json!([0])), Valid, vec![("d/a", Deleted), ("d/b", Kept), ("a", Uncertain)]));
    cells.push(("index: a directory".into(), open(json!([1])), Valid, vec![("d/a", Deleted), ("d/b", Kept), ("d", Uncertain)]));
    cells.push(("index: the last".into(), open(json!([6])), Valid, vec![("d/a", Deleted), ("d/b", Kept), ("l", Uncertain)]));
    for (state, recovered) in [("deleted", Deleted), ("externally_absent", ExternallyAbsent), ("kept", Kept)] {
        cells.push((
            format!("results states: {state}"),
            m.with_line(3, set(&lines[3], "/results/states/0", json!(state))),
            Valid,
            vec![("d/a", recovered), ("d/b", Kept), ("e/x", Uncertain)],
        ));
    }

    let mut invalid = |cell: &str, bytes: Vec<u8>| cells.push((cell.to_string(), bytes, Invalid, Vec::new()));

    // (field, line, pointer, wrong type, empty, invalid contents). Each is also
    // absent, null, and repeated; no field of these lines may be empty.
    let fields: Vec<JournalField> = vec![
        (
            "header journal_version",
            0,
            "/header/journal_version",
            json!("3"),
            json!(""),
            vec![
                ("version 1", json!(1)),
                ("version 4", json!(4)),
                ("version 0", json!(0)),
                ("negative", json!(-3)),
                ("fractional", json!(3.5)),
                ("past u32", json!(u64::from(u32::MAX) + 4)),
            ],
        ),
        (
            "header operation_id",
            0,
            "/header/operation_id",
            json!(123),
            json!(""),
            vec![("another operation", json!(other_id)), ("upper case", json!(id.as_str().to_uppercase())), ("too long", json!(format!("{id}0")))],
        ),
        (
            "intent batch",
            2,
            "/intent/batch",
            json!("1"),
            json!(""),
            vec![("zero", json!(0)), ("second", json!(2)), ("negative", json!(-1)), ("fractional", json!(1.5))],
        ),
        (
            "intent entries",
            2,
            "/intent/entries",
            json!(2),
            json!([]),
            vec![("one element wrong type", json!([2, "3"])), ("every element wrong type", json!(["2", "3"]))],
        ),
        (
            "results batch",
            3,
            "/results/batch",
            json!("1"),
            json!(""),
            vec![("zero", json!(0)), ("a batch that is not open", json!(2)), ("fractional", json!(1.5))],
        ),
        (
            "results states",
            3,
            "/results/states",
            json!("deleted"),
            json!([]),
            vec![
                ("one element wrong type", json!(["deleted", 3])),
                ("every element wrong type", json!([1, 2])),
                ("too few", json!(["deleted"])),
                ("too many", json!(["deleted", "kept", "kept"])),
            ],
        ),
    ];
    for (field, line, pointer, wrong_type, empty, contents) in fields {
        let key = pointer.rsplit('/').next().unwrap();
        invalid(&format!("{field}: absent"), m.with_line(line, remove(&lines[line], pointer)));
        invalid(&format!("{field}: null"), m.with_line(line, set(&lines[line], pointer, Value::Null)));
        invalid(&format!("{field}: wrong type"), m.with_line(line, set(&lines[line], pointer, wrong_type)));
        invalid(&format!("{field}: empty"), m.with_line(line, set(&lines[line], pointer, empty)));
        invalid(&format!("{field}: duplicate key"), m.with_line(line, duplicate(&lines[line], key)));
        for (content, value) in contents {
            invalid(&format!("{field}: {content}"), m.with_line(line, set(&lines[line], pointer, value)));
        }
    }

    for (state, value) in
        [("unknown", json!("vanished")), ("capitalized", json!("Deleted")), ("numeric", json!(0)), ("null", Value::Null), ("empty", json!(""))]
    {
        invalid(&format!("results states: {state}"), m.with_line(3, set(&lines[3], "/results/states/0", value)));
    }

    invalid("index: first out of range", open(json!([7])));
    invalid("index: negative", open(json!([-1])));
    invalid("index: fractional", open(json!([5.5])));
    invalid("index: string", open(json!(["5"])));
    invalid("index: u64::MAX", open(json!([u64::MAX])));
    invalid("index: past u64", open_text("[18446744073709551616]"));
    invalid("index: exponent form", open_text("[5e0]"));
    // Every position is valid; the batch is one entry over the bound.
    invalid("index: more than 256 entries", open(json!(vec![5; 257])));
    invalid("intent: mixed parents", m.with_line(2, set(&lines[2], "/intent/entries", json!([2, 5]))));
    invalid("intent: duplicate within a batch", m.with_line(2, set(&lines[2], "/intent/entries", json!([2, 2]))));
    invalid("intent: duplicate across batches", open(json!([3])));
    invalid("intent: batch numbers skip one", m.with_line(4, set(&lines[4], "/intent/batch", json!(3))));
    invalid("intent: batch 1 again", m.with_line(4, set(&lines[4], "/intent/batch", json!(1))));
    invalid("intent: before the previous results", m.only(&[0, 1, 2, 4]));
    invalid("results: with no open intent", m.only(&[0, 1, 3]));
    invalid("results: repeated", m.only(&[0, 1, 2, 3, 3]));

    for (line, outer) in [(0, "header"), (2, "intent"), (3, "results")] {
        invalid(&format!("{outer}: duplicate line key"), m.with_line(line, duplicate(&lines[line], outer)));
        invalid(&format!("{outer}: extra member"), m.with_line(line, set(&lines[line], &format!("/{outer}/extra"), json!(1))));
        invalid(&format!("{outer}: extra line member"), m.with_line(line, set(&lines[line], "/extra", json!(1))));
    }
    let version_2 = version_2_lines(&id);
    invalid("intent: version 2 shape", m.with_line(2, compact(&version_2[2])));
    invalid("results: version 2 shape", m.with_line(3, compact(&version_2[3])));
    invalid("header: version 2 over version 3 batches", m.with_line(0, compact(&version_2[0])));

    let mut garbage = m.line(2);
    garbage.extend_from_slice(b" junk");
    invalid("line: trailing garbage", m.with_line(2, garbage));
    invalid("line: two values on one line", m.with_line(2, [m.line(2), m.line(2)].concat()));
    let mut garbage = m.line(0);
    garbage.extend_from_slice(b"{}");
    invalid("header: trailing garbage", m.with_line(0, garbage));
    invalid("line: not JSON", m.with_line(2, b"intent".to_vec()));
    invalid("line: blank", m.with_line(2, Vec::new()));
    invalid("line: blank at the end", m.appended(b""));
    invalid("document: empty", Vec::new());
    invalid("document: a newline only", b"\n".to_vec());
    invalid("header: absent", m.only(&[1, 2, 3, 4]));
    invalid("header: repeated", m.with_line(2, m.line(0)));
    invalid("inventory: absent", m.only(&[0, 2, 3, 4]));
    invalid("inventory: header only", m.only(&[0]));
    invalid("inventory: repeated", m.with_line(2, m.line(1)));
    invalid("inventory: after an intent", m.only(&[0, 2, 1, 3, 4]));

    // A crash can cut short only the final line; the same bytes terminated
    // are a malformed line, and the opening lines are never optional.
    let torn_intent = &m.line(4)[..10];
    let torn_results = &m.line(3)[..10];
    invalid("intent: truncated but terminated", m.torn(&[0, 1, 2, 3], &[torn_intent, b"\n"].concat()));
    invalid("results: truncated but terminated", m.torn(&[0, 1, 2], &[torn_results, b"\n"].concat()));
    invalid("inventory: torn", m.torn(&[0], &m.line(1)[..10]));
    invalid("header: torn", m.line(0)[..10].to_vec());

    cells.push(("torn intent".into(), m.torn(&[0, 1, 2, 3], torn_intent), Valid, vec![("d/a", Deleted), ("d/b", Kept)]));
    cells.push(("torn results".into(), m.torn(&[0, 1, 2], torn_results), Valid, vec![("d/a", Uncertain), ("d/b", Planned)]));
    cells.push((
        "torn: a whole intent without its newline".into(),
        m.torn(&[0, 1, 2, 3], &m.line(4)),
        Valid,
        vec![("d/a", Deleted), ("d/b", Kept)],
    ));

    m.assert_cells(cells);

    // The refusal names the edited line: the header is line 1, the inventory
    // line 2, and the first intent line 3.
    assert_eq!(m.malformed_at(&m.with_line(0, set(&lines[0], "/header/journal_version", json!("3")))).0, 1);
    assert_eq!(m.malformed_at(&m.only(&[0])).0, 2);
    assert_eq!(m.malformed_at(&m.with_line(2, set(&lines[2], "/intent/entries", json!([])))).0, 3);
    assert_eq!(m.malformed_at(&m.with_line(2, set(&lines[2], "/intent/entries", json!([2, 5])))).0, 3);
    assert_eq!(m.malformed_at(&m.with_line(3, set(&lines[3], "/results/states/0", json!("vanished")))).0, 4);
    assert_eq!(m.malformed_at(&open(json!([7]))).0, 5);
    assert_eq!(m.malformed_at(&m.torn(&[0, 1, 2, 3], &[torn_intent, b"\n"].concat())).0, 5);
    // The bound itself refuses 257 valid positions, not a later check.
    let (line, reason) = m.malformed_at(&open(json!(vec![5; 257])));
    assert_eq!(line, 5);
    assert!(reason.contains("bound"), "{reason}");
    assert_eq!(m.refusal(&m.with_line(0, set(&lines[0], "/header/journal_version", json!(1)))), JournalInvalid::UnsupportedVersion(1));
    assert_eq!(m.refusal(&m.with_line(0, set(&lines[0], "/header/journal_version", json!(4)))), JournalInvalid::UnsupportedVersion(4));
    assert_eq!(m.refusal(&m.with_line(0, set(&lines[0], "/header/operation_id", json!(other_id)))), JournalInvalid::OperationMismatch);

    // Recovery only reads: a refused journal leaves the operation as it was.
    let dir = m.f.store.operation_dir(&id);
    let snapshot = || -> Vec<(std::ffi::OsString, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(&dir).unwrap().map(|entry| entry.unwrap().path()).map(|path| (path.file_name().unwrap().to_owned(), fs::read(&path).unwrap())).collect();
        files.sort();
        files
    };
    let refused = open(json!([7]));
    fs::write(&m.path, &refused).unwrap();
    let before = snapshot();
    assert!(m.recover(&refused).is_err());
    assert_eq!(snapshot(), before, "a refused recovery changed the operation directory");
    assert_eq!(fs::read(&m.path).unwrap(), refused);
}

/// Version 2 batch lines, from frozen bytes, each file read by its own
/// header.
#[test]
fn journal_robustness_matrix_version_2() {
    let m = JournalMatrix::version_2();
    let lines = &m.lines;

    use EntryState::*;
    use Expect::*;
    let mut cells: Vec<JournalCell> = vec![
        ("control".into(), m.all(), Valid, journal_control()),
        ("torn intent".into(), m.torn(&[0, 1, 2, 3], &m.line(4)[..10]), Valid, vec![("d/a", Deleted), ("d/b", Kept)]),
    ];
    let mut invalid = |cell: &str, bytes: Vec<u8>| cells.push((cell.to_string(), bytes, Invalid, Vec::new()));

    // (column, line, pointer, wrong type, empty, invalid content).
    let columns: Vec<(&str, usize, &str, Value, Value, Value)> = vec![
        ("path", 2, "/intent/entries/0/path", json!(123), json!({}), json!({"display": "../d/a"})),
        ("kind", 2, "/intent/entries/0/kind", json!(123), json!(""), json!("socket")),
        ("state", 3, "/results/entries/0/state", json!(123), json!(""), json!("vanished")),
    ];
    for (column, line, pointer, wrong_type, empty, bad) in columns {
        let key = pointer.rsplit('/').next().unwrap();
        invalid(&format!("{column}: absent"), m.with_line(line, remove(&lines[line], pointer)));
        invalid(&format!("{column}: null"), m.with_line(line, set(&lines[line], pointer, Value::Null)));
        invalid(&format!("{column}: wrong type"), m.with_line(line, set(&lines[line], pointer, wrong_type)));
        invalid(&format!("{column}: empty"), m.with_line(line, set(&lines[line], pointer, empty)));
        invalid(&format!("{column}: duplicate key"), m.with_line(line, duplicate(&lines[line], key)));
        invalid(&format!("{column}: invalid content"), m.with_line(line, set(&lines[line], pointer, bad)));
    }
    let entry = lines[2].pointer("/intent/entries/0").unwrap().clone();
    invalid("entries: one element wrong type", m.with_line(2, set(&lines[2], "/intent/entries", json!([entry, 123]))));
    invalid("entries: every element wrong type", m.with_line(2, set(&lines[2], "/intent/entries", json!([123]))));
    invalid("entries: empty batch", m.with_line(2, set(&lines[2], "/intent/entries", json!([]))));
    invalid("path: unknown member", m.with_line(2, set(&lines[2], "/intent/entries/0/path", json!({"display": "d/a", "extra": 1}))));
    invalid("intent entry: unknown member", m.with_line(2, set(&lines[2], "/intent/entries/0/extra", json!(1))));
    invalid("results entry: unknown member", m.with_line(3, set(&lines[3], "/results/entries/0/extra", json!(1))));
    invalid("intent: an entry the inventory does not list", m.with_line(2, set(&lines[2], "/intent/entries/0/path", json!({"display": "d/new"}))));
    invalid("intent: a kind other than the inventory's", m.with_line(2, set(&lines[2], "/intent/entries/0/kind", json!("directory"))));
    invalid("intent: duplicate within a batch", m.with_line(2, set(&lines[2], "/intent/entries/1", entry.clone())));
    invalid("intent: duplicate across batches", m.with_line(4, set(&lines[4], "/intent/entries/0/path", json!({"display": "d/b"}))));
    invalid("intent: version 3 shape", m.with_line(2, set(&lines[2], "/intent/entries", json!([2, 3]))));
    invalid("results: version 3 shape", m.with_line(3, br#"{"results":{"batch":1,"states":["deleted","kept"]}}"#.to_vec()));
    invalid("results: wrong batch", m.with_line(3, set(&lines[3], "/results/batch", json!(2))));
    let mut swapped = lines[3].clone();
    swapped.pointer_mut("/results/entries").unwrap().as_array_mut().unwrap().reverse();
    invalid("results: misaligned entries", m.with_line(3, compact(&swapped)));
    // The inventory line goes through the validator version 3 uses.
    invalid("inventory entries: wrong type", m.with_line(1, set(&lines[1], "/inventory/entries", json!(123))));

    m.assert_cells(cells);
    assert_eq!(m.malformed_at(&m.with_line(1, set(&lines[1], "/inventory/entries", json!(123)))).0, 2);
    assert_eq!(m.malformed_at(&m.with_line(4, set(&lines[4], "/intent/entries/0/path", json!({"display": "d/b"})))).0, 5);
}

/// The inventory line, which both versions read with the same code; its
/// version 2 wiring is checked in `journal_robustness_matrix_version_2`.
#[test]
fn journal_robustness_matrix_shared_inventory() {
    let m = JournalMatrix::version_3();
    let lines = &m.lines;

    use Expect::*;
    let mut cells: Vec<JournalCell> = vec![("control".into(), m.all(), Valid, journal_control())];
    // The same inventory with different but equivalent spacing.
    let spaced = m.with_line(1, serde_json::to_string(&lines[1]).unwrap().replace("\":", "\": ").into_bytes());
    cells.push(("inventory: spaced".into(), spaced, Valid, journal_control()));
    let mut invalid = |cell: &str, bytes: Vec<u8>| cells.push((cell.to_string(), bytes, Invalid, Vec::new()));

    // (column, pointer, wrong type, empty, invalid content). Every pointer
    // names the first entry (the hashed file "a") or the only link, so
    // `duplicate`'s first-occurrence edit lands on the same member.
    let columns: Vec<(&str, &str, Value, Value, Value)> = vec![
        ("inventory root", "/inventory/root", json!(123), json!({}), json!({"kind": "unix", "device": "x", "inode": "3"})),
        ("inventory entries", "/inventory/entries", json!(123), json!({}), json!([{"path": {"display": "a"}}])),
        ("inventory path", "/inventory/entries/0/path", json!(123), json!({}), json!({"display": "/a"})),
        ("inventory identity", "/inventory/entries/0/identity", json!(123), json!({}), json!({"kind": "plan9", "qid": "1"})),
        ("inventory shape", "/inventory/entries/0/shape", json!(123), json!({}), json!({"socket": {}})),
        ("file size", "/inventory/entries/0/shape/file/size", json!("5"), json!(""), json!(-1)),
        ("file modified", "/inventory/entries/0/shape/file/modified", json!(123), json!({}), json!({"secs": 1, "nanos": -1})),
        ("file mode", "/inventory/entries/0/shape/file/mode", json!("644"), json!(""), json!(-1)),
        ("file content", "/inventory/entries/0/shape/file/content", json!(123), json!(""), json!({"blake3": "zz"})),
        ("link target", "/inventory/entries/6/shape/link/target", json!(123), json!({}), json!({"display": "a\u{0}b"})),
    ];
    for (column, pointer, wrong_type, empty, bad) in columns {
        let key = pointer.rsplit('/').next().unwrap();
        invalid(&format!("{column}: absent"), m.with_line(1, remove(&lines[1], pointer)));
        invalid(&format!("{column}: null"), m.with_line(1, set(&lines[1], pointer, Value::Null)));
        invalid(&format!("{column}: wrong type"), m.with_line(1, set(&lines[1], pointer, wrong_type)));
        invalid(&format!("{column}: empty"), m.with_line(1, set(&lines[1], pointer, empty)));
        invalid(&format!("{column}: duplicate key"), m.with_line(1, duplicate(&lines[1], key)));
        invalid(&format!("{column}: invalid content"), m.with_line(1, set(&lines[1], pointer, bad)));
    }
    let listed = lines[1].pointer("/inventory/entries/0").unwrap().clone();
    invalid("inventory entries: one element wrong type", m.with_line(1, edited(&lines[1], "/inventory/entries", |parent, key| {
        parent[key].as_array_mut().unwrap().push(json!(123));
    })));
    invalid("inventory entries: every element wrong type", m.with_line(1, set(&lines[1], "/inventory/entries", json!([123]))));
    // An empty inventory is a valid line, but the batches then plan
    // positions it does not have.
    invalid("inventory entries: empty", m.with_line(1, set(&lines[1], "/inventory/entries", json!([]))));
    invalid("inventory: unknown member", m.with_line(1, set(&lines[1], "/inventory/extra", json!(1))));
    invalid("inventory entry: unknown member", m.with_line(1, set(&lines[1], "/inventory/entries/0/extra", json!(1))));
    invalid("inventory entries: a path twice", m.with_line(1, edited(&lines[1], "/inventory/entries", |parent, key| {
        parent[key].as_array_mut().unwrap().push(listed.clone());
    })));
    invalid("inventory entries: a child before its directory", m.with_line(1, edited(&lines[1], "/inventory/entries", |parent, key| {
        parent[key].as_array_mut().unwrap().swap(1, 2);
    })));
    invalid("inventory root: another checkout", m.with_line(1, set(&lines[1], "/inventory/root", json!({"kind": "unix", "device": "7", "inode": "4"}))));
    invalid("inventory: duplicate line key", m.with_line(1, duplicate(&lines[1], "inventory")));
    let mut garbage = m.line(1);
    garbage.extend_from_slice(b" junk");
    invalid("inventory line: trailing content", m.with_line(1, garbage));

    m.assert_cells(cells);
    assert_eq!(m.malformed_at(&m.with_line(1, set(&lines[1], "/inventory/entries", json!(123)))).0, 2);
}

#[test]
fn begin_refuses_an_inventory_of_another_checkout() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let lock = f.store.lock(&saved.repository).unwrap();
    let other = Arc::new(DeletionInventory::listing(identity(4), &["a"]));
    assert!(matches!(f.store.begin(lock, saved.clone(), other), Err(StoreError::InventoryMismatch)));
    assert!(f.store.pending().unwrap().is_empty(), "nothing was saved");
}

// ---- Replacing the journal on retry ----------------------------------------

/// An operation that deleted `a` and stopped, with its lock released: what a
/// retry finds.
fn stopped_after_deleting_a(f: &Fixture) -> (RecoveryRecord, Vec<u8>) {
    let saved = record(f, "wt1");
    let mut op = begin(f, saved.clone());
    op.journal().begin_batch(vec![file("a")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted]).unwrap();
    let journal = fs::read(op.dir().join(JOURNAL_FILE)).unwrap();
    drop(op);
    (saved, journal)
}

fn deleted_a() -> Vec<JournalEntry> {
    vec![JournalEntry { attempt: 1, batch: 1, path: NativeString::from("a"), kind: EntryKind::File, state: EntryState::Deleted }]
}

fn reopen(f: &Fixture, saved: &RecoveryRecord) -> Result<Operation, StoreError> {
    let lock = f.store.lock(&saved.repository).unwrap();
    f.store.reopen(lock, &saved.operation_id, &saved.repository, inventory())
}

/// Every shape `reopen` can leave when it stops part-way: the previous
/// attempt stays recoverable through the public API, its journal is
/// unchanged, and no operation (the only thing that journals a deletion)
/// is returned. A later retry then reopens normally.
#[test]
fn a_reopen_stopped_at_any_step_leaves_the_previous_attempt_recoverable() {
    type Write = fn(&[u8]) -> Vec<u8>;
    let header_only: Write = |journal| journal[..=journal.iter().position(|b| *b == b'\n').unwrap()].to_vec();
    let partial_inventory: Write = |journal| {
        let header = journal.iter().position(|b| *b == b'\n').unwrap() + 1;
        journal[..header + (journal.len() - header) / 4].to_vec()
    };
    let cases: [(&str, ReopenStep, Fault, Option<Write>); 7] = [
        ("after archiving", ReopenStep::Archived, Fault::Crash, None),
        ("empty replacement", ReopenStep::Created, Fault::Crash, None),
        ("header only", ReopenStep::Created, Fault::Crash, Some(header_only)),
        ("header and part of the inventory", ReopenStep::Created, Fault::Crash, Some(partial_inventory)),
        ("complete but unpublished", ReopenStep::Initialized, Fault::Crash, None),
        ("initialization write failure", ReopenStep::Created, Fault::Fail, Some(header_only)),
        ("publication failure", ReopenStep::Initialized, Fault::Fail, None),
    ];
    for (name, at, fault, write) in cases {
        let f = fixture();
        let (saved, previous) = stopped_after_deleting_a(&f);
        let dir = f.store.operation_dir(&saved.operation_id);
        // The replacement repeats the previous journal's header and
        // inventory: same operation, same inventory.
        let written = write.map(|write| write(&previous));
        let injected = fault::inject(move |step, path| {
            if step != at {
                return None;
            }
            if let Some(bytes) = &written {
                fs::write(path, bytes).unwrap();
            }
            Some(fault)
        });
        let outcome = reopen(&f, &saved);
        assert!(outcome.is_err(), "{name}: the stopped reopen returned an operation");
        drop(outcome);
        drop(injected);

        assert_eq!(fs::read(dir.join(JOURNAL_FILE)).unwrap(), previous, "{name}: the journal was replaced");
        assert_eq!(f.store.pending().unwrap().len(), 1, "{name}");
        let recovered = f.store.recover(&saved.operation_id, &saved.repository, |_| Ok(false)).unwrap();
        assert_eq!(recovered.record, saved, "{name}");
        assert_eq!(recovered.entries, deleted_a(), "{name}: the confirmed deletion is still recoverable");
        if fault == Fault::Fail {
            let hidden = fs::read_dir(&dir).unwrap().filter(|entry| entry.as_ref().unwrap().file_name().to_string_lossy().ends_with(".tmp"));
            assert_eq!(hidden.count(), 0, "{name}: a failed reopen removes its replacement");
        }

        // The retry after the stop archives the same journal once, not twice.
        drop(reopen(&f, &saved).unwrap());
        assert_eq!(fs::read(dir.join("journal-1.jsonl")).unwrap(), previous, "{name}");
        assert!(!dir.join("journal-2.jsonl").exists(), "{name}: the same journal was archived twice");
        let resumed = f.store.recover(&saved.operation_id, &saved.repository, |_| Ok(false)).unwrap();
        assert_eq!(resumed.entries, deleted_a(), "{name}: the new attempt starts its own journal and the earlier deletion is kept");
    }
}

/// The clean control: a reopen that runs to the end keeps the previous
/// journal as `journal-1.jsonl` and publishes a new one holding only the
/// fresh inventory, owner-only like every store file.
#[test]
fn a_completed_reopen_archives_the_previous_journal_and_publishes_a_new_one() {
    let f = fixture();
    let (saved, previous) = stopped_after_deleting_a(&f);
    let mut op = reopen(&f, &saved).unwrap();
    let dir = op.dir().to_path_buf();
    assert_eq!(fs::read(dir.join("journal-1.jsonl")).unwrap(), previous);
    #[cfg(unix)]
    assert_eq!(mode(&dir.join("journal-1.jsonl")), 0o600);
    op.journal().begin_batch(vec![file("d/a")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted]).unwrap();
    drop(op);
    let recovered = f.store.recover(&saved.operation_id, &saved.repository, |_| Ok(false)).unwrap();
    let deleted_d_a = JournalEntry { attempt: 2, batch: 1, path: NativeString::from("d/a"), kind: EntryKind::File, state: EntryState::Deleted };
    assert_eq!(recovered.entries, [deleted_a(), vec![deleted_d_a]].concat());
    let names: BTreeSet<String> = fs::read_dir(&dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, BTreeSet::from([RECORD_FILE.into(), JOURNAL_FILE.into(), "journal-1.jsonl".into()]));
}

/// A journal that cannot be used names the file that failed, so an archived
/// attempt's `journal-<n>.jsonl` is told apart from the current
/// `journal.jsonl`.
#[test]
fn an_unusable_journal_error_names_the_current_or_archived_file() {
    let f = fixture();
    let (saved, _) = stopped_after_deleting_a(&f);
    drop(reopen(&f, &saved).unwrap());
    let dir = f.store.operation_dir(&saved.operation_id);
    let recover = || f.store.recover(&saved.operation_id, &saved.repository, |_| Ok(false));
    let garbage = b"{\"header\":{}}\n";

    for name in ["journal-1.jsonl", JOURNAL_FILE] {
        let path = dir.join(name);
        let original = fs::read(&path).unwrap();
        fs::write(&path, garbage).unwrap();

        let error = recover().map(|_| ()).unwrap_err();

        match &error {
            StoreError::Journal { path: named, reason: JournalInvalid::Malformed { line: 1, .. } } => {
                assert_eq!(named, &path, "{error}");
            }
            other => panic!("{name}: expected a journal refusal, got {other:?}"),
        }
        assert!(error.to_string().contains(&path.display().to_string()), "{error}");
        fs::write(&path, original).unwrap();
    }
}

/// Every attempt's journal is replayed. An earlier attempt keeps what was
/// gone when the next one began: its deletions, and an unconfirmed entry the
/// next inventory no longer lists, as uncertain. What the next inventory
/// lists (kept, or unconfirmed but still present) is left to the attempt
/// that resolves it, so nothing is counted twice.
#[test]
fn earlier_attempts_keep_their_deletions_and_uncertainty_without_double_counting() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let mut op = begin(&f, saved.clone());
    op.journal().begin_batch(vec![file("e/x")]).unwrap();
    op.journal().finish_batch(&[ResultState::Kept]).unwrap();
    op.journal().begin_batch(vec![file("a")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted]).unwrap();
    // A crash between unlinking `d/a` and writing the batch's results.
    op.journal().begin_batch(vec![file("d/a"), file("d/b")]).unwrap();
    drop(op);

    let mut survivors = inventory_value();
    survivors["entries"].as_array_mut().unwrap().retain(|entry| !["a", "d/a"].contains(&entry["path"]["display"].as_str().unwrap()));
    let lock = f.store.lock(&saved.repository).unwrap();
    let survivors: DeletionInventory = serde_json::from_value(survivors).unwrap();
    let mut op = f.store.reopen(lock, &saved.operation_id, &saved.repository, Arc::new(survivors)).unwrap();
    op.journal().begin_batch(vec![file("d/b")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted]).unwrap();

    let entry = |attempt, batch, path: &str, state| JournalEntry { attempt, batch, path: NativeString::from(path), kind: EntryKind::File, state };
    let expected = vec![
        entry(1, 2, "a", EntryState::Deleted),
        entry(1, 3, "d/a", EntryState::Uncertain),
        entry(2, 1, "d/b", EntryState::Deleted),
    ];
    // The filesystem now says every path is gone; only the current attempt
    // asks it.
    assert_eq!(op.journal_entries(|_| Ok(false)).unwrap(), expected);
    assert_eq!(f.store.recover(&saved.operation_id, &saved.repository, |_| Ok(false)).unwrap().entries, expected);

    // Completion that could not list the entries keeps every journal.
    let dir = op.dir().to_path_buf();
    let report = OperationReport::new(Some(saved.operation_id.clone()), RemovalOutcome::Removed, saved.original.clone());
    assert_eq!(op.complete(&report, false).unwrap(), Vec::new());
    assert!(dir.join(JOURNAL_FILE).exists() && dir.join("journal-1.jsonl").exists());
}

/// A discard renames the directory away before deleting it, so a crash
/// part-way leaves a hidden directory that is no pending operation.
#[test]
fn a_discarded_operation_is_never_left_pending_without_its_journal() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let dir = begin(&f, saved.clone()).dir().to_path_buf();
    // What a crash inside the deletion of the renamed directory leaves.
    let doomed = f.store.root().join(format!(".{}.discarded", saved.operation_id));
    fs::rename(&dir, &doomed).unwrap();
    fs::remove_file(doomed.join(JOURNAL_FILE)).unwrap();
    assert!(f.store.pending().unwrap().is_empty());
    drop(begin(&f, record(&f, "wt1")));

    let other = record(&f, "wt2");
    let dir = begin(&f, other.clone()).dir().to_path_buf();
    reopen(&f, &other).unwrap().discard().unwrap();
    assert!(!dir.exists());
    assert!(!f.store.root().join(format!(".{}.discarded", other.operation_id)).exists());
}

// Retention of finished operations (`prune_completed`). Every case runs on a
// fixed clock and backdates folders with `filetime`; nothing sleeps.

const DAY_MS: u64 = 24 * 60 * 60 * 1000;
const NOW_MS: u64 = 1_900_000_000_000;

fn now() -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_millis(NOW_MS)
}

/// Sets `path`'s modification time to `NOW_MS - age_ms`.
fn backdate(path: &Path, age_ms: u64) {
    let at = std::time::UNIX_EPOCH + std::time::Duration::from_millis(NOW_MS - age_ms);
    filetime::set_file_mtime(path, filetime::FileTime::from_system_time(at)).unwrap();
}

/// A completed operation on worktree record `admin` whose saved report's
/// timeline holds `events` (Unix milliseconds), in order. Its folder's
/// modification time is set to `NOW_MS - dir_age_ms`.
fn completed(f: &Fixture, admin: &str, events: &[u64], dir_age_ms: u64) -> PathBuf {
    use crate::remove::outcome::{Step, StepResult, TimelineEvent};
    let saved = record(f, admin);
    let op = begin(f, saved.clone());
    let dir = op.dir().to_path_buf();
    let mut report = OperationReport::new(Some(saved.operation_id.clone()), RemovalOutcome::Removed, saved.original.clone());
    report.timeline = events
        .iter()
        .map(|&at_ms| TimelineEvent { at_ms, step: Step::Moved, result: StepResult::Succeeded, detail: None })
        .collect();
    assert_eq!(op.complete(&report, true).unwrap(), Vec::new());
    backdate(&dir, dir_age_ms);
    dir
}

/// A completed operation finished `age_ms` ago by its report, in a folder
/// last modified just now.
fn finished_ago(f: &Fixture, admin: &str, age_ms: u64) -> PathBuf {
    completed(f, admin, &[NOW_MS - age_ms], 0)
}

/// A staging folder `begin` never published, last modified `age_ms` ago.
fn staging(f: &Fixture, age_ms: u64) -> PathBuf {
    let dir = f.store.root().join(format!(".{}.tmp", OperationId::generate().unwrap()));
    storage::create_private_dir(&dir).unwrap();
    storage::write_durable(&dir.join(RECORD_FILE), b"{}").unwrap();
    backdate(&dir, age_ms);
    dir
}

/// Names in the store root, sorted.
fn store_names(f: &Fixture) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(f.store.root()).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

fn name_of(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// The success criterion's store: of a completed operation finished 15 days
/// ago, one finished 13 days ago, one pending for 30 days, a 15-day-old
/// staging folder, and an expired completed one whose lock is held, only the
/// first and the staging folder go.
#[test]
fn retention_deletes_only_expired_unlocked_completed_operations_and_old_staging() {
    let f = fixture();
    let expired = finished_ago(&f, "wt1", 15 * DAY_MS);
    let recent = finished_ago(&f, "wt2", 13 * DAY_MS);
    let pending_record = record(&f, "wt3");
    let pending = begin(&f, pending_record.clone()).dir().to_path_buf();
    backdate(&pending, 30 * DAY_MS);
    let stale_staging = staging(&f, 15 * DAY_MS);
    let locked = finished_ago(&f, "wt4", 15 * DAY_MS);
    let lock = f.store.lock(&binding(&f, "wt4")).unwrap();

    f.store.prune_completed(now());

    let mut kept = vec![name_of(&recent), name_of(&pending), name_of(&locked), LOCKS_DIR.to_string()];
    kept.sort();
    assert_eq!(store_names(&f), kept, "only {} and {} go", name_of(&expired), name_of(&stale_staging));
    assert!(f.store.read_record(&pending_record.operation_id).is_ok(), "the pending record is untouched");
    assert_eq!(f.store.pending().unwrap().len(), 1);

    drop(lock);
    f.store.prune_completed(now());
    assert!(!locked.exists(), "the lock was the only reason it stayed");
    assert!(recent.exists() && pending.exists());
}

#[test]
fn retention_keeps_exactly_fourteen_days_and_future_completions() {
    let f = fixture();
    let boundary = finished_ago(&f, "wt1", 14 * DAY_MS);
    let past_boundary = finished_ago(&f, "wt2", 14 * DAY_MS + 1);
    let future = completed(&f, "wt3", &[NOW_MS + DAY_MS], 30 * DAY_MS);

    f.store.prune_completed(now());

    assert!(boundary.exists(), "exactly 14 days is kept");
    assert!(!past_boundary.exists(), "one millisecond more is not");
    assert!(future.exists(), "a completion after now is kept, whatever the folder's age");
}

/// The last timeline event dates completion: not the latest event, and not
/// the folder's modification time.
#[test]
fn retention_dates_completion_by_the_last_timeline_event() {
    let f = fixture();
    let last_is_old = completed(&f, "wt1", &[NOW_MS - 13 * DAY_MS, NOW_MS - 15 * DAY_MS], 0);
    let last_is_recent = completed(&f, "wt2", &[NOW_MS - 30 * DAY_MS, NOW_MS - 13 * DAY_MS], 30 * DAY_MS);

    f.store.prune_completed(now());

    assert!(!last_is_old.exists(), "the last event is 15 days old though an earlier one is 13 and the folder is new");
    assert!(last_is_recent.exists(), "the last event is 13 days old though the folder is 30");
}

/// A report that is missing, malformed, or has no timeline events falls back
/// to the folder's modification time.
#[test]
fn retention_falls_back_to_the_folder_time_when_the_report_cannot_date_completion() {
    let f = fixture();
    let mut cases = Vec::new();
    for (n, edit) in ["missing", "malformed", "empty timeline", "not an object"].into_iter().enumerate() {
        for (age, expect_kept) in [(15 * DAY_MS, false), (13 * DAY_MS, true)] {
            let admin = format!("wt-{n}-{}", age / DAY_MS);
            let events: &[u64] = match edit {
                "empty timeline" => &[],
                _ => &[NOW_MS],
            };
            let dir = completed(&f, &admin, events, 0);
            let report = dir.join(REPORT_FILE);
            match edit {
                "missing" => fs::remove_file(&report).unwrap(),
                "malformed" => storage::write_durable(&report, b"{\"timeline\": [").unwrap(),
                "not an object" => storage::write_durable(&report, b"[1, 2]").unwrap(),
                _ => {}
            }
            backdate(&dir, age);
            cases.push((edit, age / DAY_MS, dir, expect_kept));
        }
    }

    f.store.prune_completed(now());

    for (edit, days, dir, expect_kept) in cases {
        assert_eq!(dir.exists(), expect_kept, "{edit}, folder {days} days old");
    }
}

/// Only a readable record marked completed lets an operation go: a pending
/// one and one whose record cannot be read are kept however old.
#[test]
fn retention_never_expires_a_pending_or_unreadable_record() {
    let f = fixture();
    let pending = begin(&f, record(&f, "wt1")).dir().to_path_buf();
    backdate(&pending, 365 * DAY_MS);
    let foreign = finished_ago(&f, "wt3", 30 * DAY_MS);
    let unreadable = finished_ago(&f, "wt2", 30 * DAY_MS);
    // Corrupted only now: `begin` refuses while any record is unreadable.
    let mut other_id = f.store.read_record(&OperationId::parse(&name_of(&foreign)).unwrap()).unwrap();
    other_id.operation_id = OperationId::generate().unwrap();
    storage::write_durable(&foreign.join(RECORD_FILE), &other_id.to_json()).unwrap();
    storage::write_durable(&unreadable.join(RECORD_FILE), b"{\"record_version\": 1").unwrap();
    backdate(&unreadable, 30 * DAY_MS);
    backdate(&foreign, 30 * DAY_MS);

    f.store.prune_completed(now());

    assert!(pending.exists());
    assert!(unreadable.exists(), "an unreadable record could be pending");
    assert!(foreign.exists(), "a record naming another operation is not this folder's");
}

/// Young staging folders, the locks, discarded folders, other names, files,
/// and a staging name that is a file are all left alone.
#[test]
fn retention_leaves_everything_that_is_not_an_operation_or_old_staging() {
    let f = fixture();
    let young = staging(&f, 13 * DAY_MS);
    let id = || OperationId::generate().unwrap();
    let others = [
        f.store.root().join("notes"),
        f.store.root().join(format!(".{}.discarded", id())),
        f.store.root().join(format!(".{}.tmp.extra", id())),
        f.store.root().join(".not-an-id.tmp"),
    ];
    for dir in &others {
        storage::create_private_dir(dir).unwrap();
        backdate(dir, 30 * DAY_MS);
    }
    let staging_file = f.store.root().join(format!(".{}.tmp", id()));
    storage::write_durable(&staging_file, b"x").unwrap();
    backdate(&staging_file, 30 * DAY_MS);
    let loose = f.store.root().join("loose.json");
    storage::write_durable(&loose, b"x").unwrap();
    drop(f.store.lock(&binding(&f, "wt1")).unwrap());
    let before = store_names(&f);

    f.store.prune_completed(now());

    assert_eq!(store_names(&f), before);
    assert!(young.exists() && f.store.root().join(LOCKS_DIR).join(format!("{}.lock", lock::key_for("wt1".as_ref()))).exists());
}

/// A link named as an operation or as staging is never followed: its target
/// survives even when it is an expired completed operation.
#[test]
fn retention_never_follows_a_link() {
    let f = fixture();
    let expired = finished_ago(&f, "wt1", 30 * DAY_MS);
    let moved = f.common.join("elsewhere");
    fs::rename(&expired, &moved).unwrap();
    let old_staging = staging(&f, 30 * DAY_MS);
    let staging_target = f.common.join("staging-elsewhere");
    fs::rename(&old_staging, &staging_target).unwrap();
    if symlink_dir(&moved, &expired).is_err() || symlink_dir(&staging_target, &old_staging).is_err() {
        // Windows without symlink privilege cannot build this fixture.
        return;
    }

    f.store.prune_completed(now());

    assert!(fs::symlink_metadata(&expired).is_ok() && fs::symlink_metadata(&old_staging).is_ok(), "the links stay");
    assert!(moved.join(RECORD_FILE).is_file() && staging_target.join(RECORD_FILE).is_file(), "their targets are untouched");
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

/// A deletion that fails is skipped without stopping the others. The folder
/// it left is no longer named as an operation, so it never reads as an
/// unreadable pending record, and the next call finishes it.
#[cfg(unix)]
#[test]
fn a_failed_deletion_is_skipped_and_finished_on_the_next_run() {
    use std::os::unix::fs::PermissionsExt;
    // SAFETY: `geteuid` has no preconditions.
    if unsafe { libc::geteuid() } == 0 {
        // Root ignores the permission that makes the deletion fail.
        return;
    }
    let f = fixture();
    let stuck = finished_ago(&f, "wt1", 30 * DAY_MS);
    let protected = stuck.join("protected");
    fs::create_dir(&protected).unwrap();
    fs::write(protected.join("f"), b"x").unwrap();
    fs::set_permissions(&protected, fs::Permissions::from_mode(0o500)).unwrap();
    let other = finished_ago(&f, "wt2", 30 * DAY_MS);
    let stuck_id = name_of(&stuck);

    f.store.prune_completed(now());

    assert!(!other.exists(), "the other expired operation still goes");
    assert!(!stuck.exists());
    let leftover = f.store.root().join(format!(".{stuck_id}.expired"));
    assert!(leftover.is_dir(), "the cut-short deletion is left hidden: {:?}", store_names(&f));
    assert!(f.store.pending().unwrap().is_empty(), "and never reads as pending");
    drop(begin(&f, record(&f, "wt3")));

    fs::set_permissions(leftover.join("protected"), fs::Permissions::from_mode(0o700)).unwrap();
    f.store.prune_completed(now());
    assert!(!leftover.exists());
}

mod saved_report;

mod journal_size;
