//! `darkmatter::config::migration_receipt`: receipts written by a migrating
//! transaction before publication, and the allowlisted cleanup that finishes
//! them, exercised through the public API in temporary directories.

use biscuit_file::canonicalize_simplified;
use darkmatter::config::migration_receipt::{
    CleanupOutcome, MigrationErrorKind, MigrationSource, complete_cleanup, content_hash, read_receipt,
    receipt_path,
};
use darkmatter::config::write::{ConfigTransaction, ConfigWriteError, SourceExpectation, TargetChange};
use darkmatter::yaml_edit::YamlEdit;
use serde_json::{Value, json};
use simplified_schema::PropertyPath;
use std::path::{Path, PathBuf};

const LEGACY_TEXT: &str = "prefix ls\n";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    target: PathBuf,
    legacy: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        let target = root.join("config.yaml");
        let legacy = root.join(".darkmatter-shell-whitelist");
        std::fs::write(&target, "# mine\nnetwork: deny\n").unwrap();
        std::fs::write(&legacy, LEGACY_TEXT).unwrap();
        Self { _dir: dir, root, target, legacy }
    }

    fn known(&self) -> Vec<PathBuf> {
        vec![self.legacy.clone()]
    }

    fn receipt(&self) -> PathBuf {
        receipt_path(&self.target).unwrap()
    }

    fn migrating_change(&self, sources: Vec<MigrationSource>) -> TargetChange {
        TargetChange::edit(
            &self.target,
            SourceExpectation::Current,
            vec![YamlEdit::Set {
                path: PropertyPath::parse("shell-commands").unwrap(),
                value: json!(["prefix ls"]),
            }],
        )
        .with_migration(sources)
    }

    /// Publishes the migrated target with its receipt and stops before
    /// cleanup, as a crash would.
    fn publish(&self, sources: Vec<MigrationSource>) {
        let mut transaction = ConfigTransaction::new();
        transaction.add(self.migrating_change(sources)).unwrap();
        transaction.commit(&|_| Ok(())).unwrap();
    }

    fn publish_deleting_legacy(&self) {
        self.publish(vec![MigrationSource::delete(&self.legacy, LEGACY_TEXT.as_bytes())]);
    }

    fn receipt_value(&self) -> Value {
        serde_json::from_str(&std::fs::read_to_string(self.receipt()).unwrap()).unwrap()
    }

    fn write_receipt(&self, value: &Value) {
        std::fs::write(self.receipt(), serde_json::to_string_pretty(value).unwrap()).unwrap();
    }

    fn assert_legacy_kept(&self) {
        assert_eq!(std::fs::read_to_string(&self.legacy).unwrap(), LEGACY_TEXT);
    }
}

fn refusal(target: &Path, known: &[PathBuf]) -> darkmatter::config::migration_receipt::MigrationError {
    complete_cleanup(target, known).expect_err("cleanup must refuse")
}

#[test]
fn a_receipt_is_written_before_cleanup_and_names_the_published_bytes() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();

    let receipt = read_receipt(&fixture.target).unwrap().expect("a pending receipt");
    let published = std::fs::read(&fixture.target).unwrap();
    assert_eq!(receipt.target, fixture.target);
    assert_eq!(receipt.published_hash, content_hash(&published));
    assert_eq!(receipt.sources, [MigrationSource::delete(&fixture.legacy, LEGACY_TEXT.as_bytes())]);
    assert_eq!(
        fixture.receipt().file_name().unwrap(),
        "config.yaml.darkmatter-migration.json"
    );
    assert_eq!(fixture.receipt_value()["format_version"], 1);
    fixture.assert_legacy_kept();
}

#[test]
fn an_interrupted_cleanup_completes_later() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();

    let outcome = complete_cleanup(&fixture.target, &fixture.known()).unwrap();

    assert_eq!(
        outcome,
        CleanupOutcome::Completed { deleted: vec![fixture.legacy.clone()], backed_up: vec![], already_clean: vec![] }
    );
    assert!(!fixture.legacy.exists());
    assert!(!fixture.receipt().exists());
    assert_eq!(std::fs::read_to_string(&fixture.target).unwrap(), "# mine\nnetwork: deny\nshell-commands:\n- prefix ls\n");
}

#[test]
fn a_completed_cleanup_is_idempotent() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    complete_cleanup(&fixture.target, &fixture.known()).unwrap();

    assert_eq!(complete_cleanup(&fixture.target, &fixture.known()).unwrap(), CleanupOutcome::NothingPending);
    assert_eq!(read_receipt(&fixture.target).unwrap(), None);
}

#[test]
fn an_already_deleted_source_completes_its_item() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    std::fs::remove_file(&fixture.legacy).unwrap();

    let outcome = complete_cleanup(&fixture.target, &fixture.known()).unwrap();

    assert_eq!(
        outcome,
        CleanupOutcome::Completed { deleted: vec![], backed_up: vec![], already_clean: vec![fixture.legacy.clone()] }
    );
    assert!(!fixture.receipt().exists());
}

#[test]
fn a_backup_moves_the_source() {
    let fixture = Fixture::new();
    let backup = fixture.root.join(".darkmatter-shell-whitelist.bak");
    fixture.publish(vec![MigrationSource::backup(&fixture.legacy, LEGACY_TEXT.as_bytes(), &backup)]);

    let outcome = complete_cleanup(&fixture.target, &[fixture.legacy.clone(), backup.clone()]).unwrap();

    assert_eq!(
        outcome,
        CleanupOutcome::Completed {
            deleted: vec![],
            backed_up: vec![(fixture.legacy.clone(), backup.clone())],
            already_clean: vec![]
        }
    );
    assert!(!fixture.legacy.exists());
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), LEGACY_TEXT);
}

#[test]
fn an_occupied_backup_path_is_refused() {
    let fixture = Fixture::new();
    let backup = fixture.root.join(".darkmatter-shell-whitelist.bak");
    fixture.publish(vec![MigrationSource::backup(&fixture.legacy, LEGACY_TEXT.as_bytes(), &backup)]);
    std::fs::write(&backup, "someone else's\n").unwrap();

    let error = refusal(&fixture.target, &[fixture.legacy.clone(), backup.clone()]);

    assert_eq!(error.kind, MigrationErrorKind::BackupOccupied);
    assert_eq!(error.pointer, "/sources/0/backup_path");
    fixture.assert_legacy_kept();
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "someone else's\n");
}

#[test]
fn a_target_changed_after_publication_is_refused() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    std::fs::write(&fixture.target, "network: allow\n").unwrap();

    let error = refusal(&fixture.target, &fixture.known());

    assert_eq!(error.kind, MigrationErrorKind::Unpublished);
    assert_eq!(error.pointer, "/published_hash");
    assert_eq!(error.receipt, fixture.receipt());
    fixture.assert_legacy_kept();
    assert!(fixture.receipt().exists());
}

#[cfg(feature = "effects-instrumentation")]
#[test]
fn a_receipt_whose_target_was_never_published_never_authorizes_cleanup() {
    let fixture = Fixture::new();
    let _failure = darkmatter::config::write::test_hooks::fail_next_replacement();
    let mut transaction = ConfigTransaction::new();
    transaction
        .add(fixture.migrating_change(vec![MigrationSource::delete(&fixture.legacy, LEGACY_TEXT.as_bytes())]))
        .unwrap();
    assert!(matches!(transaction.commit(&|_| Ok(())), Err(ConfigWriteError::ReplacementFailed { .. })));
    assert!(fixture.receipt().exists(), "the receipt is written before publication");

    let error = refusal(&fixture.target, &fixture.known());

    assert_eq!(error.kind, MigrationErrorKind::Unpublished);
    assert_eq!(error.pointer, "/published_hash");
    fixture.assert_legacy_kept();
    assert_eq!(std::fs::read_to_string(&fixture.target).unwrap(), "# mine\nnetwork: deny\n");
}

#[test]
fn a_validation_refusal_writes_no_receipt() {
    let fixture = Fixture::new();
    let mut transaction = ConfigTransaction::new();
    transaction
        .add(fixture.migrating_change(vec![MigrationSource::delete(&fixture.legacy, LEGACY_TEXT.as_bytes())]))
        .unwrap();
    let refused = transaction.commit(&|_| Err(darkmatter::config::write::ValidationRefusal::new("no")));

    assert!(matches!(refused, Err(ConfigWriteError::ValidationFailed { .. })));
    assert!(!fixture.receipt().exists());
}

#[test]
fn a_source_outside_the_known_list_is_refused() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();

    for known in [vec![], vec![fixture.root.join("other-legacy")]] {
        let error = refusal(&fixture.target, &known);
        assert_eq!(error.kind, MigrationErrorKind::UnknownPath);
        assert_eq!(error.pointer, "/sources/0/path");
    }
    fixture.assert_legacy_kept();
    assert!(fixture.receipt().exists());
}

#[test]
fn a_backup_path_outside_the_known_list_is_refused() {
    let fixture = Fixture::new();
    let backup = fixture.root.join("elsewhere.bak");
    fixture.publish(vec![MigrationSource::backup(&fixture.legacy, LEGACY_TEXT.as_bytes(), &backup)]);

    let error = refusal(&fixture.target, &fixture.known());

    assert_eq!(error.kind, MigrationErrorKind::UnknownPath);
    assert_eq!(error.pointer, "/sources/0/backup_path");
    fixture.assert_legacy_kept();
    assert!(!backup.exists());
}

#[test]
fn a_receipt_naming_another_target_is_refused() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    let mut receipt = fixture.receipt_value();
    receipt["target"] = json!(biscuit_file::to_portable_string(&fixture.root.join("other.yaml")));
    fixture.write_receipt(&receipt);

    let error = refusal(&fixture.target, &fixture.known());

    assert_eq!(error.kind, MigrationErrorKind::TargetMismatch);
    assert_eq!(error.pointer, "/target");
    fixture.assert_legacy_kept();
}

#[test]
fn a_source_naming_the_target_is_refused_even_when_listed() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    let mut receipt = fixture.receipt_value();
    let published = std::fs::read(&fixture.target).unwrap();
    receipt["sources"][0]["path"] = receipt["target"].clone();
    receipt["sources"][0]["hash"] = json!(content_hash(&published));
    fixture.write_receipt(&receipt);

    let error = refusal(&fixture.target, std::slice::from_ref(&fixture.target));

    assert_eq!(error.kind, MigrationErrorKind::UnknownPath);
    assert_eq!(error.pointer, "/sources/0/path");
    assert!(fixture.target.exists());
}

#[test]
fn a_changed_source_is_kept() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    std::fs::write(&fixture.legacy, "prefix ls\nprefix rm\n").unwrap();

    let error = refusal(&fixture.target, &fixture.known());

    assert_eq!(error.kind, MigrationErrorKind::SourceChanged);
    assert_eq!(error.pointer, "/sources/0/hash");
    assert_eq!(std::fs::read_to_string(&fixture.legacy).unwrap(), "prefix ls\nprefix rm\n");
    assert!(fixture.receipt().exists());
}

#[test]
fn one_changed_source_keeps_every_source() {
    let fixture = Fixture::new();
    let second = fixture.root.join(".darkmatter-shell-blacklist");
    std::fs::write(&second, "exact rm\n").unwrap();
    fixture.publish(vec![
        MigrationSource::delete(&fixture.legacy, LEGACY_TEXT.as_bytes()),
        MigrationSource::delete(&second, b"exact rm\n"),
    ]);
    std::fs::write(&second, "exact rm -rf\n").unwrap();

    let error = refusal(&fixture.target, &[fixture.legacy.clone(), second.clone()]);

    assert_eq!(error.pointer, "/sources/1/hash");
    fixture.assert_legacy_kept();
    assert!(second.exists());
}

/// The input robustness matrix for the receipt loader: every refused shape is
/// a located `Malformed` error and keeps the source.
#[test]
fn invalid_receipt_shapes_are_located_errors_that_keep_the_source() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    let valid = fixture.receipt_value();
    let valid_text = std::fs::read_to_string(fixture.receipt()).unwrap();
    let edited = |edit: &dyn Fn(&mut Value)| {
        let mut value = valid.clone();
        edit(&mut value);
        serde_json::to_string_pretty(&value).unwrap()
    };
    let backup = biscuit_file::to_portable_string(&fixture.root.join("legacy.bak"));

    let cases: Vec<(&str, String, &str)> = vec![
        ("missing field", edited(&|v| drop(v.as_object_mut().unwrap().remove("published_hash"))), "/published_hash"),
        ("explicit null", edited(&|v| v["target"] = Value::Null), "/target"),
        ("wrong-type field", edited(&|v| v["sources"] = json!("x")), "/sources"),
        ("wrong-type element", edited(&|v| v["sources"] = json!([5])), "/sources/0"),
        ("wrong-type nested field", edited(&|v| v["sources"][0]["hash"] = json!(7)), "/sources/0/hash"),
        ("empty sources", edited(&|v| v["sources"] = json!([])), "/sources"),
        ("unknown root key", edited(&|v| v["note"] = json!("hi")), "/note"),
        ("unknown source key", edited(&|v| v["sources"][0]["mode"] = json!("x")), "/sources/0/mode"),
        ("unknown action", edited(&|v| v["sources"][0]["action"] = json!("shred")), "/sources/0/action"),
        ("format_version 2", edited(&|v| v["format_version"] = json!(2)), "/format_version"),
        ("format_version text", edited(&|v| v["format_version"] = json!("1")), "/format_version"),
        ("short hash", edited(&|v| v["published_hash"] = json!("abc")), "/published_hash"),
        ("backup_path with delete", edited(&|v| v["sources"][0]["backup_path"] = json!(backup)), "/sources/0/backup_path"),
        ("backup without backup_path", edited(&|v| v["sources"][0]["action"] = json!("backup")), "/sources/0/backup_path"),
        ("not an object", "[]".to_string(), ""),
        ("not JSON", "{ format_version: 1 }".to_string(), ""),
        (
            "duplicate key",
            valid_text.replacen("\"format_version\": 1,", "\"format_version\": 1,\n  \"format_version\": 1,", 1),
            "",
        ),
        ("trailing content", format!("{valid_text}{{}}\n"), ""),
    ];

    for (name, text, pointer) in cases {
        std::fs::write(fixture.receipt(), &text).unwrap();
        let error = refusal(&fixture.target, &fixture.known());
        assert_eq!(error.kind, MigrationErrorKind::Malformed, "{name}: {error}");
        assert_eq!(error.pointer, pointer, "{name}: {error}");
        assert_eq!(error.receipt, fixture.receipt(), "{name}");
        fixture.assert_legacy_kept();
        assert_eq!(std::fs::read_to_string(fixture.receipt()).unwrap(), text, "{name}: the receipt is kept");
        assert_eq!(read_receipt(&fixture.target).unwrap_err(), error, "{name}: the reader agrees");
    }

    std::fs::write(fixture.receipt(), &valid_text).unwrap();
    assert!(matches!(
        complete_cleanup(&fixture.target, &fixture.known()).unwrap(),
        CleanupOutcome::Completed { .. }
    ));
}

#[test]
fn an_unreadable_receipt_keeps_the_source() {
    let fixture = Fixture::new();
    fixture.publish_deleting_legacy();
    std::fs::write(fixture.receipt(), [0xff, 0xfe]).unwrap();

    let error = refusal(&fixture.target, &fixture.known());

    assert_eq!(error.kind, MigrationErrorKind::Unreadable);
    fixture.assert_legacy_kept();
}
