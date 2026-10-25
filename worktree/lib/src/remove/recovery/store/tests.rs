//! The legacy (`record_version` 1) record loader, through
//! [`RecoveryStore::resume`]: the only reader of such a record now migrates
//! it, so every shape the loader decides is asserted at that public result.

use serde_json::{Value, json};
use sniff::filesystem::query::{FileIdentity, PathUsageReport};
use tempfile::TempDir;

use super::*;
use crate::remove::recovery::record::{BranchBinding, IntendedAction, Phase, RECORD_VERSION, RecoveryRecord};

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

/// Writes `record` as an earlier `wt` left it, owner-only, and returns its
/// `record.json`.
fn save_legacy(f: &Fixture, record: &RecoveryRecord) -> PathBuf {
    let dir = f.store.operation_dir(&record.operation_id);
    storage::create_private_dir(&dir).unwrap();
    let path = dir.join(RECORD_FILE);
    storage::write_durable(&path, &record.to_json()).unwrap();
    path
}

fn resume(f: &Fixture, id: &OperationId, expected: &RepositoryBinding) -> Result<PendingRemoval, StoreError> {
    let lock = f.store.lock(expected).unwrap();
    f.store.resume(lock, id, expected)
}

#[test]
fn a_linked_operation_directory_is_never_read() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let dir = save_legacy(&f, &saved).parent().unwrap().to_path_buf();
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
    let operations = f.store.operations().unwrap();
    assert!(matches!(operations[0].record, Err(StoreError::Storage { problem: StorageProblem::Link, .. })));
    assert!(matches!(
        resume(&f, &saved.operation_id, &saved.repository),
        Err(StoreError::Storage { problem: StorageProblem::Link, .. })
    ));
    assert!(elsewhere.join(RECORD_FILE).is_file(), "nothing behind the link was touched");
}

// ---------------------------------------------------------------------------
// Input robustness matrix. It starts from a record an earlier `wt` wrote,
// applies one edit per cell, and asserts through `resume`.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    Valid,
    Invalid,
}

/// A record field: its name, pointer, wrong-type value, empty value with its
/// expected outcome (when the shape applies), and an invalid value.
type RecordField<'a> = (&'a str, &'a str, Value, Option<(Value, Expect)>, Value);

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
    let path = save_legacy(&f, &saved);
    let written = fs::read(&path).unwrap();
    let doc: Value = serde_json::from_slice(&written).unwrap();
    let other_id = "ffffffffffffffffffffffffffffffff";

    use Expect::*;
    // (field, pointer, wrong type, empty, invalid content)
    let fields: Vec<RecordField> = vec![
        ("record_version", "/record_version", json!("1"), None, json!(99)),
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
        let outcome = resume(&f, &id, &saved.repository);
        let ok = match (&outcome, expect) {
            (Ok(resumed), Valid) => resumed.record().operation_id == id,
            (Err(StoreError::Record { .. }), Invalid) => true,
            _ => false,
        };
        if !ok {
            failures.push(format!("{cell}: expected {expect:?}, got {:?}", outcome.map(|r| r.id().clone())));
        }
    }
    assert!(failures.is_empty(), "matrix cells failed:\n{}", failures.join("\n"));

    // The reasons a user is shown for the three semantic refusals.
    let reason = |bytes: Vec<u8>| {
        fs::write(&path, &bytes).unwrap();
        match resume(&f, &id, &saved.repository) {
            Err(StoreError::Record { reason, .. }) => reason,
            other => panic!("expected a record refusal, got {:?}", other.map(|r| r.id().clone())),
        }
    };
    assert_eq!(reason(set(&doc, "/record_version", json!(99))), RecordInvalid::UnsupportedVersion(99));
    assert_eq!(reason(set(&doc, "/operation_id", json!(other_id))), RecordInvalid::OperationMismatch);
    assert_eq!(
        reason(set(&doc, "/repository/common_dir_identity", json!({"kind": "unix", "device": "7", "inode": "999"}))),
        RecordInvalid::IdentityMismatch
    );
}
