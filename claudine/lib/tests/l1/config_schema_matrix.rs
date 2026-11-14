//! Input robustness matrix for Claudine's configuration files, through the
//! public loader (`load_claudine_config`, `load_repo_override_config`).
//!
//! The control is `tests/config/user-control.yaml`, which writes every
//! setting; each negative cell edits exactly one value of it and must fail
//! with a located `ConfigInvalid` problem at that value's JSON pointer, or
//! with the semantic error `ClaudineConfig::validate` keeps.

use std::path::{Path, PathBuf};

use claudine::config::claudine_config::ClaudineConfig;
use claudine::config::{ModelOverrideMode, MessengerProviderConfig, TtsValue, VoiceSelection};
use claudine::dispatch::loader::{load_claudine_config, load_repo_override_config};
use claudine::error::ClaudineError;
use serde_json::{Value, json};

fn yaml(text: &str) -> Value {
    biscuit_file::serde_yaml_ng::from_str(text).expect("fixture is YAML")
}

fn user_control() -> Value {
    yaml(include_str!("../config/user-control.yaml"))
}

fn repository_control() -> Value {
    yaml(include_str!("../config/repository-control.yaml"))
}

/// `document` with the value at `pointer` replaced by `value`, or removed.
fn edited(document: &Value, pointer: &str, value: Option<Value>) -> Value {
    let mut document = document.clone();
    let (parent, key) = pointer.rsplit_once('/').expect("a pointer");
    let target = document.pointer_mut(parent).unwrap_or_else(|| panic!("{parent} exists"));
    match (target, value) {
        (Value::Object(map), Some(value)) => {
            map.insert(key.to_string(), value);
        }
        (Value::Object(map), None) => {
            map.remove(key);
        }
        (Value::Array(items), Some(value)) => items[key.parse::<usize>().unwrap()] = value,
        (Value::Array(items), None) => {
            items.remove(key.parse::<usize>().unwrap());
        }
        _ => panic!("{parent} is a container"),
    }
    document
}

fn write_yaml(path: &Path, document: &Value) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, biscuit_file::serde_yaml_ng::to_string(document).unwrap()).unwrap();
}

fn load_user(document: &Value) -> (tempfile::TempDir, Result<ClaudineConfig, ClaudineError>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    write_yaml(&path, document);
    let result = load_claudine_config(Some(&path), None);
    (dir, result)
}

fn load_with_repository(user: &Value, repository: &Value) -> (tempfile::TempDir, Result<ClaudineConfig, ClaudineError>) {
    let dir = tempfile::tempdir().unwrap();
    let user_path = dir.path().join("user.yaml");
    write_yaml(&user_path, user);
    let root = dir.path().join("repo");
    write_yaml(&root.join(".claudine/config.yaml"), repository);
    let result = load_claudine_config(Some(&user_path), Some(&root));
    (dir, result)
}

fn loads(cell: &str, document: &Value) -> ClaudineConfig {
    match load_user(document).1 {
        Ok(config) => config,
        Err(error) => panic!("{cell}: should load, got {error}"),
    }
}

/// The load fails with a located problem at `pointer`, in the file at line > 0.
fn assert_located(cell: &str, result: Result<ClaudineConfig, ClaudineError>, pointer: &str) {
    match result {
        Err(ClaudineError::ConfigInvalid { problems }) => {
            let problem = problems
                .iter()
                .find(|problem| problem.pointer == pointer || problem.pointer.starts_with(&format!("{pointer}/")))
                .unwrap_or_else(|| panic!("{cell}: no problem at {pointer}: {problems:?}"));
            assert!(problem.file.as_ref().is_some_and(|file| file.extension().is_some_and(|ext| ext == "yaml")), "{cell}");
            assert!(problem.line.is_some_and(|line| line > 0) && problem.column.is_some(), "{cell}: {problem:?}");
        }
        Err(other) => panic!("{cell}: expected a located ConfigInvalid at {pointer}, got {other}"),
        Ok(_) => panic!("{cell}: expected a located ConfigInvalid at {pointer}, loaded"),
    }
}

fn rejected(cell: &str, pointer: &str, value: Option<Value>) {
    assert_located(cell, load_user(&edited(&user_control(), pointer, value)).1, pointer);
}

/// As [`rejected`], for an edit inside a union arm: the problem is located
/// at the union's value, `at`, or below it.
fn rejected_within(cell: &str, pointer: &str, value: Option<Value>, at: &str) {
    assert_located(cell, load_user(&edited(&user_control(), pointer, value)).1, at);
}

fn semantic_error(cell: &str, document: &Value) -> ClaudineError {
    match load_user(document).1 {
        Err(error @ ClaudineError::ConfigInvalid { .. }) => panic!("{cell}: expected a semantic error, got {error}"),
        Err(error) => error,
        Ok(_) => panic!("{cell}: expected a semantic error, loaded"),
    }
}

#[test]
fn the_control_loads_every_setting_as_written() {
    let config = loads("control", &user_control());
    assert!(!config.logging);
    assert!(!config.prompt_for_missing && config.harvest_unmatched);
    let TtsValue::Config(tts) = &config.tts else { panic!("tts object") };
    assert!(matches!(tts.voice, Some(VoiceSelection::Gendered { .. })));
    assert_eq!(tts.prewarm, Some(false));
    let messenger = config.messenger.as_ref().unwrap();
    assert_eq!(messenger.configurations.len(), 6);
    assert!(matches!(
        &messenger.configurations["alerts"],
        MessengerProviderConfig::Discord { bot_token_env, .. } if bot_token_env == "DISCORD_BOT_TOKEN"
    ));
    assert!(matches!(messenger.configurations["hook"], MessengerProviderConfig::DiscordWebhook { .. }));
    assert_eq!(config.actions.values().map(Vec::len).sum::<usize>(), 6);
    assert_eq!(config.protect.custom_patterns.len(), 2);
    assert_eq!(config.models.len(), 2);
    assert_eq!(config.guard_settings.repetition.max_repeats, 10);
    assert_eq!(config.steering.automatic.enabled, Some(false));
    assert!(config.permissions.is_some(), "permissions are kept as written");
}

#[test]
fn an_empty_file_is_not_a_mapping_and_an_explicit_empty_mapping_takes_every_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    std::fs::write(&path, "").unwrap();
    assert_located("empty file", load_claudine_config(Some(&path), None), "");
    std::fs::write(&path, "{}\n").unwrap();
    let config = load_claudine_config(Some(&path), None).unwrap();
    let defaults = ClaudineConfig::default();
    assert_eq!(serde_json::to_value(&config).unwrap(), serde_json::to_value(&defaults).unwrap());
    assert!(config.logging && config.prompt_for_missing && !config.harvest_unmatched);
    assert!(config.protect.enabled && config.actions.is_empty() && config.models.is_empty());
    assert!(matches!(config.tts, TtsValue::Boolean(false)));
    assert_eq!(config.guard_settings, claudine::runaway::GuardSettings::default(), "schema and built-in guards agree");
    assert!(config.permissions.is_none(), "Darkmatter's defaults are never inserted");
}

/// Every top-level setting: a value of the wrong type is refused at its own
/// path.
#[test]
fn every_top_level_setting_is_wired_to_its_validator() {
    let wrong: &[(&str, Value)] = &[
        ("tts", json!("yes")),
        ("messenger", json!(5)),
        ("logging", json!("yes")),
        ("protect", json!("on")),
        ("actions", json!([])),
        ("matchers", json!([])),
        ("preferred_agent", json!("nope")),
        ("canonical_provider", json!(1)),
        ("models", json!([])),
        ("default_sounds", json!("x")),
        ("prompt_for_missing", json!("true")),
        ("harvest_unmatched", json!(1)),
        ("exit_expressions", json!("x")),
        ("guard_settings", json!([])),
        ("steering", json!(false)),
        ("permissions", json!(5)),
    ];
    for (key, value) in wrong {
        rejected(&format!("{key} wrong type"), &format!("/{key}"), Some(value.clone()));
    }
    rejected("favorite_agent wrong type", "/favorite_agent", Some(json!(1)));
    rejected("unknown key", "/volume", Some(json!(1)));
}

#[test]
fn every_absent_top_level_setting_takes_its_schema_default() {
    let defaults = serde_json::to_value(ClaudineConfig::default()).unwrap();
    let keys = user_control().as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    for key in keys {
        let absent = serde_json::to_value(loads(&format!("{key} absent"), &edited(&user_control(), &format!("/{key}"), None))).unwrap();
        assert_eq!(absent.get(&key), defaults.get(&key), "{key} absent takes its default");
    }
}

#[test]
fn the_former_preferred_agent_name_is_read_and_both_names_are_refused() {
    let document = edited(&edited(&user_control(), "/preferred_agent", None), "/favorite_agent", Some(json!("gemini")));
    assert_eq!(loads("favorite_agent", &document).preferred_agent, Some(claudine::provider::Provider::Gemini));
    rejected("both names", "/favorite_agent", Some(json!("gemini")));
}

/// Explicit `null`: an optional setting reads as absent; a setting with a
/// default refuses it.
#[test]
fn null_is_absent_only_for_optional_settings() {
    for key in ["messenger", "preferred_agent", "canonical_provider", "exit_expressions"] {
        let config = loads(key, &edited(&user_control(), &format!("/{key}"), Some(Value::Null)));
        let value = serde_json::to_value(&config).unwrap();
        assert!(value.get(key).is_none(), "{key}: null reads as absent");
    }
    for key in [
        "tts", "logging", "protect", "actions", "matchers", "models", "default_sounds", "prompt_for_missing",
        "harvest_unmatched", "guard_settings", "steering", "permissions",
    ] {
        rejected(&format!("{key} null"), &format!("/{key}"), Some(Value::Null));
    }
}

/// The shared validators, walked once each: absent, null, wrong whole type,
/// empty; and for collections one wrong element and all wrong elements.
#[test]
fn shared_validators_take_the_full_shape_walk() {
    // Numeric (integer, minimum): no coercion of a quoted number.
    let at = "/guard_settings/repetition/max_repeats";
    let absent = loads("number absent", &edited(&user_control(), at, None));
    assert_eq!(absent.guard_settings.repetition.max_repeats, 30, "the schema default fills a written parent");
    for (cell, value) in [("null", json!(null)), ("string", json!("30")), ("negative", json!(-1)), ("fraction", json!(1.5))] {
        rejected(&format!("number {cell}"), at, Some(value));
    }
    // Enum: empty and unknown members.
    for value in [json!(""), json!("nope")] {
        rejected("enum member", "/preferred_agent", Some(value));
    }
    // Mapping: an empty one is explicit.
    assert!(loads("mapping empty", &edited(&user_control(), "/default_sounds", Some(json!({})))).default_sounds.success.is_none());
    // Sequence (`action[]`): empty is explicit, never absent.
    let at = "/actions/session_start";
    let empty = loads("sequence empty", &edited(&user_control(), at, Some(json!([]))));
    assert_eq!(empty.actions[&claudine::events::AgenticEvent::SessionStart].len(), 0);
    rejected("sequence null", at, Some(Value::Null));
    rejected("sequence wrong type", at, Some(json!({})));
    rejected("sequence one wrong element", "/actions/session_start/1", Some(json!(5)));
    rejected("sequence all wrong", at, Some(json!([1, "x"])));
    // Dictionary (`<string>` routes).
    let at = "/messenger/configurations";
    rejected("dictionary one wrong element", "/messenger/configurations/team", Some(json!("slack")));
    rejected("dictionary null", at, Some(Value::Null));
    let empty = edited(&edited(&user_control(), at, Some(json!({}))), "/messenger/active_config", None);
    assert!(loads("dictionary empty", &empty).messenger.unwrap().configurations.is_empty());
    // A list of strings.
    rejected("string list element", "/actions/before_tool/1/args/0", Some(json!(1)));
}

#[test]
fn nested_variants_keep_their_accepted_shapes() {
    // Voice selections.
    let single = loads("voice single", &edited(&user_control(), "/tts/voice", Some(json!("Alex"))));
    assert!(matches!(&single.tts, TtsValue::Config(tts) if tts.voice == Some(VoiceSelection::Single("Alex".into()))));
    rejected_within("voice half pair", "/tts/voice", Some(json!({ "male": "Daniel" })), "/tts");
    rejected_within("tts provider null", "/tts/provider", Some(Value::Null), "/tts");
    let defaulted = loads("tts gender absent", &edited(&user_control(), "/tts/gender", None));
    assert!(matches!(&defaulted.tts, TtsValue::Config(tts) if tts.gender == claudine::config::Gender::Female));
    // Messenger routes.
    rejected_within("route unknown provider", "/messenger/configurations/alerts/provider", Some(json!("telegram")), "/messenger/configurations/alerts");
    rejected_within("route missing field", "/messenger/configurations/alerts/channel_id", None, "/messenger/configurations/alerts");
    rejected_within("route foreign field", "/messenger/configurations/alerts/recipient", Some(json!("x")), "/messenger/configurations/alerts");
    // Protect rules and custom patterns.
    for (cell, pointer, value) in [
        ("unknown rule group", "/protect/rules/everything", Some(json!(false))),
        ("rule group wrong type", "/protect/rules/git_destructive", Some(json!("off"))),
        ("custom pattern without pattern", "/protect/custom_patterns/0/pattern", None),
        ("custom pattern surface", "/protect/custom_patterns/1/surface", Some(json!("anywhere"))),
    ] {
        rejected_within(cell, pointer, value, "/protect");
    }
    assert!(!loads("protect shorthand", &edited(&user_control(), "/protect", Some(json!(false)))).protect.enabled);
}

#[test]
fn action_and_model_variants_keep_their_accepted_shapes() {
    // Event action variants and mappers.
    let sound = loads("sound defaults", &user_control());
    let claudine::actions::HookAction::SoundEffect { volume, speed, .. } =
        &sound.actions[&claudine::events::AgenticEvent::SessionStart][0]
    else {
        panic!("sound effect")
    };
    assert_eq!((*volume, *speed), (1.0, 1.0), "action defaults come from the schema");
    rejected_within("unknown action type", "/actions/session_start/0/type", Some(json!("shout")), "/actions/session_start/0");
    rejected_within("action missing field", "/actions/session_start/0/effect", None, "/actions/session_start/0");
    rejected("unknown event", "/actions/at_lunch", Some(json!([])));
    for mapper in [json!({ "type": "json_object" }), json!({ "type": "exit_code" }), json!({ "type": "regex", "pattern": "(?P<decision>\\w+)" })] {
        loads("mapper variant", &edited(&user_control(), "/actions/before_tool/1/mapper", Some(mapper)));
    }
    rejected_within("mapper unknown type", "/actions/before_tool/1/mapper/type", Some(json!("xml")), "/actions/before_tool/1");
    rejected_within("timeout negative", "/actions/before_tool/1/timeout_ms", Some(json!(-5)), "/actions/before_tool/1");
    // Model overrides.
    let models = loads("model mode absent", &edited(&user_control(), "/models/claude/mode", None));
    assert_eq!(models.models[&claudine::provider::Provider::Claude].mode(), ModelOverrideMode::Add);
    rejected_within("model mode", "/models/claude/mode", Some(json!("merge")), "/models/claude");
    rejected("model unknown provider", "/models/copilot", Some(json!(["x"])));
    rejected_within("model entry without id", "/models/claude/values/1/id", None, "/models/claude");
}

#[test]
fn sound_rule_guard_and_steering_paths_are_typed() {
    // Sound categories.
    rejected("sound wrong type", "/default_sounds/success", Some(json!(1)));
    // Exit-expression rules.
    rejected_within("rule with both", "/exit_expressions/rules/0/patterns", Some(json!(["x"])), "/exit_expressions/rules/0");
    rejected_within("rule with neither", "/exit_expressions/rules/0/pattern", None, "/exit_expressions/rules/0");
    rejected_within("rule kind", "/exit_expressions/rules/1/kind", Some(json!("glob")), "/exit_expressions");
    let array = loads("rules array form", &edited(&user_control(), "/exit_expressions", Some(json!([{ "pattern": "X" }]))));
    assert!(array.exit_expressions.unwrap().explicit_mode().is_none());
    // Repetition and volume settings.
    rejected("volume bytes", "/guard_settings/volume/max_bytes", Some(json!("1MB")));
    rejected("repetition unknown", "/guard_settings/repetition/posture", Some(json!("strict")));
    // Steering.
    rejected("steering enabled null", "/steering/automatic/enabled", Some(Value::Null));
    rejected("steering automatic null", "/steering/automatic", Some(Value::Null));
}

#[test]
fn semantic_constraints_remain_errors_after_the_schema_accepts_the_shape() {
    let control = user_control();
    semantic_error("unknown sound", &edited(&control, "/default_sounds/success", Some(json!("no-such-sound"))));
    semantic_error("invalid custom regex", &edited(&control, "/protect/custom_patterns/0/pattern", Some(json!("("))));
    semantic_error("write_path surface", &edited(&control, "/protect/custom_patterns/1/surface", Some(json!("write_path"))));
    let detailed = json!({ "enabled": true, "allow_paths": ["/x"] });
    semantic_error("allow_paths on a group without them", &edited(&control, "/protect/rules/git_destructive", Some(detailed)));
    semantic_error("inactive route", &edited(&control, "/messenger/active_config", Some(json!("nowhere"))));
    semantic_error("invalid exit regex", &edited(&control, "/exit_expressions/rules/1/patterns/0", Some(json!("["))));
    semantic_error("unknown scope agent", &edited(&control, "/exit_expressions/rules/1/scope", Some(json!("nonsense/k2"))));
}

#[test]
fn a_repository_file_overrides_only_what_it_writes() {
    let (_dir, merged) = load_with_repository(&user_control(), &repository_control());
    let merged = merged.unwrap();
    assert_eq!(merged.canonical_provider, Some(claudine::provider::Provider::Gemini));
    assert_eq!(merged.messenger.as_ref().unwrap().active_config.as_deref(), Some("team"));
    assert_eq!(merged.preferred_agent, Some(claudine::provider::Provider::Codex), "user-only setting kept");
    assert!(!merged.guard_settings.repetition.enabled);
    assert_eq!(merged.guard_settings.repetition.max_repeats, 30, "a written repository parent takes its child defaults");

    // `active_messenger`: absent inherits, null disables, an unknown route fails.
    let (_dir, absent) = load_with_repository(&user_control(), &edited(&repository_control(), "/active_messenger", None));
    assert_eq!(absent.unwrap().messenger.unwrap().active_config.as_deref(), Some("alerts"));
    let (_dir, null) = load_with_repository(&user_control(), &edited(&repository_control(), "/active_messenger", Some(Value::Null)));
    assert_eq!(null.unwrap().messenger.unwrap().active_config, None);
    let (_dir, unknown) =
        load_with_repository(&user_control(), &edited(&repository_control(), "/active_messenger", Some(json!("nowhere"))));
    assert!(matches!(unknown, Err(ClaudineError::ConfigValidation(_))), "{unknown:?}");

    // Inheritance fields are never inserted.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claudine/config.yaml");
    write_yaml(&path, &json!({}));
    let empty = load_repo_override_config(&path).unwrap().unwrap();
    assert!(empty.is_empty(), "{empty:?}");
    write_yaml(&path, &json!({ "guard_settings": null }));
    assert!(load_repo_override_config(&path).unwrap().unwrap().guard_settings.is_none(), "null inherits");
}

#[test]
fn a_repository_file_accepts_only_its_override_keys() {
    for (key, value) in [
        ("tts", json!(true)),
        ("logging", json!(true)),
        ("protect", json!(true)),
        ("preferred_agent", json!("claude")),
        ("models", json!({})),
        ("default_sounds", json!({})),
        ("prompt_for_missing", json!(true)),
        ("harvest_unmatched", json!(true)),
        ("messenger", json!({})),
    ] {
        let (_dir, result) = load_with_repository(&user_control(), &edited(&repository_control(), &format!("/{key}"), Some(value)));
        assert_located(&format!("repository {key}"), result, &format!("/{key}"));
    }
    for (key, value) in [("steering", Value::Null), ("actions", Value::Null), ("guard_settings", json!(5)), ("canonical_provider", json!("nope"))] {
        let (_dir, result) = load_with_repository(&user_control(), &edited(&repository_control(), &format!("/{key}"), Some(value)));
        assert_located(&format!("repository {key}"), result, &format!("/{key}"));
    }
    let (_dir, scope) = load_with_repository(
        &user_control(),
        &edited(&repository_control(), "/exit_expressions", Some(json!([{ "pattern": "x", "scope": "nonsense" }]))),
    );
    assert!(matches!(scope, Err(ClaudineError::ConfigValidation(_))), "provider scope stays a semantic error");
}

/// The schema's enums list exactly the Rust enums' serde names.
#[test]
fn the_schema_names_every_provider_and_event() {
    let schema = yaml(claudine::config::schema::CONFIG_SCHEMA_SOURCE);
    let types = &schema["types"];
    let providers: Vec<String> = claudine::provider::PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(|provider| serde_json::to_value(provider).unwrap().as_str().unwrap().to_string())
        .collect();
    let mut declared: Vec<&str> = types["model-overrides"].as_object().unwrap().keys().map(String::as_str).collect();
    let mut expected: Vec<&str> = providers.iter().map(String::as_str).collect();
    declared.sort_unstable();
    expected.sort_unstable();
    assert_eq!(declared, expected);
    for provider in &providers {
        assert!(types["provider"].as_str().unwrap().contains(provider.as_str()), "{provider}");
    }
    let events: Vec<&str> = types["event-actions"].as_object().unwrap().keys().map(String::as_str).collect();
    let matchers: Vec<&str> = types["event-matchers"].as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(events, matchers);
    for event in events {
        let parsed: claudine::events::AgenticEvent = serde_json::from_value(json!(event)).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), json!(event));
    }
}

/// A located problem names the file.
#[test]
fn problems_name_the_file_line_and_pointer() {
    let dir = tempfile::tempdir().unwrap();
    let path: PathBuf = dir.path().join("config.yaml");
    std::fs::write(&path, "# mine\nlogging: true\nprotect:\n  enabled: maybe\n").unwrap();
    let error = load_claudine_config(Some(&path), None).unwrap_err();
    let text = error.to_string();
    assert!(text.contains(&format!("{}:4:", path.display())), "{text}");
    assert!(text.contains("/protect"), "{text}");
}
