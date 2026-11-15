//! Every keyed action and messenger route through the public file loader
//! (`load_claudine_config`): one test per kind, generated from one table, with
//! a positive control (the kind's entry in `tests/config/user-control.yaml`,
//! as written) and one invalid field (a single mutation of that entry) that
//! fails with a problem located in the file at the kind's payload.
//!
//! The shared wrapper checks (null, scalar, zero keys, two keys, unknown
//! kind) are run once in `src/config/wire_tests.rs`.

use std::path::Path;

use claudine::config::claudine_config::ClaudineConfig;
use claudine::config::load::read_document;
use claudine::config::schema::{CONFIG_SCHEMA_SOURCE, ConfigScope};
use claudine::dispatch::loader::load_claudine_config;
use claudine::error::ClaudineError;
use claudine::events::AgenticEvent;
use serde_json::{Value, json};

fn yaml(text: &str) -> Value {
    biscuit_file::serde_yaml_ng::from_str(text).expect("fixture is YAML")
}

fn user_control() -> Value {
    yaml(include_str!("../config/user-control.yaml"))
}

fn write_yaml(path: &Path, document: &Value) {
    std::fs::write(path, biscuit_file::serde_yaml_ng::to_string(document).unwrap()).unwrap();
}

fn load(document: &Value) -> Result<ClaudineConfig, ClaudineError> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    write_yaml(&path, document);
    load_claudine_config(Some(&path), None)
}

/// The control with `field` of the payload at `entry/kind` set to `value`.
fn with_field(entry: &str, kind: &str, field: &str, value: Value) -> Value {
    let mut document = user_control();
    let payload = document
        .pointer_mut(&format!("{entry}/{kind}"))
        .and_then(Value::as_object_mut)
        .unwrap_or_else(|| panic!("the control writes `{kind}` at {entry}"));
    payload.insert(field.to_string(), value);
    document
}

/// The load fails with a problem at or below `pointer`, located in the file.
fn assert_refused_at(cell: &str, result: Result<ClaudineConfig, ClaudineError>, pointer: &str) {
    let Err(ClaudineError::ConfigInvalid { problems }) = result else {
        panic!("{cell}: expected a located ConfigInvalid at {pointer}, got {result:?}");
    };
    let problem = problems
        .iter()
        .find(|problem| problem.pointer == pointer || problem.pointer.starts_with(&format!("{pointer}/")))
        .unwrap_or_else(|| panic!("{cell}: no problem at {pointer}: {problems:?}"));
    assert!(problem.file.as_ref().is_some_and(|file| file.ends_with("config.yaml")), "{cell}: {problem:?}");
    assert!(problem.line.is_some_and(|line| line > 1) && problem.column.is_some(), "{cell}: {problem:?}");
}

/// `/actions/<event>/<index>` as its event and index.
fn action_slot(entry: &str) -> (AgenticEvent, usize) {
    let mut parts = entry.trim_start_matches("/actions/").split('/');
    let event = serde_json::from_value(json!(parts.next().unwrap())).expect("an event name");
    (event, parts.next().unwrap().parse().unwrap())
}

fn assert_action(kind: &str, entry: &str, expected: Value, field: &str, invalid: Value) {
    let config = load(&user_control()).unwrap_or_else(|error| panic!("{kind}: the control loads: {error}"));
    let (event, index) = action_slot(entry);
    let read = serde_json::to_value(&config.actions[&event][index]).unwrap();
    assert_eq!(read, expected, "{kind}: the control's entry reads with its payload defaults");

    let invalid = with_field(entry, kind, field, invalid);
    assert_refused_at(kind, load(&invalid), &format!("{entry}/{kind}"));
}

fn assert_route(kind: &str, name: &str, expected: Value, field: &str, invalid: Value) {
    let config = load(&user_control()).unwrap_or_else(|error| panic!("{kind}: the control loads: {error}"));
    let route = &config.messenger.as_ref().expect("messenger").configurations[name];
    assert_eq!(serde_json::to_value(route).unwrap(), expected, "{kind}: the control's route reads with its defaults");

    let entry = format!("/messenger/configurations/{name}");
    let invalid = with_field(&entry, kind, field, invalid);
    assert_refused_at(kind, load(&invalid), &format!("{entry}/{kind}"));
}

/// One `#[test]` per action kind, and `ACTION_KINDS` listing them for the
/// guard below.
macro_rules! action_cells {
    ($($test:ident: $kind:literal at $entry:literal reads $expected:tt; invalid $field:literal = $invalid:tt;)+) => {
        const ACTION_KINDS: &[&str] = &[$($kind),+];
        $(
            #[test]
            fn $test() {
                assert_action($kind, $entry, json!($expected), $field, json!($invalid));
            }
        )+
    };
}

/// One `#[test]` per route kind, and `ROUTE_KINDS` listing them.
macro_rules! route_cells {
    ($($test:ident: $kind:literal at $name:literal reads $expected:tt; invalid $field:literal = $invalid:tt;)+) => {
        const ROUTE_KINDS: &[&str] = &[$($kind),+];
        $(
            #[test]
            fn $test() {
                assert_route($kind, $name, json!($expected), $field, json!($invalid));
            }
        )+
    };
}

action_cells! {
    sound_effect_action: "sound_effect" at "/actions/session_start/0"
        reads { "type": "sound_effect", "effect": "doorbell", "volume": 1.0, "speed": 1.0 };
        invalid "volume" = "loud";
    speak_action: "speak" at "/actions/session_start/1"
        reads { "type": "speak", "message": "Starting {{ tool_name }}", "voice": "Samantha", "gender": "female" };
        invalid "gender" = "robot";
    bash_action: "bash" at "/actions/before_tool/0"
        reads { "type": "bash", "command": "echo", "params": "", "when": "tool_name == 'Bash'" };
        invalid "command" = 7;
    call_action: "call" at "/actions/before_tool/1"
        reads {
            "type": "call",
            "command": "./check.sh",
            "args": ["--strict"],
            "timeout_ms": 5000,
            "mapper": { "type": "json_field", "field": "decision" }
        };
        invalid "timeout_ms" = (-1);
    report_action: "report" at "/actions/turn_complete/0"
        reads { "type": "report", "handler": { "format": "json", "template": null, "include_metadata": false } };
        invalid "handler" = { "format": "yaml" };
    message_action: "message" at "/actions/turn_complete/1"
        reads { "type": "message", "message": "Done" };
        invalid "image" = [];
}

route_cells! {
    discord_route: "discord" at "alerts"
        reads { "provider": "discord", "channel_id": "123", "bot_token_env": "DISCORD_BOT_TOKEN" };
        invalid "channel_id" = 5;
    slack_route: "slack" at "team"
        reads { "provider": "slack", "channel_id": "C01", "bot_token_env": "TEAM_TOKEN" };
        invalid "bot_token_env" = false;
    signal_route: "signal" at "phone"
        reads {
            "provider": "signal",
            "recipient": "+15550100",
            "rpc_url_env": "SIGNAL_RPC_URL",
            "account_env": "SIGNAL_ACCOUNT"
        };
        invalid "channel_id" = "x";
    whatsapp_route: "whatsapp" at "biz"
        reads {
            "provider": "whatsapp",
            "recipient": "+15550101",
            "access_token_env": "WHATSAPP_ACCESS_TOKEN",
            "phone_number_id_env": "WHATSAPP_PHONE_NUMBER_ID"
        };
        invalid "recipient" = ["+15550101"];
    discord_webhook_route: "discord_webhook" at "hook"
        reads { "provider": "discord_webhook", "webhook_url_env": "MY_DISCORD_URL" };
        invalid "webhook_url" = 5;
    slack_webhook_route: "slack_webhook" at "slackhook"
        reads {
            "provider": "slack_webhook",
            "webhook_url": "https://hooks.slack.com/services/T000/B000/XXXX",
            "webhook_url_env": "SLACK_WEBHOOK_URL"
        };
        invalid "webhook_url_env" = false;
}

/// The keys the shipped configuration schema's union `union` admits: each arm
/// is a one-key entry type, and that key is the kind.
fn schema_kinds(union: &str) -> Vec<String> {
    let schema = yaml(CONFIG_SCHEMA_SOURCE);
    let types = &schema["types"];
    let arms = types[union].as_array().unwrap_or_else(|| panic!("`{union}` is a union"));
    let mut kinds: Vec<String> = arms
        .iter()
        .map(|arm| {
            let name = arm.as_str().expect("a type reference").split(" ->").next().unwrap().trim();
            let entry = types[name].as_object().unwrap_or_else(|| panic!("`{name}` is an entry type"));
            let keys: Vec<&String> = entry.keys().filter(|key| !key.starts_with('$')).collect();
            assert_eq!(keys.len(), 1, "`{name}` holds one kind: {keys:?}");
            keys[0].clone()
        })
        .collect();
    kinds.sort();
    kinds
}

fn sorted(kinds: &[&str]) -> Vec<String> {
    let mut kinds: Vec<String> = kinds.iter().map(|kind| (*kind).to_string()).collect();
    kinds.sort();
    kinds
}

/// A kind added to the schema without a cell above fails here.
#[test]
fn every_schema_action_and_route_kind_has_its_own_test() {
    assert_eq!(sorted(ACTION_KINDS), schema_kinds("action"));
    assert_eq!(sorted(ROUTE_KINDS), schema_kinds("messenger-route"));
}

/// The one-key wrappers are judged on the completed document: every default
/// the schema fills sits inside a written payload, so no default adds a
/// second key beside a kind, writes a kind, or creates an entry.
#[test]
fn defaults_fill_only_inside_written_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    write_yaml(&path, &user_control());
    let document = read_document(&path, ConfigScope::User).unwrap();
    for (pointer, kind) in [
        ("/actions/session_start/0", "sound_effect"),
        ("/actions/before_tool/0", "bash"),
        ("/messenger/configurations/alerts", "discord"),
        ("/messenger/configurations/slackhook", "slack_webhook"),
    ] {
        let entry = document.effective.pointer(pointer).and_then(Value::as_object).unwrap();
        assert_eq!(entry.keys().collect::<Vec<_>>(), [kind], "{pointer} keeps its one key once completed");
    }
    // An entry is `actions.<event>.<index>` or `messenger.configurations.<name>`
    // and its payload one level below; a default fills the collection itself
    // (`actions`, `messenger.configurations`) or a field inside a payload.
    let inside_payloads = |document: &claudine::config::load::ConfigDocument| {
        for default in &document.defaults {
            let segments = default.path.segments();
            let collection = match segments {
                [first, ..] if first == "actions" => 1,
                [first, second, ..] if first == "messenger" && second == "configurations" => 2,
                _ => continue,
            };
            assert!(
                segments.len() == collection || segments.len() >= 5,
                "a default at {segments:?} is not inside a written payload"
            );
        }
    };
    inside_payloads(&document);
    assert!(
        document.defaults.iter().any(|default| default.path.segments().join(".") == "actions.session_start.0.sound_effect.volume"),
        "the control's payload defaults are reported"
    );

    // Nothing written: defaults create no entry.
    for text in ["{}\n", "messenger: {}\n", "actions: {}\nmessenger:\n  configurations: {}\n"] {
        std::fs::write(&path, text).unwrap();
        let document = read_document(&path, ConfigScope::User).unwrap();
        inside_payloads(&document);
        assert_eq!(document.effective["actions"], json!({}), "{text}");
        if let Some(messenger) = document.effective.get("messenger") {
            assert_eq!(messenger["configurations"], json!({}), "{text}");
        }
    }
    std::fs::write(&path, "{}\n").unwrap();
    let repository = read_document(&path, ConfigScope::Repository).unwrap();
    assert!(repository.effective.get("actions").is_none(), "a repository override is never created");
}
