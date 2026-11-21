//! The indentation of the YAML Claudine writes, through its normal entry
//! points: setup's first write and permissions handshake, `save_changes`,
//! and the legacy `config.json` conversion. A new file takes its
//! destination's preference (four spaces outside a repository, the
//! worktree's `.editorconfig` inside one); an edit keeps the file's own
//! layout. Parsed values and key order are checked apart from exact layout.

use std::collections::HashMap;
use std::num::NonZeroU8;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, Indentation, canonicalize_simplified};
use claudine::actions::HookAction;
use claudine::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};
use claudine::config::persist::{SaveOutcome, initial_text, initialize_config, save_changes};
use claudine::config::schema::ConfigScope;
use claudine::config::setup::{PermissionsHandoff, setup_with_darkmatter};
use claudine::dispatch::loader::{load_claudine_config, load_claudine_config_with_notices, load_repo_override_config};
use claudine::error::ClaudineError;
use claudine::events::AgenticEvent;
use darkmatter::config::init::default_permissions;
use darkmatter::config::migration_receipt::receipt_path;
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use serde_json::{Value, json};
use simplified_schema::yaml_documents::load_yaml_document;

/// A temporary directory that is not inside any Git worktree.
fn plain_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = canonicalize_simplified(dir.path()).unwrap();
    (dir, root)
}

/// A Git worktree (a `.git` skeleton, so no `git` is spawned) whose
/// `.editorconfig` asks for two-space YAML.
fn two_space_repository() -> (tempfile::TempDir, PathBuf) {
    let (dir, root) = plain_dir();
    let git = root.join(".git");
    std::fs::create_dir_all(git.join("objects")).unwrap();
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").unwrap();
    std::fs::write(root.join(".editorconfig"), "root = true\n\n[*.yaml]\nindent_style = space\nindent_size = 2\n").unwrap();
    (dir, root)
}

fn context(root: &Path) -> FileResolutionContext {
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    FileResolutionContext::from_snapshot(root.to_path_buf(), Some(home), HashMap::new())
}

fn document(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap();
    load_yaml_document(&text, path).unwrap().json
}

fn keys(value: &Value) -> Vec<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

fn spaces(width: u8) -> Indentation {
    Indentation::spaces(NonZeroU8::new(width).unwrap())
}

fn sound(effect: &str) -> HookAction {
    HookAction::SoundEffect { effect: effect.into(), volume: 1.0, speed: 1.0, when: None }
}

#[test]
fn setup_outside_a_repository_writes_four_space_permissions() {
    let (_dir, root) = plain_dir();
    let context = context(&root);

    let setup = setup_with_darkmatter(&context, ConfigScope::User, |path| {
        initialize_config(path, ConfigScope::User, &ClaudineConfig::default(), &ClaudineConfig::default())
    })
    .unwrap();

    assert_eq!(setup.permissions, PermissionsHandoff::Referenced);
    let path = root.join("home/.claudine/config.yaml");
    let written = document(&path);
    assert_eq!(written["permissions"], default_permissions(DarkmatterScope::User));
    let mut expected_keys = keys(&load_yaml_document(&initial_text(ConfigScope::User), &path).unwrap().json);
    expected_keys.push("permissions".into());
    assert_eq!(keys(&written), expected_keys, "the projection's order, then the handshake's block");

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\npermissions:\n    file-read:\n        - ./*.txt\n        - ./*.text\n"), "{text}");
    assert!(text.contains("\nguard_settings:\n    repetition:\n        enabled: true\n"), "{text}");
}

#[test]
fn setup_in_a_repository_takes_the_worktrees_editorconfig_width() {
    let (_dir, root) = two_space_repository();
    let context = context(&root).with_repository_root(&root);

    setup_with_darkmatter(&context, ConfigScope::Repository, |path| {
        initialize_config(path, ConfigScope::Repository, &RepoOverrideConfig::default(), &RepoOverrideConfig::default())
    })
    .unwrap();

    let path = root.join(".claudine/config.yaml");
    let block = default_permissions(DarkmatterScope::Repository);
    assert_eq!(document(&path), json!({ "permissions": block }));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("permissions:\n  file-read:\n    - '&**/*.txt'\n"), "{text}");
    assert_eq!(text, biscuit_file::to_yaml_string(&json!({ "permissions": block }), spaces(2)).unwrap());
}

#[test]
fn create_edit_and_an_unchanged_save_round_trip() {
    let (_dir, root) = plain_dir();
    let path = root.join(".claudine/config.yaml");
    let created = initialize_config(&path, ConfigScope::User, &ClaudineConfig::default(), &ClaudineConfig::default()).unwrap();
    assert_eq!(created, SaveOutcome { created: true, written: true });
    assert_eq!(std::fs::read_to_string(&path).unwrap(), initial_text(ConfigScope::User), "four spaces, as the preview");

    let opened = load_claudine_config(Some(&path), None).unwrap();
    let mut edited = opened.clone();
    edited.actions.insert(AgenticEvent::SessionStart, vec![sound("doorbell")]);
    save_changes(&path, ConfigScope::User, &opened, &edited).unwrap();

    let reread = load_claudine_config(Some(&path), None).unwrap();
    assert_eq!(serde_json::to_value(&reread).unwrap(), serde_json::to_value(&edited).unwrap());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\nactions:\n    session_start:\n        - sound_effect:\n"), "{text}");

    let bytes = std::fs::read(&path).unwrap();
    let again = load_claudine_config(Some(&path), None).unwrap();
    let outcome = save_changes(&path, ConfigScope::User, &reread, &again).unwrap();
    assert!(!outcome.written);
    let outcome = initialize_config(&path, ConfigScope::User, &ClaudineConfig::default(), &ClaudineConfig::default()).unwrap();
    assert!(!outcome.written);
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "a second save leaves every byte");
}

#[test]
fn an_edit_keeps_a_commented_files_own_two_space_layout() {
    let (_dir, root) = plain_dir();
    let path = root.join("config.yaml");
    let authored = "\
# mine
logging: true   # keep
guard_settings:
  repetition:
    enabled: true
    max_repeats: 30
    max_cycle_length: 16
  volume:
    enabled: true
    max_lines: 50000
    max_bytes: 33554432
permissions:
  file-read:
  - ./*.md
";
    std::fs::write(&path, authored).unwrap();
    let opened = load_claudine_config(Some(&path), None).unwrap();
    let mut edited = opened.clone();
    edited.actions.insert(AgenticEvent::SessionStart, vec![sound("doorbell")]);
    edited.logging = false;

    save_changes(&path, ConfigScope::User, &opened, &edited).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let kept = authored.replace("logging: true   # keep", "logging: false   # keep");
    assert!(text.starts_with(&kept), "every authored line, comment, and indentless list is kept: {text}");
    // An indentless list is never an example, so the new one takes the
    // file's two-space mapping offset rather than the four-space preference.
    let added = &text[kept.len()..];
    assert!(added.starts_with("actions:\n  session_start:\n    - sound_effect:\n"), "{added}");
    let fields: Vec<&str> = added.lines().skip(3).collect();
    assert!(fields.contains(&"        effect: doorbell"), "{added}");
    assert!(fields.iter().all(|line| line.starts_with("        ") && !line.starts_with("         ")), "{added}");
    let reread = load_claudine_config(Some(&path), None).unwrap();
    assert_eq!(serde_json::to_value(&reread).unwrap(), serde_json::to_value(&edited).unwrap());
}

#[test]
fn a_refused_creation_writes_nothing() {
    let (_dir, root) = plain_dir();
    let path = root.join(".claudine/config.yaml");
    let mut initialized = ClaudineConfig::default();
    initialized.default_sounds.success = Some("no-such-sound".into());

    let error = initialize_config(&path, ConfigScope::User, &ClaudineConfig::default(), &initialized).unwrap_err();

    assert!(matches!(error, ClaudineError::ConfigEditRefused { .. }), "{error:?}");
    assert!(!path.exists());
}

/// The canonical `.claudine` directory under `base`, holding `json` as its
/// legacy file.
fn legacy(base: &Path, json: &str) -> PathBuf {
    let dir = base.join(".claudine");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.json"), json).unwrap();
    dir
}

const USER_JSON: &str = r#"{ "logging": false, "guard_settings": { "repetition": { "enabled": false, "max_repeats": 3, "max_cycle_length": 2 }, "volume": { "enabled": true, "max_lines": 10, "max_bytes": 20 } }, "actions": { "session_start": [{ "type": "sound_effect", "effect": "doorbell" }] } }"#;

#[test]
fn a_legacy_user_file_converts_with_four_spaces_and_keeps_its_backup() {
    let (_dir, root) = plain_dir();
    let dir = legacy(&root, USER_JSON);
    let yaml = dir.join("config.yaml");

    let loaded = load_claudine_config_with_notices(Some(&yaml), None).unwrap();

    assert_eq!(loaded.notices.len(), 1, "{:?}", loaded.notices);
    assert!(!loaded.config.logging);
    assert_eq!(std::fs::read(dir.join("config.json.bak")).unwrap(), USER_JSON.as_bytes());
    assert!(!dir.join("config.json").exists() && !receipt_path(&yaml).unwrap().exists());
    let written = document(&yaml);
    assert_eq!(keys(&written), ["logging", "guard_settings", "actions"]);
    assert_eq!(written["actions"]["session_start"], json!([{ "sound_effect": { "effect": "doorbell" } }]));
    let text = std::fs::read_to_string(&yaml).unwrap();
    assert_eq!(
        text,
        "logging: false\nguard_settings:\n    repetition:\n        enabled: false\n        max_repeats: 3\n        max_cycle_length: 2\n    volume:\n        enabled: true\n        max_lines: 10\n        max_bytes: 20\nactions:\n    session_start:\n        - sound_effect:\n              effect: doorbell\n"
    );
}

#[test]
fn a_legacy_repository_file_converts_with_the_worktrees_width() {
    let (_dir, root) = two_space_repository();
    let json = r#"{ "canonical_provider": "gemini", "guard_settings": { "repetition": { "enabled": true, "max_repeats": 3, "max_cycle_length": 2 }, "volume": { "enabled": true, "max_lines": 10, "max_bytes": 20 } } }"#;
    let dir = legacy(&root, json);
    let yaml = dir.join("config.yaml");

    let repo = load_repo_override_config(&yaml).unwrap().unwrap();

    assert_eq!(repo.canonical_provider, Some(claudine::provider::Provider::Gemini));
    assert_eq!(std::fs::read(dir.join("config.json.bak")).unwrap(), json.as_bytes());
    assert_eq!(
        std::fs::read_to_string(&yaml).unwrap(),
        "canonical_provider: gemini\nguard_settings:\n  repetition:\n    enabled: true\n    max_repeats: 3\n    max_cycle_length: 2\n  volume:\n    enabled: true\n    max_lines: 10\n    max_bytes: 20\n"
    );
}

/// A conversion whose YAML cannot be published reports it, leaves the legacy
/// file and no YAML, and the next load converts it.
fn assert_failed_publication_recovers(base: &Path, json: &str, load: impl Fn(&Path) -> claudine::error::Result<()>) {
    let dir = legacy(base, json);
    let yaml = dir.join("config.yaml");
    {
        let _hook = darkmatter::config::write::test_hooks::fail_next_replacement();
        let error = load(&yaml).unwrap_err();
        let ClaudineError::ConfigConversion { problems } = &error else { panic!("{error:?}") };
        assert_eq!(problems[0].file.as_deref(), Some(dir.join("config.json").as_path()), "{error}");
    }
    assert_eq!(std::fs::read(dir.join("config.json")).unwrap(), json.as_bytes());
    assert!(!yaml.exists() && !dir.join("config.json.bak").exists());

    load(&yaml).unwrap();
    assert!(yaml.is_file() && !dir.join("config.json").exists());
    assert_eq!(std::fs::read(dir.join("config.json.bak")).unwrap(), json.as_bytes());
}

#[test]
fn a_failed_user_conversion_is_reported_and_retried() {
    let (_dir, root) = plain_dir();
    assert_failed_publication_recovers(&root, r#"{ "logging": false }"#, |yaml| load_claudine_config(Some(yaml), None).map(|_| ()));
}

#[test]
fn a_failed_repository_conversion_is_reported_and_retried() {
    let (_dir, root) = two_space_repository();
    assert_failed_publication_recovers(&root, r#"{ "canonical_provider": "codex" }"#, |yaml| {
        load_repo_override_config(yaml).map(|_| ())
    });
}
