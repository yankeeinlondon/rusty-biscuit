//! The temporary conversion of a legacy `config.json` (JSON5) to
//! `config.yaml`, through the public loader at a canonical location.

use std::path::{Path, PathBuf};

use claudine::config::json_conversion::NoticeLevel;
use claudine::dispatch::loader::{load_claudine_config, load_claudine_config_with_notices, load_repo_override_config};
use claudine::error::ClaudineError;
use darkmatter::config::migration_receipt::{MigrationSource, receipt_path};
use darkmatter::config::write::{ConfigTransaction, TargetChange};

struct Scope {
    _dir: tempfile::TempDir,
    dir: PathBuf,
}

impl Scope {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(".claudine");
        std::fs::create_dir_all(&dir).unwrap();
        Self { _dir: root, dir }
    }
    fn yaml(&self) -> PathBuf {
        self.dir.join("config.yaml")
    }
    fn json(&self) -> PathBuf {
        self.dir.join("config.json")
    }
    fn backup(&self) -> PathBuf {
        self.dir.join("config.json.bak")
    }
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

/// A failed conversion names `config.json`, keeps its bytes, and writes no YAML.
fn assert_refused(cell: &str, text: &str) {
    let scope = Scope::new();
    std::fs::write(scope.json(), text).unwrap();
    match load_claudine_config(Some(&scope.yaml()), None) {
        Err(ClaudineError::ConfigConversion { problems }) => {
            assert!(problems.iter().all(|problem| problem.file.as_deref() == Some(scope.json().as_path())), "{cell}: {problems:?}");
        }
        other => panic!("{cell}: expected a conversion error, got {other:?}"),
    }
    assert_eq!(read(&scope.json()), text.as_bytes(), "{cell}: the source is untouched");
    assert!(!scope.yaml().exists() && !scope.backup().exists(), "{cell}: nothing was published");
}

#[test]
fn the_legacy_fixture_converts_to_the_same_values_as_the_yaml_fixture() {
    let scope = Scope::new();
    let json5 = include_str!("../config/user-control.json5");
    std::fs::write(scope.json(), json5).unwrap();
    let loaded = load_claudine_config_with_notices(Some(&scope.yaml()), None).unwrap();

    let yaml_dir = tempfile::tempdir().unwrap();
    let yaml_path = yaml_dir.path().join("config.yaml");
    std::fs::write(&yaml_path, include_str!("../config/user-control.yaml")).unwrap();
    let from_yaml = load_claudine_config(Some(&yaml_path), None).unwrap();
    assert_eq!(serde_json::to_value(&loaded.config).unwrap(), serde_json::to_value(&from_yaml).unwrap());

    assert_eq!(loaded.notices.len(), 1);
    let notice = &loaded.notices[0];
    assert_eq!((notice.code, notice.level), ("dm.config.migration", NoticeLevel::Notice));
    assert!(notice.message.contains("JSON5 comments were not carried over"), "{}", notice.message);
    assert_eq!(notice.files, [scope.json(), scope.yaml()]);
    assert_eq!(read(&scope.backup()), json5.as_bytes(), "the unchanged source is the backup");
    assert!(!scope.json().exists());
    assert!(!receipt_path(&scope.yaml()).unwrap().exists(), "cleanup finished");

    let again = load_claudine_config_with_notices(Some(&scope.yaml()), None).unwrap();
    assert!(again.notices.is_empty(), "a repeated load converts nothing");

    // The flat JSON5 actions and routes are written keyed.
    let written: serde_json::Value =
        biscuit_file::serde_yaml_ng::from_str(&String::from_utf8(read(&scope.yaml())).unwrap()).unwrap();
    assert_eq!(written["actions"]["session_start"][0], serde_json::json!({ "sound_effect": { "effect": "doorbell" } }));
    assert_eq!(written["actions"]["before_tool"][1]["call"]["mapper"], serde_json::json!({ "type": "json_field", "field": "decision" }));
    assert_eq!(
        written["messenger"]["configurations"]["hook"],
        serde_json::json!({ "discord_webhook": { "webhook_url_env": "MY_DISCORD_URL" } }),
        "the hyphenated flat provider is written with its one keyed spelling"
    );
}

#[test]
fn a_converted_file_round_trips_through_a_save() {
    let scope = Scope::new();
    std::fs::write(scope.json(), include_str!("../config/user-control.json5")).unwrap();
    let opened = load_claudine_config(Some(&scope.yaml()), None).unwrap();
    let mut edited = opened.clone();
    edited.actions.insert(
        claudine::events::AgenticEvent::TurnError,
        vec![claudine::actions::HookAction::Bash { command: "echo".into(), params: String::new(), when: None }],
    );
    claudine::config::persist::save_changes(&scope.yaml(), claudine::config::schema::ConfigScope::User, &opened, &edited).unwrap();
    let reread = load_claudine_config(Some(&scope.yaml()), None).unwrap();
    assert_eq!(serde_json::to_value(&reread).unwrap(), serde_json::to_value(&edited).unwrap());
    let text = String::from_utf8(read(&scope.yaml())).unwrap();
    assert!(text.contains("turn_error:") && text.contains("- bash:"), "the new list is written keyed: {text}");
    let unchanged = load_claudine_config(Some(&scope.yaml()), None).unwrap();
    claudine::config::persist::save_changes(&scope.yaml(), claudine::config::schema::ConfigScope::User, &reread, &unchanged).unwrap();
    assert_eq!(String::from_utf8(read(&scope.yaml())).unwrap(), text, "a second read/write/read changes nothing");
}

#[test]
fn comments_and_trailing_commas_are_allowed() {
    let scope = Scope::new();
    std::fs::write(scope.json(), "// mine\n{ logging: false, /* x */ actions: {}, }\n").unwrap();
    assert!(!load_claudine_config(Some(&scope.yaml()), None).unwrap().logging);
}

#[test]
fn malformed_legacy_files_are_refused_and_kept() {
    assert_refused("decoded duplicate key", "{ logging: true, \"logging\": false }");
    assert_refused("trailing garbage", "{ logging: true } trailing");
    assert_refused("inexact number", "{ guard_settings: { volume: { max_bytes: 9007199254740993 } } }");
    assert_refused("not finite", "{ guard_settings: { volume: { max_bytes: Infinity } } }");
    assert_refused("NaN", "{ guard_settings: { volume: { max_bytes: NaN } } }");
    assert_refused("overflow", "{ guard_settings: { volume: { max_bytes: 1e400 } } }");
    assert_refused("wrong root", "[1, 2]");
    assert_refused("scalar root", "\"x\"");
    assert_refused("schema refusal", "{ logging: \"yes\" }");
    assert_refused("unknown flat action", "{ actions: { session_start: [{ type: \"shout\" }] } }");
    assert_refused("flat action missing a field", "{ actions: { session_start: [{ type: \"bash\" }] } }");
    assert_refused("flat route unknown provider", "{ messenger: { configurations: { a: { provider: \"telegram\" } } } }");
    assert_refused("not JSON5", "{ logging: ");
}

#[test]
fn a_duplicate_key_is_located() {
    let scope = Scope::new();
    std::fs::write(scope.json(), "{\n  logging: true,\n  logging: false,\n}\n").unwrap();
    let Err(ClaudineError::ConfigConversion { problems }) = load_claudine_config(Some(&scope.yaml()), None) else {
        panic!("refused")
    };
    assert_eq!((problems[0].line, problems[0].pointer.as_str()), (Some(3), "/logging"));
}

#[test]
fn with_both_files_the_yaml_wins_and_the_json_is_reported_ignored() {
    let scope = Scope::new();
    std::fs::write(scope.yaml(), "logging: false\n").unwrap();
    std::fs::write(scope.json(), "{ logging: not json5").unwrap();
    let loaded = load_claudine_config_with_notices(Some(&scope.yaml()), None).unwrap();
    assert!(!loaded.config.logging);
    assert_eq!(loaded.notices.len(), 1);
    assert_eq!(loaded.notices[0].level, NoticeLevel::Warning);
    assert!(loaded.notices[0].message.contains("is ignored"), "{}", loaded.notices[0].message);
    assert_eq!(read(&scope.yaml()), b"logging: false\n");
    assert_eq!(read(&scope.json()), b"{ logging: not json5");
}

#[test]
fn an_identical_backup_lets_the_unchanged_source_be_removed() {
    let scope = Scope::new();
    std::fs::write(scope.json(), "{ logging: false }").unwrap();
    std::fs::write(scope.backup(), "{ logging: false }").unwrap();
    let loaded = load_claudine_config_with_notices(Some(&scope.yaml()), None).unwrap();
    assert!(!loaded.config.logging);
    assert!(!scope.json().exists());
    assert_eq!(read(&scope.backup()), b"{ logging: false }");
    assert!(loaded.notices[0].message.contains("removed"), "{}", loaded.notices[0].message);
}

#[test]
fn a_differing_backup_is_never_overwritten() {
    let scope = Scope::new();
    std::fs::write(scope.json(), "{ logging: false }").unwrap();
    std::fs::write(scope.backup(), "{ logging: true }").unwrap();
    let Err(ClaudineError::ConfigConversion { problems }) = load_claudine_config(Some(&scope.yaml()), None) else {
        panic!("refused")
    };
    assert_eq!(problems[0].file.as_deref(), Some(scope.backup().as_path()));
    assert_eq!(read(&scope.json()), b"{ logging: false }");
    assert_eq!(read(&scope.backup()), b"{ logging: true }");
    assert!(!scope.yaml().exists());
}

/// Publishes `config.yaml` with a pending receipt and stops, as a crash
/// between publishing and renaming would.
fn interrupted(scope: &Scope, json: &str) {
    std::fs::write(scope.json(), json).unwrap();
    let mut transaction = ConfigTransaction::new();
    transaction
        .add(
            TargetChange::create(scope.yaml(), "logging: false\n", Vec::new())
                .with_migration(vec![MigrationSource::backup(scope.json(), json.as_bytes(), scope.backup())]),
        )
        .unwrap();
    transaction.commit(&|_| Ok(())).unwrap();
    assert!(scope.json().exists() && receipt_path(&scope.yaml()).unwrap().exists());
}

#[test]
fn the_next_load_finishes_an_interrupted_conversion() {
    let scope = Scope::new();
    interrupted(&scope, "{ logging: false }");
    let loaded = load_claudine_config_with_notices(Some(&scope.yaml()), None).unwrap();
    assert!(!loaded.config.logging);
    assert!(!scope.json().exists());
    assert_eq!(read(&scope.backup()), b"{ logging: false }");
    assert!(!receipt_path(&scope.yaml()).unwrap().exists());
    assert_eq!(loaded.notices.len(), 1);
    assert!(load_claudine_config_with_notices(Some(&scope.yaml()), None).unwrap().notices.is_empty());
}

#[test]
fn a_source_changed_after_the_receipt_is_kept() {
    let scope = Scope::new();
    interrupted(&scope, "{ logging: false }");
    std::fs::write(scope.json(), "{ logging: true }").unwrap();
    let result = load_claudine_config(Some(&scope.yaml()), None);
    assert!(matches!(result, Err(ClaudineError::ConfigConversion { .. })), "{result:?}");
    assert_eq!(read(&scope.json()), b"{ logging: true }");
    assert!(!scope.backup().exists());
}

#[test]
fn a_repository_legacy_file_converts_with_the_repository_schema() {
    let scope = Scope::new();
    std::fs::write(scope.json(), "{ canonical_provider: \"gemini\" }").unwrap();
    let repo = load_repo_override_config(&scope.yaml()).unwrap().unwrap();
    assert_eq!(repo.canonical_provider, Some(claudine::provider::Provider::Gemini));
    assert!(scope.yaml().is_file() && scope.backup().is_file());

    let refused = Scope::new();
    std::fs::write(refused.json(), "{ logging: true }").unwrap();
    assert!(matches!(load_repo_override_config(&refused.yaml()), Err(ClaudineError::ConfigConversion { .. })));
    assert!(refused.json().exists());
}

#[test]
fn an_explicit_noncanonical_path_is_read_as_yaml_without_conversion() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.json"), "{ logging: false }").unwrap();
    let custom = dir.path().join("custom.yaml");
    assert!(matches!(load_claudine_config(Some(&custom), None), Err(ClaudineError::ConfigNotFound(_))));
    assert!(dir.path().join("config.json").exists());
    let json_path = dir.path().join("config.json");
    assert!(!load_claudine_config(Some(&json_path), None).unwrap().logging, "read as YAML at that exact path");
    assert!(!dir.path().join("config.yaml").exists() && !dir.path().join("config.json.bak").exists());
}
