use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sniff::filesystem::query::{FileIdentity, PathUsageReport};

use super::*;
use crate::remove::recovery::OperationId;
use crate::remove::recovery::compact;
use crate::remove::recovery::record::{QUARANTINE_PREFIX, RECORD_VERSION, RepositoryBinding};

fn root() -> PathBuf {
    std::env::temp_dir().join("wt-legacy-record")
}

fn identity(inode: u64) -> FileIdentity {
    FileIdentity::Unix { device: 7, inode }
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

const ID: &str = "0123456789abcdef0123456789abcdef";

/// A legacy operation that deletes its checkout, its branch, and the branch
/// on origin, stopped after the move.
fn legacy_record(phase: Phase) -> RecoveryRecord {
    let root = root();
    let common = root.join("repo.git");
    let original = root.join("wt1");
    RecoveryRecord {
        record_version: RECORD_VERSION,
        operation_id: OperationId::parse(ID).unwrap(),
        created_at_ms: 17,
        requested_name: "fix/path".into(),
        aliases: vec!["wt1".into()],
        original: original.as_path().into(),
        quarantine: root.join(format!("{QUARANTINE_PREFIX}wt1-a91c")).as_path().into(),
        checkout_identity: identity(3),
        repository: RepositoryBinding {
            common_dir: common.as_path().into(),
            common_dir_identity: identity(1),
            admin_dir: common.join("worktrees").join("wt1").as_path().into(),
            admin_dir_identity: identity(103),
        },
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

/// The shape of a version-6 report, as far as migration reads it.
fn report(version: Value, operation_id: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "report_version": version,
        "operation_id": operation_id,
        "outcome": "activity_detected",
        "journal_entries": [],
    }))
    .unwrap()
}

#[test]
fn record_v1_with_report_v6_migrates_its_checkout_approval_and_keeps_its_remote_step_undone() {
    let legacy = read(&legacy_record(Phase::Deleting).to_json(), Some(&report(json!(6), json!(ID)))).unwrap();
    assert_eq!(legacy.report, LegacyReport::Read { version: 6 });
    let compact = legacy.to_compact();
    let source = &legacy.record;
    assert_eq!(
        (&compact.operation_id, &compact.original, &compact.deletion_path, compact.checkout_identity, &compact.repository),
        (&source.operation_id, &source.original, &source.quarantine, source.checkout_identity, &source.repository)
    );
    assert_eq!((compact.deletion_mode, compact.progress, compact.unlock_needed), (DeletionMode::Moved, Progress::Placed, false));
    assert_eq!((&compact.requested_name, &compact.aliases), (&source.requested_name, &source.aliases));
    assert_eq!(
        compact.branch_steps,
        vec![
            BranchStep {
                action: BranchAction::Local { name: "fix/path".into(), expected_tip: "a".repeat(40) },
                approval: StepApproval::Conditional,
                state: StepState::Pending,
            },
            BranchStep {
                action: BranchAction::Unbound { target: UnboundTarget::Remote { remote: "origin".into(), name: "fix/path".into() } },
                approval: StepApproval::Explicit,
                state: StepState::Kept,
            },
        ]
    );
    // What migration publishes reads back as itself.
    assert_eq!(compact::parse(&compact.to_json()), Ok(compact));
}

#[test]
fn record_v1_alone_migrates_the_same_as_with_its_report() {
    let bytes = legacy_record(Phase::Deleting).to_json();
    let alone = read(&bytes, None).unwrap();
    assert_eq!(alone.report, LegacyReport::Absent);
    assert_eq!(alone.to_compact(), read(&bytes, Some(&report(json!(6), json!(ID)))).unwrap().to_compact());
}

#[test]
fn a_report_never_changes_what_migration_authorizes() {
    let bytes = legacy_record(Phase::Moved).to_json();
    let expected = read(&bytes, None).unwrap().to_compact();
    let mut duplicated = report(json!(6), json!(ID));
    duplicated.splice(1..1, br#""report_version":6,"#.iter().copied());
    let cells: Vec<(&str, Vec<u8>, bool)> = vec![
        ("v4", report(json!(4), json!(ID)), true),
        ("v5", report(json!(5), json!(ID)), true),
        ("no operation id yet", report(json!(6), Value::Null), true),
        ("newer than any legacy wt", report(json!(7), json!(ID)), false),
        ("version zero", report(json!(0), json!(ID)), false),
        ("version as text", report(json!("6"), json!(ID)), false),
        ("version absent", serde_json::to_vec(&json!({"operation_id": ID})).unwrap(), false),
        ("another operation", report(json!(6), json!("ffffffffffffffffffffffffffffffff")), false),
        ("operation id absent", serde_json::to_vec(&json!({"report_version": 6})).unwrap(), false),
        ("duplicate key", duplicated, false),
        ("not JSON", b"report".to_vec(), false),
        ("truncated", report(json!(6), json!(ID))[..20].to_vec(), false),
    ];
    for (cell, bytes_of_report, readable) in cells {
        let legacy = read(&bytes, Some(&bytes_of_report)).unwrap_or_else(|error| panic!("{cell}: {error}"));
        assert_eq!(matches!(legacy.report, LegacyReport::Read { .. }), readable, "{cell}: {:?}", legacy.report);
        assert_eq!(legacy.to_compact(), expected, "{cell}");
    }
}

#[test]
fn a_report_version_alone_grants_nothing_without_a_valid_record() {
    let good_report = report(json!(6), json!(ID));
    let mut doc: Value = serde_json::from_slice(&legacy_record(Phase::Moved).to_json()).unwrap();
    doc["approved_tip"] = json!("not-a-commit");
    let malformed = serde_json::to_vec(&doc).unwrap();
    assert!(matches!(read(&malformed, Some(&good_report)), Err(RecordInvalid::Malformed(_))));
    doc["record_version"] = json!(5);
    let unknown = serde_json::to_vec(&doc).unwrap();
    assert_eq!(read(&unknown, Some(&good_report)), Err(RecordInvalid::UnsupportedVersion(5)));
    assert!(read(b"", Some(&good_report)).is_err());
    assert!(read(&good_report, Some(&good_report)).is_err(), "a report is not a record");
}

#[test]
fn every_legacy_phase_maps_to_progress_no_further_than_it_proves() {
    let cells = [
        (Phase::Prepared, Progress::Approved),
        (Phase::Moved, Progress::Placed),
        (Phase::Associated, Progress::Placed),
        (Phase::Deleting, Progress::Placed),
        (Phase::ContentRemoved, Progress::ContentRemoved),
        (Phase::Unregistered, Progress::Unregistered),
        (Phase::BranchActions, Progress::Unregistered),
    ];
    for (phase, progress) in cells {
        let legacy = read(&legacy_record(phase).to_json(), None).unwrap();
        assert!(!legacy.is_completed(), "{phase:?}");
        let compact = legacy.to_compact();
        assert_eq!(compact.progress, progress, "{phase:?}");
        // Branch steps start over: an absent branch later completes its step.
        assert_eq!(compact.branch_steps[0].state, StepState::Pending, "{phase:?}");
        assert_eq!(compact.validate(), Ok(()), "{phase:?}");
    }
    assert!(read(&legacy_record(Phase::Completed).to_json(), None).unwrap().is_completed());
}

#[test]
fn a_local_step_for_another_branch_than_the_checkouts_is_never_rebuilt() {
    let mut record = legacy_record(Phase::Moved);
    record.branch = BranchBinding::Detached;
    record.intended_actions.truncate(2);
    let compact = read(&record.to_json(), None).unwrap().to_compact();
    assert_eq!(
        compact.branch_steps,
        vec![BranchStep {
            action: BranchAction::Unbound { target: UnboundTarget::Local { name: "fix/path".into() } },
            approval: StepApproval::Explicit,
            state: StepState::Kept,
        }]
    );
    record.intended_actions.truncate(1);
    assert!(read(&record.to_json(), None).unwrap().to_compact().branch_steps.is_empty());
}
