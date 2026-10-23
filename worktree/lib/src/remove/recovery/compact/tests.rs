use std::path::PathBuf;

use serde_json::{Value, json};
use sniff::filesystem::query::FileIdentity;

use super::*;

/// An absolute directory on every OS; nothing here touches the filesystem.
fn root() -> PathBuf {
    std::env::temp_dir().join("wt-compact-record")
}

fn identity(inode: u64) -> FileIdentity {
    FileIdentity::Unix { device: 7, inode }
}

/// A moved checkout with one step of each kind, past its registration.
pub(crate) fn sample() -> CompactRecord {
    let root = root();
    let common = root.join("repo.git");
    CompactRecord {
        record_version: COMPACT_RECORD_VERSION,
        operation_id: OperationId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        created_at_ms: 1,
        requested_name: "fix/path".into(),
        aliases: vec!["fix-path".into()],
        original: root.join("wt1").as_path().into(),
        deletion_path: root.join(".wt-remove-wt1-a91c").as_path().into(),
        deletion_mode: DeletionMode::Moved,
        checkout_identity: identity(3),
        repository: RepositoryBinding {
            common_dir: common.as_path().into(),
            common_dir_identity: identity(1),
            admin_dir: common.join("worktrees").join("wt1").as_path().into(),
            admin_dir_identity: identity(103),
        },
        unlock_needed: true,
        branch_steps: vec![
            BranchStep {
                action: BranchAction::Local { name: "fix/path".into(), expected_tip: "a".repeat(40) },
                approval: StepApproval::Conditional,
                state: StepState::Done,
            },
            BranchStep {
                action: BranchAction::Remote {
                    remote: "origin".into(),
                    destination: "fix/path".into(),
                    endpoint: "https://example.invalid/repo.git".into(),
                    expected_head: "b".repeat(40),
                },
                approval: StepApproval::Explicit,
                state: StepState::Pending,
            },
        ],
        progress: Progress::Unregistered,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Valid,
    Invalid,
}

fn set(doc: &Value, pointer: &str, replacement: Value) -> Vec<u8> {
    let mut value = doc.clone();
    *value.pointer_mut(pointer).unwrap_or_else(|| panic!("{pointer} is in the fixture")) = replacement;
    serde_json::to_vec(&value).unwrap()
}

fn remove(doc: &Value, pointer: &str) -> Vec<u8> {
    let mut value = doc.clone();
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let parent = if parent.is_empty() { &mut value } else { value.pointer_mut(parent).unwrap() };
    parent.as_object_mut().unwrap().remove(key).unwrap();
    serde_json::to_vec(&value).unwrap()
}

/// Repeats `key`'s first member, with the same value, in compact JSON.
fn duplicate(doc: &Value, key: &str) -> Vec<u8> {
    let text = serde_json::to_string(doc).unwrap();
    let needle = format!("\"{key}\":");
    let start = text.find(&needle).unwrap_or_else(|| panic!("{key} is in the fixture"));
    let value_start = start + needle.len();
    let mut stream = serde_json::Deserializer::from_str(&text[value_start..]).into_iter::<Value>();
    stream.next().unwrap().unwrap();
    let value = &text[value_start..value_start + stream.byte_offset()];
    format!("{}{needle}{value},{}", &text[..start], &text[start..]).into_bytes()
}

#[test]
fn the_control_record_round_trips_through_its_own_bytes_twice() {
    let record = sample();
    let first = parse(&record.to_json()).unwrap();
    assert_eq!(first, record);
    assert_eq!(parse(&first.to_json()).unwrap(), record);
}

/// (field, pointer, wrong type, empty and its outcome, invalid content)
type FieldCells<'a> = (&'a str, &'a str, Value, Option<(Value, Expect)>, Value);

#[test]
fn compact_record_robustness_matrix() {
    let doc: Value = serde_json::from_slice(&sample().to_json()).unwrap();
    let elsewhere = root().join("elsewhere").join(".wt-remove-wt1-a91c");
    use Expect::*;
    let fields: Vec<FieldCells> = vec![
        ("record_version", "/record_version", json!("2"), None, json!(1)),
        ("operation_id", "/operation_id", json!(123), Some((json!(""), Invalid)), json!("../../etc")),
        ("created_at_ms", "/created_at_ms", json!("1"), None, json!(-1)),
        ("requested_name", "/requested_name", json!(123), Some((json!(""), Invalid)), json!(["fix/path"])),
        ("aliases", "/aliases", json!("fix-path"), Some((json!([]), Valid)), json!([""])),
        ("original", "/original", json!(123), Some((json!({}), Invalid)), json!({"display": "relative/wt1"})),
        ("deletion_path", "/deletion_path", json!(123), Some((json!({}), Invalid)), json!({"display": elsewhere})),
        ("deletion_mode", "/deletion_mode", json!(0), Some((json!(""), Invalid)), json!("in_place")),
        ("checkout_identity", "/checkout_identity", json!(123), Some((json!({}), Invalid)), json!({"kind": "unix", "device": "7"})),
        ("common_dir", "/repository/common_dir", json!(123), Some((json!({}), Invalid)), json!({"display": "repo.git"})),
        (
            "common_dir_identity",
            "/repository/common_dir_identity",
            json!(123),
            Some((json!({}), Invalid)),
            json!({"kind": 0, "device": "7", "inode": "1"}),
        ),
        ("admin_dir", "/repository/admin_dir", json!(123), Some((json!({}), Invalid)), json!({"display": root().join("wt1")})),
        ("admin_dir_identity", "/repository/admin_dir_identity", json!(123), Some((json!({}), Invalid)), json!("103")),
        ("unlock_needed", "/unlock_needed", json!("true"), None, json!(1)),
        ("branch_steps", "/branch_steps", json!("local"), Some((json!([]), Valid)), json!([{}])),
        ("progress", "/progress", json!(4), Some((json!(""), Invalid)), json!("completed")),
        ("step action", "/branch_steps/0/action", json!("local"), Some((json!({}), Invalid)), json!({"teleport": {}})),
        ("step approval", "/branch_steps/0/approval", json!(true), Some((json!(""), Invalid)), json!("implicit")),
        ("step state", "/branch_steps/0/state", json!(1), Some((json!(""), Invalid)), json!("deleted")),
        ("local name", "/branch_steps/0/action/local/name", json!(1), Some((json!(""), Invalid)), json!(["fix/path"])),
        ("expected_tip", "/branch_steps/0/action/local/expected_tip", json!(1), Some((json!(""), Invalid)), json!("A".repeat(40))),
        ("remote", "/branch_steps/1/action/remote/remote", json!(1), Some((json!(""), Invalid)), json!(["origin"])),
        ("destination", "/branch_steps/1/action/remote/destination", json!(1), Some((json!(""), Invalid)), json!({})),
        ("endpoint", "/branch_steps/1/action/remote/endpoint", json!(1), Some((json!(""), Invalid)), json!([])),
        ("expected_head", "/branch_steps/1/action/remote/expected_head", json!(1), Some((json!(""), Invalid)), json!("c".repeat(39))),
    ];

    let mut cells: Vec<(String, Vec<u8>, Expect)> = vec![("control".into(), serde_json::to_vec(&doc).unwrap(), Valid)];
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
    let step = doc.pointer("/branch_steps/0").unwrap().clone();
    for (field, pointer, valid) in [("aliases", "/aliases", json!("fix-path")), ("branch_steps", "/branch_steps", step)] {
        cells.push((format!("{field}: one element wrong type"), set(&doc, pointer, json!([valid, 123])), Invalid));
        cells.push((format!("{field}: every element wrong type"), set(&doc, pointer, json!([123])), Invalid));
    }
    let text = serde_json::to_string(&doc).unwrap();
    let inner = text.replacen("\"checkout_identity\":{", "\"checkout_identity\":{\"inode\":\"3\",", 1);
    cells.push(("checkout_identity: duplicate inner key".into(), inner.into_bytes(), Invalid));
    cells.push(("document: trailing content".into(), format!("{text}\n{{}}").into_bytes(), Invalid));
    cells.push(("document: unknown field".into(), set_new(&doc, "surprise"), Invalid));
    cells.push(("document: not JSON".into(), b"record".to_vec(), Invalid));
    cells.push(("document: an array".into(), b"[]".to_vec(), Invalid));

    let failures: Vec<String> = cells
        .into_iter()
        .filter_map(|(cell, bytes, expect)| {
            let outcome = parse(&bytes);
            let ok = matches!((&outcome, expect), (Ok(_), Valid) | (Err(_), Invalid));
            (!ok).then(|| format!("{cell}: expected {expect:?}, got {outcome:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "matrix cells failed:\n{}", failures.join("\n"));
}

fn set_new(doc: &Value, key: &str) -> Vec<u8> {
    let mut value = doc.clone();
    value.as_object_mut().unwrap().insert(key.into(), json!(true));
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn an_unknown_version_is_refused_before_anything_else_is_read() {
    for version in [0, 1, 3, u64::MAX] {
        let bytes = format!(r#"{{"record_version": {version}, "operation_id": 5}}"#);
        assert_eq!(parse(bytes.as_bytes()), Err(RecordInvalid::UnsupportedVersion(version)), "{version}");
    }
    assert!(matches!(parse(br#"{"record_version": "2"}"#), Err(RecordInvalid::Malformed(_))));
}

#[test]
fn an_in_place_deletion_deletes_the_original_and_nothing_else() {
    let mut record = sample();
    record.deletion_mode = DeletionMode::InPlace;
    assert!(matches!(record.validate(), Err(RecordInvalid::Malformed(_))), "a hidden path is not in place");
    record.deletion_path = record.original.clone();
    assert_eq!(parse(&record.to_json()), Ok(record.clone()));
    record.deletion_mode = DeletionMode::Moved;
    assert!(matches!(record.validate(), Err(RecordInvalid::Malformed(_))), "a move to the original moves nothing");
}

#[test]
fn branch_steps_finish_only_after_the_registration_is_gone() {
    let mut record = sample();
    record.progress = Progress::ContentRemoved;
    assert!(matches!(record.validate(), Err(RecordInvalid::Malformed(_))), "a done step before unregistering");
    record.branch_steps[0].state = StepState::Pending;
    assert_eq!(record.validate(), Ok(()));
}

#[test]
fn an_unbound_step_is_always_kept_and_counts_toward_its_kind() {
    let mut record = sample();
    record.progress = Progress::Approved;
    record.branch_steps[0].state = StepState::Pending;
    record.branch_steps[1] = BranchStep {
        action: BranchAction::Unbound { target: UnboundTarget::Remote { remote: "origin".into(), name: "fix/path".into() } },
        approval: StepApproval::Explicit,
        state: StepState::Kept,
    };
    assert_eq!(record.validate(), Ok(()), "kept before unregistering, since it never runs");
    record.branch_steps[1].state = StepState::Pending;
    assert!(matches!(record.validate(), Err(RecordInvalid::Malformed(_))), "an unbound step would run");
    record.branch_steps[1].state = StepState::Kept;
    let second_local = BranchStep {
        action: BranchAction::Unbound { target: UnboundTarget::Local { name: "other".into() } },
        approval: StepApproval::Explicit,
        state: StepState::Kept,
    };
    record.branch_steps.push(second_local);
    assert!(matches!(record.validate(), Err(RecordInvalid::Malformed(_))), "two local steps");
}

#[test]
fn a_binding_matches_only_the_same_repository_and_worktree_record() {
    let record = sample();
    assert_eq!(record.check_binding(&record.repository), Ok(()));
    let mut replaced = record.repository.clone();
    replaced.admin_dir_identity = identity(999);
    assert_eq!(record.check_binding(&replaced), Err(RecordInvalid::IdentityMismatch));
    let mut other = record.repository.clone();
    other.common_dir_identity = identity(998);
    assert_eq!(record.check_binding(&other), Err(RecordInvalid::IdentityMismatch));
}
