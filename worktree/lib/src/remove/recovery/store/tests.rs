use std::collections::BTreeSet;

use serde_json::{Value, json};
use sniff::filesystem::query::{FileIdentity, NativeString, PathUsageReport};
use tempfile::TempDir;

use super::*;
use crate::remove::observation::inventory::DeletionInventory;
use crate::remove::outcome::RemovalOutcome;
use crate::remove::recovery::journal::{EntryKind, EntryState, PlannedEntry, ResultState};
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

    assert_eq!(op.complete(&report).unwrap(), Vec::new());
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
    let warnings = op.complete(&report).unwrap();
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

#[test]
fn journal_robustness_matrix() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let id = saved.operation_id.clone();
    let mut op = begin(&f, saved.clone());
    op.journal().begin_batch(vec![file("d/a"), file("d/b")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted, ResultState::Kept]).unwrap();
    op.journal().begin_batch(vec![file("e/x")]).unwrap();
    drop(op);
    let path = f.store.operation_dir(&id).join(JOURNAL_FILE);
    let written = fs::read(&path).unwrap();
    let lines: Vec<Value> =
        written.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()).map(|line| serde_json::from_slice(line).unwrap()).collect();
    assert_eq!(lines.len(), 5, "header, inventory, intent, results, open intent");
    assert_eq!(lines[1]["inventory"], inventory_value(), "the inventory is persisted as given");

    let join = |lines: &[Vec<u8>]| -> Vec<u8> { lines.iter().flat_map(|line| line.iter().copied().chain(*b"\n")).collect() };
    let compact = |line: &Value| serde_json::to_vec(line).unwrap();
    let with_line = |index: usize, replacement: Vec<u8>| -> Vec<u8> {
        let mut all: Vec<Vec<u8>> = lines.iter().map(compact).collect();
        all[index] = replacement;
        join(&all)
    };
    let other_id = "ffffffffffffffffffffffffffffffff";

    use EntryState::*;
    use Expect::*;
    let control = vec![("d/a", Deleted), ("d/b", Kept), ("e/x", Uncertain)];
    let mut cells: Vec<JournalCell> = vec![("control".into(), written.clone(), Valid, control.clone())];
    let mut invalid = |cell: &str, bytes: Vec<u8>| cells.push((cell.to_string(), bytes, Invalid, Vec::new()));

    // (column, line, pointer, wrong type, empty, invalid content). Every
    // inventory pointer names the first entry (the hashed file "a") or the
    // only link, so `duplicate`'s first-occurrence edit lands on the same
    // member.
    let columns: Vec<(&str, usize, &str, Value, Value, Value)> = vec![
        ("path", 2, "/intent/entries/0/path", json!(123), json!({}), json!({"display": "../d/a"})),
        ("kind", 2, "/intent/entries/0/kind", json!(123), json!(""), json!("socket")),
        ("state", 3, "/results/entries/0/state", json!(123), json!(""), json!("vanished")),
        ("inventory root", 1, "/inventory/root", json!(123), json!({}), json!({"kind": "unix", "device": "x", "inode": "3"})),
        ("inventory entries", 1, "/inventory/entries", json!(123), json!({}), json!([{"path": {"display": "a"}}])),
        ("inventory path", 1, "/inventory/entries/0/path", json!(123), json!({}), json!({"display": "/a"})),
        ("inventory identity", 1, "/inventory/entries/0/identity", json!(123), json!({}), json!({"kind": "plan9", "qid": "1"})),
        ("inventory shape", 1, "/inventory/entries/0/shape", json!(123), json!({}), json!({"socket": {}})),
        ("file size", 1, "/inventory/entries/0/shape/file/size", json!("5"), json!(""), json!(-1)),
        ("file modified", 1, "/inventory/entries/0/shape/file/modified", json!(123), json!({}), json!({"secs": 1, "nanos": -1})),
        ("file mode", 1, "/inventory/entries/0/shape/file/mode", json!("644"), json!(""), json!(-1)),
        ("file content", 1, "/inventory/entries/0/shape/file/content", json!(123), json!(""), json!({"blake3": "zz"})),
        ("link target", 1, "/inventory/entries/6/shape/link/target", json!(123), json!({}), json!({"display": "a\u{0}b"})),
    ];
    for (column, line, pointer, wrong_type, empty, bad) in columns {
        let key = pointer.rsplit('/').next().unwrap();
        invalid(&format!("{column}: absent"), with_line(line, remove(&lines[line], pointer)));
        invalid(&format!("{column}: null"), with_line(line, set(&lines[line], pointer, Value::Null)));
        invalid(&format!("{column}: wrong type"), with_line(line, set(&lines[line], pointer, wrong_type)));
        invalid(&format!("{column}: empty"), with_line(line, set(&lines[line], pointer, empty)));
        invalid(&format!("{column}: duplicate key"), with_line(line, duplicate(&lines[line], key)));
        invalid(&format!("{column}: invalid content"), with_line(line, set(&lines[line], pointer, bad)));
    }
    let entry = lines[2].pointer("/intent/entries/0").unwrap().clone();
    invalid("entries: one element wrong type", with_line(2, set(&lines[2], "/intent/entries", json!([entry, 123]))));
    invalid("entries: every element wrong type", with_line(2, set(&lines[2], "/intent/entries", json!([123]))));
    invalid("entries: empty batch", with_line(2, set(&lines[2], "/intent/entries", json!([]))));
    invalid("path: unknown member", with_line(2, set(&lines[2], "/intent/entries/0/path", json!({"display": "d/a", "extra": 1}))));
    let listed = lines[1].pointer("/inventory/entries/0").unwrap().clone();
    invalid("inventory entries: one element wrong type", with_line(1, edited(&lines[1], "/inventory/entries", |parent, key| {
        parent[key].as_array_mut().unwrap().push(json!(123));
    })));
    invalid("inventory entries: every element wrong type", with_line(1, set(&lines[1], "/inventory/entries", json!([123]))));
    // An empty inventory is a valid line, but the batches then plan entries it
    // does not list.
    invalid("inventory entries: empty", with_line(1, set(&lines[1], "/inventory/entries", json!([]))));
    invalid("inventory: unknown member", with_line(1, set(&lines[1], "/inventory/extra", json!(1))));
    invalid("inventory entry: unknown member", with_line(1, set(&lines[1], "/inventory/entries/0/extra", json!(1))));
    invalid("inventory entries: a path twice", with_line(1, edited(&lines[1], "/inventory/entries", |parent, key| {
        parent[key].as_array_mut().unwrap().push(listed.clone());
    })));
    invalid("inventory entries: a child before its directory", with_line(1, edited(&lines[1], "/inventory/entries", |parent, key| {
        parent[key].as_array_mut().unwrap().swap(1, 2);
    })));
    invalid("inventory root: another checkout", with_line(1, set(&lines[1], "/inventory/root", json!({"kind": "unix", "device": "7", "inode": "4"}))));
    invalid("intent: an entry the inventory does not list", with_line(2, set(&lines[2], "/intent/entries/0/path", json!({"display": "d/new"}))));
    invalid("intent: a kind other than the inventory's", with_line(2, set(&lines[2], "/intent/entries/0/kind", json!("directory"))));
    let mut garbage = compact(&lines[2]);
    garbage.extend_from_slice(b" junk");
    invalid("line: trailing content", with_line(2, garbage));
    let mut garbage = compact(&lines[1]);
    garbage.extend_from_slice(b" junk");
    invalid("inventory line: trailing content", with_line(1, garbage));
    invalid("line: blank line", with_line(2, Vec::new()));
    invalid("header: absent", join(&lines[1..].iter().map(compact).collect::<Vec<_>>()));
    invalid("header: repeated", with_line(2, compact(&lines[0])));
    invalid("header: other version", with_line(0, set(&lines[0], "/header/journal_version", json!(1))));
    invalid("header: other operation", with_line(0, set(&lines[0], "/header/operation_id", json!(other_id))));
    let without_inventory: Vec<Vec<u8>> = lines.iter().enumerate().filter(|(at, _)| *at != 1).map(|(_, line)| compact(line)).collect();
    invalid("inventory: absent", join(&without_inventory));
    invalid("inventory: header only", join(&[compact(&lines[0])]));
    invalid("inventory: repeated", with_line(2, compact(&lines[1])));
    let mut after_intent: Vec<Vec<u8>> = lines.iter().map(compact).collect();
    after_intent.swap(1, 2);
    invalid("inventory: after an intent", join(&after_intent));
    invalid("results: wrong batch", with_line(3, set(&lines[3], "/results/batch", json!(2))));
    let mut swapped = lines[3].clone();
    swapped.pointer_mut("/results/entries").unwrap().as_array_mut().unwrap().reverse();
    invalid("results: misaligned entries", with_line(3, compact(&swapped)));

    let mut torn_inventory = join(&lines[..1].iter().map(compact).collect::<Vec<_>>());
    torn_inventory.extend_from_slice(&compact(&lines[1])[..10]);
    invalid("inventory: torn", torn_inventory);

    // Valid: no batches, the two places a crash can tear a line, and the
    // unchanged inventory written with different but equivalent spacing.
    cells.push(("empty: header and inventory only".into(), join(&lines[..2].iter().map(compact).collect::<Vec<_>>()), Valid, Vec::new()));
    let mut torn_intent = join(&lines[..4].iter().map(compact).collect::<Vec<_>>());
    torn_intent.extend_from_slice(&compact(&lines[4])[..10]);
    cells.push(("torn intent".into(), torn_intent, Valid, vec![("d/a", Deleted), ("d/b", Kept)]));
    let mut torn_results = join(&lines[..3].iter().map(compact).collect::<Vec<_>>());
    torn_results.extend_from_slice(&compact(&lines[3])[..10]);
    cells.push(("torn results".into(), torn_results, Valid, vec![("d/a", Uncertain), ("d/b", Planned)]));
    let pretty_spaces = with_line(1, serde_json::to_string(&lines[1]).unwrap().replace("\":", "\": ").into_bytes());
    cells.push(("inventory: spaced".into(), pretty_spaces, Valid, control));

    let present = BTreeSet::from(["d/b"]);
    let mut failures = Vec::new();
    for (cell, bytes, expect, states) in cells {
        fs::write(&path, &bytes).unwrap();
        let outcome = f.store.recover(&id, &saved.repository, |path| Ok(present.contains(path.to_str().unwrap())));
        let ok = match (&outcome, expect) {
            (Ok(recovered), Valid) => {
                let got: Vec<_> = recovered.entries.iter().map(|entry| (entry.path.display().into_owned(), entry.state)).collect();
                let want: Vec<_> = states.iter().map(|(path, state)| (path.to_string(), *state)).collect();
                got == want && recovered.torn_tail == cell.starts_with("torn") && recovered.inventory == *inventory()
            }
            (Err(StoreError::Journal { .. } | StoreError::InventoryMismatch), Invalid) => true,
            _ => false,
        };
        if !ok {
            failures.push(format!("{cell}: expected {expect:?}, got {:?}", outcome.map(|r| (r.entries, r.torn_tail))));
        }
    }
    assert!(failures.is_empty(), "matrix cells failed:\n{}", failures.join("\n"));
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
