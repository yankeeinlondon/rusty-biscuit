use super::*;
use crate::actions::HookAction;
use crate::dispatch::loader::{load_claudine_config, load_repo_override_config};
use crate::events::AgenticEvent;
use crate::provider::Provider;
use serde_json::json;
use std::path::PathBuf;

/// A hand-edited user file: comments, a blank line, a quoted scalar, and a
/// `permissions` block Claudine does not manage.
const COMMENTED: &str = "\
# Claudine settings, edited by hand
preferred_agent: 'claude'   # quoted on purpose

logging: true
tts:
  gender: female   # mine
protect: true

# Darkmatter owns this block
permissions:
  file-read:
    - \"./*.yaml\"
  network: deny
";

fn user_file(text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    std::fs::write(&path, text).unwrap();
    (dir, path)
}

fn load(path: &Path) -> ClaudineConfig {
    load_claudine_config(Some(path), None).unwrap()
}

fn value_of<T: Serialize>(config: &T) -> Value {
    serde_json::to_value(config).unwrap()
}

fn sound(effect: &str) -> HookAction {
    HookAction::SoundEffect { effect: effect.into(), volume: 1.0, speed: 1.0, when: None }
}

#[test]
fn initial_user_text_is_the_default_without_the_never_written_values() {
    let text = initial_text(ConfigScope::User);
    let written = load_yaml_document(&text, Path::new("config.yaml")).unwrap().json;
    assert_eq!(written["tts"], json!(false), "{text}");
    assert!(written.pointer("/steering/automatic/enabled").is_none(), "{text}");

    let (_dir, path) = user_file(&text);
    assert_eq!(value_of(&load(&path)), value_of(&ClaudineConfig::default()));
}

#[test]
fn initial_repository_text_inherits_everything() {
    assert_eq!(initial_text(ConfigScope::Repository), "{}\n");
}

#[test]
fn one_changed_leaf_keeps_every_other_byte() {
    let (_dir, path) = user_file(COMMENTED);
    let opened = load(&path);
    let mut edited = opened.clone();
    edited.logging = false;

    let outcome = save_changes(&path, ConfigScope::User, &opened, &edited).unwrap();

    assert_eq!(outcome, SaveOutcome { created: false, written: true });
    assert_eq!(std::fs::read_to_string(&path).unwrap(), COMMENTED.replace("logging: true", "logging: false"));
}

#[test]
fn read_write_read_is_stable() {
    let (_dir, path) = user_file(COMMENTED);
    let first = value_of(&load(&path));
    for pass in 0..2 {
        let opened = load(&path);
        let outcome = save_changes(&path, ConfigScope::User, &opened, &opened.clone()).unwrap();
        assert!(!outcome.written, "pass {pass}");
        assert_eq!(value_of(&load(&path)), first, "pass {pass}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), COMMENTED, "pass {pass}");
    }
}

#[test]
fn two_sessions_and_another_writer_keep_each_others_edits() {
    let (_dir, path) = user_file(COMMENTED);
    let opened_a = load(&path);
    let opened_b = load(&path);

    // Another program edits the permissions block while both are open.
    let mut transaction = ConfigTransaction::new();
    transaction
        .add(TargetChange::edit(&path, SourceExpectation::Current, vec![set_path("permissions.network", json!("allow")).unwrap()]))
        .unwrap();
    transaction.commit(&|_| Ok(())).unwrap();

    let mut edited_a = opened_a.clone();
    edited_a.logging = false;
    save_changes(&path, ConfigScope::User, &opened_a, &edited_a).unwrap();

    let mut edited_b = opened_b.clone();
    edited_b.prompt_for_missing = false;
    save_changes(&path, ConfigScope::User, &opened_b, &edited_b).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let expected = COMMENTED
        .replace("logging: true", "logging: false")
        .replace("network: deny", "network: allow");
    let expected = format!("{expected}prompt_for_missing: false\n");
    assert_eq!(text, expected);
    let saved = load(&path);
    assert!(!saved.logging);
    assert!(!saved.prompt_for_missing);
}

#[test]
fn a_changed_sequence_is_replaced_as_one_node() {
    let text = "actions:\n  human_in_the_loop:   # ring\n    - type: sound_effect\n      effect: doorbell\n  session_start:\n    - type: sound_effect\n      effect: error-1\n";
    let (_dir, path) = user_file(text);
    let opened = load(&path);
    let mut edited = opened.clone();
    edited.actions.insert(AgenticEvent::HumanInTheLoop, vec![sound("doorbell"), sound("error-1")]);

    let authored = load_yaml_document(text, &path).unwrap().json;
    let edits = changed_edits(Some(&authored), &value_of(&opened), &value_of(&edited)).unwrap();
    assert_eq!(edits.len(), 1, "{edits:?}");
    let YamlEdit::Set { path: edited_path, value } = &edits[0] else { panic!("{edits:?}") };
    assert_eq!(edited_path.segments(), ["actions", "human_in_the_loop"]);
    assert_eq!(value.as_array().map(Vec::len), Some(2));

    save_changes(&path, ConfigScope::User, &opened, &edited).unwrap();
    let saved = load(&path);
    assert_eq!(saved.actions[&AgenticEvent::HumanInTheLoop], vec![sound("doorbell"), sound("error-1")]);
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.starts_with("actions:\n  human_in_the_loop:   # ring\n"), "{written}");
    assert!(written.ends_with("  session_start:\n    - type: sound_effect\n      effect: error-1\n"), "{written}");
}

#[test]
fn a_removed_key_is_removed() {
    let (_dir, path) = user_file("# keep me\npreferred_agent: codex\nlogging: true\n");
    let opened = load(&path);
    let mut edited = opened.clone();
    edited.preferred_agent = None;

    save_changes(&path, ConfigScope::User, &opened, &edited).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# keep me\nlogging: true\n");
}

#[test]
fn a_removed_default_is_written_as_null() {
    let opened = json!({"tts": {"gender": "female", "voice": "Samantha"}});
    let edited = json!({"tts": {"gender": "female"}});
    let edits = changed_edits(Some(&json!({})), &opened, &edited).unwrap();
    assert_eq!(edits, vec![set_path("tts.voice", Value::Null).unwrap()]);
}

#[test]
fn a_scalar_in_the_file_is_replaced_whole_even_when_the_values_are_mappings() {
    let authored = json!({"protect": false});
    let opened = json!({"protect": {"enabled": false}});
    let edited = json!({"protect": {"enabled": true}});
    let edits = changed_edits(Some(&authored), &opened, &edited).unwrap();
    assert_eq!(edits, vec![set_path("protect", json!({"enabled": true})).unwrap()]);
}

#[test]
fn the_former_name_of_preferred_agent_is_edited_in_place() {
    let (_dir, path) = user_file("favorite_agent: codex # old name\n");

    edit_config(&path, ConfigScope::User, vec![set_path("preferred_agent", json!("claude")).unwrap()]).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "favorite_agent: claude # old name\n");

    edit_config(&path, ConfigScope::User, vec![remove_path("preferred_agent").unwrap()]).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}\n");
    assert!(load(&path).preferred_agent.is_none());
}

#[test]
fn an_unchanged_configuration_writes_nothing() {
    let (dir, path) = user_file(COMMENTED);
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    let opened = load(&path);

    let outcome = save_changes(&path, ConfigScope::User, &opened, &opened.clone()).unwrap();

    assert_eq!(outcome, SaveOutcome::default());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), COMMENTED);
    assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), before);
    assert!(!dir.path().join("config.yaml.darkmatter-lock").exists(), "no transaction was started");
}

#[test]
fn an_edit_that_already_holds_writes_nothing() {
    let (_dir, path) = user_file(COMMENTED);
    let outcome = edit_config(&path, ConfigScope::User, vec![set_path("logging", json!(true)).unwrap()]).unwrap();
    assert!(!outcome.written);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), COMMENTED);
}

#[test]
fn an_invalid_result_is_refused_and_the_file_kept() {
    let (_dir, path) = user_file(COMMENTED);

    let schema = edit_config(&path, ConfigScope::User, vec![set_path("logging", json!("yes")).unwrap()]).unwrap_err();
    let ClaudineError::ConfigEditRefused { problems, .. } = &schema else { panic!("{schema:?}") };
    assert_eq!(problems[0].pointer, "/logging");
    assert_eq!(problems[0].line, Some(4), "{schema}");

    let semantic =
        edit_config(&path, ConfigScope::User, vec![set_path("default_sounds.success", json!("no-such-sound")).unwrap()])
            .unwrap_err();
    assert!(matches!(semantic, ClaudineError::ConfigEditRefused { .. }), "{semantic:?}");
    assert!(semantic.to_string().contains("no-such-sound"), "{semantic}");

    assert_eq!(std::fs::read_to_string(&path).unwrap(), COMMENTED);
}

#[test]
fn an_edit_of_a_missing_file_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    let error = edit_config(&path, ConfigScope::User, vec![set_path("logging", json!(false)).unwrap()]).unwrap_err();
    assert!(matches!(error, ClaudineError::ConfigNotFound(_)), "{error:?}");
    assert!(!path.exists());
}

#[test]
fn a_repository_file_gets_only_repository_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claudine/config.yaml");
    let empty = RepoOverrideConfig::default();

    assert_eq!(save_changes(&path, ConfigScope::Repository, &empty, &empty.clone()).unwrap(), SaveOutcome::default());
    assert!(!path.exists(), "nothing to save creates nothing");

    let edited = RepoOverrideConfig { canonical_provider: Some(Provider::Codex), ..RepoOverrideConfig::default() };
    let outcome = save_changes(&path, ConfigScope::Repository, &empty, &edited).unwrap();
    assert_eq!(outcome, SaveOutcome { created: true, written: true });
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "canonical_provider: codex\n");
    let loaded = load_repo_override_config(&path).unwrap().unwrap();
    assert_eq!(loaded.canonical_provider, Some(Provider::Codex));

    let refused = edit_config(&path, ConfigScope::Repository, vec![set_path("tts", json!(true)).unwrap()]).unwrap_err();
    assert!(matches!(refused, ClaudineError::ConfigEditRefused { .. }), "{refused:?}");
}

#[test]
fn initialization_creates_from_defaults_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claudine/config.yaml");
    let mut initialized = ClaudineConfig { preferred_agent: Some(Provider::Codex), ..ClaudineConfig::default() };
    initialized.default_sounds.success = Some("confirmation".into());
    initialized.actions.insert(AgenticEvent::HumanInTheLoop, vec![sound("doorbell")]);

    let created = initialize_config(&path, ConfigScope::User, &ClaudineConfig::default(), &initialized).unwrap();
    assert_eq!(created, SaveOutcome { created: true, written: true });
    assert_eq!(value_of(&load(&path)), value_of(&initialized));

    let first = std::fs::read(&path).unwrap();
    let again = initialize_config(&path, ConfigScope::User, &ClaudineConfig::default(), &initialized).unwrap();
    assert!(!again.written);
    assert_eq!(std::fs::read(&path).unwrap(), first);
}

#[test]
fn initialization_with_nothing_chosen_still_creates_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    let defaults = ClaudineConfig::default();
    initialize_config(&path, ConfigScope::User, &defaults, &defaults.clone()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), initial_text(ConfigScope::User));
}

#[test]
fn initialization_of_an_existing_file_keeps_unmanaged_text() {
    let (_dir, path) = user_file(COMMENTED);
    let mut initialized = ClaudineConfig::default();
    initialized.default_sounds.attention = Some("doorbell".into());

    initialize_config(&path, ConfigScope::User, &ClaudineConfig::default(), &initialized).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with(COMMENTED), "{text}");
    assert_eq!(load(&path).default_sounds.attention.as_deref(), Some("doorbell"));
}

#[test]
fn validate_config_text_locates_problems() {
    let path = Path::new("/somewhere/config.yaml");
    validate_config_text("logging: false\n", path, ConfigScope::User).unwrap();
    let error = validate_config_text("canonical_provider: codex\ntts: true\n", path, ConfigScope::Repository).unwrap_err();
    let ClaudineError::ConfigInvalid { problems } = error else { panic!("{error:?}") };
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].pointer, "/tts");
    assert_eq!(problems[0].line, Some(2));
}
