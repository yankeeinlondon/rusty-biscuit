//! The keyed action/route spelling and the null-as-unset readers, through
//! the public document entry points ([`ClaudineConfig::from_user_document`],
//! [`RepoOverrideConfig::from_repository_document`]) and, for what only the
//! adapters can see, serde itself.

use serde_json::{Value, json};

use super::*;
use crate::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};
use crate::config::load::ConfigProblemKind;
use crate::config::messaging_block::ClaudineMessengerConfig;
use crate::config::tts::TtsValue;
use crate::error::ClaudineError;

const USER_CONTROL: &str = include_str!("../../tests/config/user-control.yaml");

fn user_control() -> Value {
    biscuit_file::serde_yaml_ng::from_str(USER_CONTROL).expect("the control is YAML")
}

fn user(document: Value) -> Result<ClaudineConfig, ClaudineError> {
    ClaudineConfig::from_user_document(document)
}

fn repository(document: Value) -> Result<RepoOverrideConfig, ClaudineError> {
    RepoOverrideConfig::from_repository_document(document)
}

/// The problems of a refused document, as `(pointer, message)`.
fn problems<T: std::fmt::Debug>(cell: &str, result: Result<T, ClaudineError>) -> Vec<(String, String)> {
    match result {
        Err(ClaudineError::ConfigInvalid { problems }) => {
            problems.into_iter().map(|problem| (problem.pointer, problem.message)).collect()
        }
        other => panic!("{cell}: expected ConfigInvalid, got {other:?}"),
    }
}

/// Asserts the document is refused with a problem at or below `pointer`.
fn refused_at<T: std::fmt::Debug>(cell: &str, result: Result<T, ClaudineError>, pointer: &str) -> Vec<(String, String)> {
    let found = problems(cell, result);
    assert!(
        found.iter().any(|(at, _)| at == pointer || at.starts_with(&format!("{pointer}/"))),
        "{cell}: no problem at {pointer}: {found:?}"
    );
    found
}

fn with_action(action: Value) -> Value {
    json!({ "actions": { "session_start": [action] } })
}

fn with_route(route: Value) -> Value {
    json!({ "messenger": { "configurations": { "alerts": route } } })
}

#[test]
fn the_control_reads_keyed_actions_and_routes_with_their_payload_defaults() {
    let config = user(user_control()).expect("the control is valid");
    let start = &config.actions[&AgenticEvent::SessionStart];
    assert_eq!(
        start[0],
        HookAction::SoundEffect { effect: "doorbell".into(), volume: 1.0, speed: 1.0, when: None },
        "volume and speed take their defaults inside `sound_effect`"
    );
    let routes = &config.messenger.as_ref().expect("messenger").configurations;
    assert_eq!(
        routes["alerts"],
        MessengerProviderConfig::Discord { channel_id: "123".into(), bot_token_env: "DISCORD_BOT_TOKEN".into() }
    );
    assert_eq!(
        routes["hook"],
        MessengerProviderConfig::DiscordWebhook { webhook_url: None, webhook_url_env: "MY_DISCORD_URL".into() }
    );
    assert_eq!(
        routes["slackhook"],
        MessengerProviderConfig::SlackWebhook {
            webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXX".into()),
            webhook_url_env: "SLACK_WEBHOOK_URL".into(),
        }
    );
}

#[test]
fn each_action_kind_reads_its_payload_and_refuses_one_invalid_field() {
    let cells: [(&str, Value, HookAction, Value); 6] = [
        (
            "sound_effect",
            json!({ "sound_effect": { "effect": "doorbell", "volume": null } }),
            HookAction::SoundEffect { effect: "doorbell".into(), volume: 1.0, speed: 1.0, when: None },
            json!({ "sound_effect": { "effect": "doorbell", "volume": "loud" } }),
        ),
        (
            "speak",
            json!({ "speak": { "message": "hi", "voice": null } }),
            HookAction::Speak { message: "hi".into(), voice: None, gender: None, when: None },
            json!({ "speak": { "message": "hi", "gender": "robot" } }),
        ),
        (
            "bash",
            json!({ "bash": { "command": "echo" } }),
            HookAction::Bash { command: "echo".into(), params: String::new(), when: None },
            json!({ "bash": { "command": 7 } }),
        ),
        (
            "call",
            json!({ "call": { "command": "./check.sh", "timeout_ms": 10 } }),
            HookAction::Call { command: "./check.sh".into(), args: None, timeout_ms: Some(10), mapper: None, when: None },
            json!({ "call": { "command": "./check.sh", "timeout_ms": -1 } }),
        ),
        (
            "report",
            json!({ "report": {} }),
            HookAction::Report { handler: None, when: None },
            json!({ "report": { "handler": { "format": "yaml" } } }),
        ),
        (
            "message",
            json!({ "message": { "message": "done" } }),
            HookAction::Message { message: "done".into(), image: None, when: None },
            json!({ "message": { "message": "done", "image": [] } }),
        ),
    ];
    for (kind, valid, expected, invalid) in cells {
        let config = user(with_action(valid)).unwrap_or_else(|error| panic!("{kind}: {error}"));
        assert_eq!(config.actions[&AgenticEvent::SessionStart], vec![expected], "{kind}");
        refused_at(kind, user(with_action(invalid)), &format!("/actions/session_start/0/{kind}"));
    }
}

#[test]
fn each_route_kind_reads_its_payload_and_refuses_one_invalid_field() {
    let cells: [(&str, Value, MessengerProviderConfig, Value); 6] = [
        (
            "discord",
            json!({ "discord": { "channel_id": "1", "bot_token_env": null } }),
            MessengerProviderConfig::Discord { channel_id: "1".into(), bot_token_env: "DISCORD_BOT_TOKEN".into() },
            json!({ "discord": {} }),
        ),
        (
            "slack",
            json!({ "slack": { "channel_id": "C1" } }),
            MessengerProviderConfig::Slack { channel_id: "C1".into(), bot_token_env: "SLACK_BOT_TOKEN".into() },
            json!({ "slack": { "channel_id": 1 } }),
        ),
        (
            "signal",
            json!({ "signal": { "recipient": "+1" } }),
            MessengerProviderConfig::Signal {
                recipient: "+1".into(),
                rpc_url_env: "SIGNAL_RPC_URL".into(),
                account_env: "SIGNAL_ACCOUNT".into(),
            },
            json!({ "signal": { "recipient": "+1", "channel_id": "x" } }),
        ),
        (
            "whatsapp",
            json!({ "whatsapp": { "recipient": "+1" } }),
            MessengerProviderConfig::Whatsapp {
                recipient: "+1".into(),
                access_token_env: "WHATSAPP_ACCESS_TOKEN".into(),
                phone_number_id_env: "WHATSAPP_PHONE_NUMBER_ID".into(),
            },
            json!({ "whatsapp": { "recipient": ["+1"] } }),
        ),
        (
            "discord_webhook",
            json!({ "discord_webhook": {} }),
            MessengerProviderConfig::DiscordWebhook { webhook_url: None, webhook_url_env: "DISCORD_WEBHOOK_URL".into() },
            json!({ "discord_webhook": { "webhook_url": 5 } }),
        ),
        (
            "slack_webhook",
            json!({ "slack_webhook": { "webhook_url": null } }),
            MessengerProviderConfig::SlackWebhook { webhook_url: None, webhook_url_env: "SLACK_WEBHOOK_URL".into() },
            json!({ "slack_webhook": { "webhook_url_env": false } }),
        ),
    ];
    for (kind, valid, expected, invalid) in cells {
        let config = user(with_route(valid)).unwrap_or_else(|error| panic!("{kind}: {error}"));
        assert_eq!(config.messenger.expect("messenger").configurations["alerts"], expected, "{kind}");
        refused_at(kind, user(with_route(invalid)), &format!("/messenger/configurations/alerts/{kind}"));
    }
}

#[test]
fn a_keyed_entry_needs_exactly_one_known_kind_holding_a_mapping() {
    let cells = [
        ("null entry", Value::Null),
        ("scalar entry", json!("bash")),
        ("list entry", json!([{ "bash": { "command": "echo" } }])),
        ("zero keys", json!({})),
        ("two keys", json!({ "bash": { "command": "echo" }, "speak": { "message": "hi" } })),
        ("null payload", json!({ "bash": null })),
        ("scalar payload", json!({ "bash": "echo" })),
        ("list payload", json!({ "bash": ["echo"] })),
        ("unknown kind", json!({ "shout": { "message": "hi" } })),
        ("tag inside the payload", json!({ "bash": { "type": "bash", "command": "echo" } })),
    ];
    for (cell, entry) in cells {
        refused_at(&format!("action {cell}"), user(with_action(entry.clone())), "/actions/session_start/0");
        refused_at(&format!("route {cell}"), user(with_route(entry)), "/messenger/configurations/alerts");
    }
}

#[test]
fn the_adapters_refuse_what_the_schema_would_and_say_why() {
    let route = |entry: Value| serde_json::from_value::<ClaudineMessengerConfig>(json!({ "configurations": { "alerts": entry } }));
    let message = |entry: Value| route(entry).expect_err("refused").to_string();
    assert!(message(json!({})).contains("names no kind"), "{}", message(json!({})));
    assert!(message(json!({ "discord": { "channel_id": "1" }, "slack": { "channel_id": "2" } })).contains("both written"));
    assert!(message(json!({ "discord": null })).contains("`discord` needs its settings"));
    assert!(message(json!({ "discord": "1" })).contains("must be a mapping"));
    assert!(message(json!("discord")).contains("one key naming its kind"));
    assert!(message(json!({ "telegram": {} })).contains("`telegram` is not a messenger route"));
    assert!(message(Value::Null).contains("alerts"), "a null route is refused, never skipped");
    let flat = message(json!({ "provider": "discord", "channel_id": "1" }));
    assert!(flat.contains("discord: { channel_id: \"1\" }"), "{flat}");
}

#[test]
fn a_flat_action_or_route_is_refused_with_its_keyed_spelling() {
    let found = refused_at(
        "flat action",
        user(with_action(json!({ "type": "bash", "command": "echo", "when": "x" }))),
        "/actions/session_start/0",
    );
    assert_eq!(found.len(), 1, "the migration message replaces the union's report: {found:?}");
    assert!(found[0].1.contains("`{ bash: { command: \"echo\", when: \"x\" } }`"), "{found:?}");

    let found = refused_at(
        "flat route",
        user(with_route(json!({ "provider": "discord-webhook", "webhook_url_env": "U" }))),
        "/messenger/configurations/alerts",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.contains("`{ discord_webhook: { webhook_url_env: \"U\" } }`"), "{found:?}");

    let found = refused_at(
        "flat repository action",
        repository(json!({ "actions": { "session_end": [{ "type": "report" }] } })),
        "/actions/session_end/0",
    );
    assert!(found[0].1.contains("`{ report: {} }`"), "{found:?}");
}

#[test]
fn a_flat_entry_keeps_its_other_problems_beside_the_migration_message() {
    let kinds = |result: Result<ClaudineConfig, ClaudineError>| match result {
        Err(ClaudineError::ConfigInvalid { problems }) => {
            let mut found: Vec<(String, ConfigProblemKind)> = problems.into_iter().map(|problem| (problem.pointer, problem.kind)).collect();
            found.sort_by(|left, right| left.0.cmp(&right.0));
            found
        }
        other => panic!("expected ConfigInvalid, got {other:?}"),
    };
    let entry = "/actions/session_start/0".to_string();
    assert_eq!(
        kinds(user(with_action(json!({ "type": "bash", "command": "echo", "timout": 5 })))),
        vec![(entry.clone(), ConfigProblemKind::InvalidSetting), (format!("{entry}/timout"), ConfigProblemKind::UnknownKey)],
        "the typo is named at the key written, and the tag is not stray"
    );
    assert_eq!(
        kinds(user(with_action(json!({ "type": "call", "command": "c", "timeout_ms": "soon" })))),
        vec![(entry.clone(), ConfigProblemKind::InvalidSetting), (format!("{entry}/timeout_ms"), ConfigProblemKind::InvalidSetting)],
    );
    let route = "/messenger/configurations/alerts".to_string();
    assert_eq!(
        kinds(user(with_route(json!({ "provider": "slack", "channel_id": "C1", "timout": 5 })))),
        vec![(route.clone(), ConfigProblemKind::InvalidSetting), (format!("{route}/timout"), ConfigProblemKind::UnknownKey)],
    );
}

#[test]
fn upgrading_rewrites_only_flat_entries_of_known_kinds() {
    let mut document = json!({
        "actions": {
            "before_tool": [
                { "type": "call", "command": "c", "mapper": { "type": "json_field", "field": "f" } },
                { "type": "shout" },
                { "bash": { "command": "echo" } }
            ]
        },
        "messenger": { "configurations": { "hook": { "provider": "slack-webhook" } } }
    });
    upgrade_flat(&mut document);
    assert_eq!(
        document,
        json!({
            "actions": {
                "before_tool": [
                    { "call": { "command": "c", "mapper": { "type": "json_field", "field": "f" } } },
                    { "type": "shout" },
                    { "bash": { "command": "echo" } }
                ]
            },
            "messenger": { "configurations": { "hook": { "slack_webhook": {} } } }
        })
    );
}

#[test]
fn the_configuration_spelling_is_keyed_and_other_serialization_stays_tagged() {
    let config = user(user_control()).expect("valid");
    let written = serde_json::to_value(&config).expect("serializes");
    assert_eq!(written["actions"]["session_start"][0], json!({ "sound_effect": { "effect": "doorbell", "volume": 1.0, "speed": 1.0 } }));
    assert_eq!(written["messenger"]["configurations"]["alerts"], json!({ "discord": { "channel_id": "123", "bot_token_env": "DISCORD_BOT_TOKEN" } }));
    let reread = user(written.clone()).expect("the written spelling reads back");
    assert_eq!(serde_json::to_value(&reread).expect("serializes"), written, "read/write/read is stable");

    let action = &config.actions[&AgenticEvent::SessionStart][0];
    assert_eq!(serde_json::to_value(action).expect("serializes")["type"], json!("sound_effect"));
    let route = &config.messenger.as_ref().expect("messenger").configurations["alerts"];
    assert_eq!(serde_json::to_value(route).expect("serializes")["provider"], json!("discord"));
}

#[test]
fn null_and_empty_user_collections_read_as_unset_or_take_their_defaults() {
    for (cell, document) in [
        ("absent", json!({})),
        ("null actions", json!({ "actions": null })),
        ("empty actions", json!({ "actions": {} })),
        ("null event list", json!({ "actions": { "session_start": null } })),
    ] {
        let config = user(document).unwrap_or_else(|error| panic!("{cell}: {error}"));
        assert!(config.actions.is_empty(), "{cell}");
    }
    refused_at("empty event list", user(json!({ "actions": { "session_start": [] } })), "/actions/session_start");
    refused_at("wrong actions type", user(json!({ "actions": ["bash"] })), "/actions");
    refused_at("wrong event list type", user(json!({ "actions": { "session_start": { "bash": {} } } })), "/actions/session_start");

    for document in [json!({ "matchers": null }), json!({ "matchers": { "before_tool": null } })] {
        assert!(user(document.clone()).expect("valid").matchers.is_empty(), "{document}");
    }
    refused_at("empty matcher", user(json!({ "matchers": { "before_tool": "" } })), "/matchers/before_tool");

    for document in [json!({ "models": null }), json!({ "models": { "codex": null } })] {
        assert!(user(document.clone()).expect("valid").models.is_empty(), "{document}");
    }
    refused_at("wrong model override", user(json!({ "models": { "codex": 3 } })), "/models/codex");
}

#[test]
fn user_steering_and_tts_take_their_newly_active_defaults() {
    for document in [
        json!({}),
        json!({ "steering": null }),
        json!({ "steering": {} }),
        json!({ "steering": { "automatic": null } }),
        json!({ "steering": { "automatic": {} } }),
        json!({ "steering": { "automatic": { "enabled": null } } }),
    ] {
        assert_eq!(user(document.clone()).expect("valid").steering.automatic.enabled, Some(true), "{document}");
    }
    let off = user(json!({ "steering": { "automatic": { "enabled": false } } })).expect("valid");
    assert_eq!(off.steering.automatic.enabled, Some(false));
    refused_at("wrong steering type", user(json!({ "steering": { "automatic": { "enabled": "yes" } } })), "/steering/automatic/enabled");
    refused_at("unknown steering key", user(json!({ "steering": { "automatic": { "on": true } } })), "/steering/automatic/on");

    for document in [json!({ "tts": { "provider": "say" } }), json!({ "tts": { "provider": null, "prewarm": null } })] {
        let TtsValue::Config(settings) = user(document.clone()).expect("valid").tts else {
            panic!("{document}: the object form");
        };
        assert_eq!(settings.prewarm, Some(true), "{document}");
    }
    let TtsValue::Config(settings) = user(json!({ "tts": { "provider": null } })).expect("valid").tts else {
        panic!("the object form");
    };
    assert_eq!(settings.provider, None, "a null provider auto-detects, as an absent one does");
}

#[test]
fn null_repository_overrides_inherit_and_written_ones_override() {
    for (cell, document) in [
        ("absent", json!({})),
        ("null actions", json!({ "actions": null })),
        ("null event list", json!({ "actions": { "session_end": null } })),
        ("null matchers", json!({ "matchers": null })),
        ("null matcher", json!({ "matchers": { "session_end": null } })),
        ("null messenger", json!({ "active_messenger": null })),
        ("null steering", json!({ "steering": null })),
        ("null automatic", json!({ "steering": { "automatic": null } })),
        ("null enabled", json!({ "steering": { "automatic": { "enabled": null } } })),
        ("null guards", json!({ "guard_settings": null, "exit_expressions": null, "canonical_provider": null })),
    ] {
        let repo = repository(document).unwrap_or_else(|error| panic!("{cell}: {error}"));
        assert!(repo.is_empty(), "{cell}: {repo:?}");
    }
    let repo = repository(json!({
        "actions": { "session_end": [{ "report": {} }] },
        "matchers": { "session_end": "true" },
        "active_messenger": "team",
        "steering": { "automatic": { "enabled": false } }
    }))
    .expect("valid");
    assert_eq!(repo.actions[&AgenticEvent::SessionEnd], vec![HookAction::Report { handler: None, when: None }]);
    assert_eq!(repo.matchers[&AgenticEvent::SessionEnd], "true");
    assert_eq!(repo.active_messenger.as_deref(), Some("team"));
    assert_eq!(repo.steering.automatic.enabled, Some(false));

    refused_at("empty actions override", repository(json!({ "actions": {} })), "/actions");
    refused_at("empty steering override", repository(json!({ "steering": {} })), "/steering");
    refused_at("empty automatic override", repository(json!({ "steering": { "automatic": {} } })), "/steering/automatic");
    refused_at("wrong messenger type", repository(json!({ "active_messenger": false })), "/active_messenger");
    refused_at("unknown repository key", repository(json!({ "tts": true })), "/tts");
}
