use super::*;
use crate::actions::*;
use crate::config::claudine_config::DefaultSounds;
use crate::events::*;
use crate::provider::Provider;

use std::collections::HashMap;

#[test]
fn user_config_path_returns_default_when_none_exists() {
    let path = user_config_path();
    assert!(path.to_string_lossy().contains(".claudine"));
}

// =====================================================================
// ClaudineConfig loading / saving / merging tests
// =====================================================================

#[test]
fn load_claudine_config_from_json5() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".claudine");
    std::fs::create_dir_all(&config_dir).unwrap();

    let json5_content = r#"{
        // TTS auto-detect
        tts: true,
        logging: true,
        protect: true,
        preferred_agent: "claude",
        actions: {
            human_in_the_loop: [
                { type: "sound_effect", effect: "doorbell", },
            ],
        },
    }"#;
    std::fs::write(config_dir.join("config.json"), json5_content).unwrap();

    let config = load_claudine_config(Some(&config_dir.join("config.json")), None).unwrap();
    assert!(config.logging);
    assert!(
        config
            .actions
            .contains_key(&crate::events::AgenticEvent::HumanInTheLoop)
    );
}

#[test]
fn load_claudine_config_accepts_hyphenated_discord_webhook_provider() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".claudine");
    std::fs::create_dir_all(&config_dir).unwrap();

    let json5_content = r#"{
        preferred_agent: "claude",
        messenger: {
            active_config: "alerts",
            configurations: {
                alerts: {
                    provider: "discord-webhook",
                    webhook_url_env: "MY_DISCORD_URL",
                },
            },
        },
    }"#;
    let path = config_dir.join("config.json");
    std::fs::write(&path, json5_content).unwrap();

    let config = load_claudine_config(Some(&path), None).unwrap();
    let messenger = config.messenger.unwrap();
    assert_eq!(messenger.active_config.as_deref(), Some("alerts"));
    assert!(matches!(
        messenger.configurations.get("alerts").unwrap(),
        MessengerProviderConfig::DiscordWebhook { .. }
    ));
}

#[test]
fn load_claudine_config_detects_old_format() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".claudine");
    std::fs::create_dir_all(&config_dir).unwrap();

    let old = r#"{"version":"1.0","settings":{},"providers":{}}"#;
    let path = config_dir.join("config.json");
    std::fs::write(&path, old).unwrap();

    let result = load_claudine_config(Some(&path), None);
    assert!(result.is_err());
    assert!(config_dir.join("config.json.bak").exists());
}

#[test]
fn save_and_reload_claudine_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claudine/config.json");

    let config = ClaudineConfig::default();
    save_claudine_config(&config, &path).unwrap();

    let loaded = load_claudine_config(Some(&path), None).unwrap();
    assert_eq!(loaded.preferred_agent, config.preferred_agent);
    assert_eq!(loaded.logging, config.logging);
}

// =====================================================================
// CanonicalRuntimeConfig tests
// =====================================================================

#[test]
fn compile_canonical_runtime_indexes_by_event() {
    let mut config = ClaudineConfig::default();
    config.actions.insert(
        AgenticEvent::HumanInTheLoop,
        vec![HookAction::SoundEffect {
            effect: "doorbell".to_string(),
            volume: 1.0,
            speed: 1.0,
            when: None,
        }],
    );
    config.default_sounds = DefaultSounds::default();

    let runtime = compile_canonical_runtime(config, None).unwrap();
    assert!(runtime.get_binding(&AgenticEvent::HumanInTheLoop).is_some());
    assert!(runtime.get_binding(&AgenticEvent::SessionStart).is_none());
}

#[test]
fn compile_canonical_runtime_builds_protect() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = true;
    config.default_sounds = DefaultSounds::default();

    let runtime = compile_canonical_runtime(config, None).unwrap();
    assert!(runtime.protect_service().is_some());
}

#[test]
fn compile_canonical_runtime_no_protect_when_disabled() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();

    let runtime = compile_canonical_runtime(config, None).unwrap();
    assert!(runtime.protect_service().is_none());
}

#[test]
fn compile_canonical_runtime_compiles_call_mappers() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions.insert(
        AgenticEvent::BeforeTool,
        vec![HookAction::Call {
            command: "echo".to_string(),
            args: Some(vec!["allow because safe".to_string()]),
            mapper: Some(Mapper::Regex {
                pattern: r"(?P<decision>allow|deny)\s+because\s+(?P<reason>.*)".to_string(),
            }),
            timeout_ms: None,
            when: None,
        }],
    );

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let binding = runtime
        .get_binding(&AgenticEvent::BeforeTool)
        .expect("missing binding");
    assert_eq!(binding.actions().len(), 1);
    assert_eq!(binding.compiled_mappers().len(), 1);
    assert!(binding.compiled_mappers()[0].is_some());
}

#[test]
fn compile_canonical_runtime_compiles_expression_matcher() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions.insert(
        AgenticEvent::BeforeTool,
        vec![HookAction::Report {
            handler: None,
            when: None,
        }],
    );
    config.matchers.insert(
        AgenticEvent::BeforeTool,
        "tool_name == 'Bash' && git.branch == 'main'".to_string(),
    );

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let binding = runtime
        .get_binding(&AgenticEvent::BeforeTool)
        .expect("missing binding");
    match binding.matcher().expect("matcher should be compiled") {
        crate::dispatch::matcher::RuntimeMatcher::Expression { source, .. } => {
            assert_eq!(source, "tool_name == 'Bash' && git.branch == 'main'");
        }
        other => panic!("expected expression matcher, got {other:?}"),
    }
}

#[test]
fn compile_canonical_runtime_compiles_regex_matcher_fallback() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions.insert(
        AgenticEvent::BeforeTool,
        vec![HookAction::Report {
            handler: None,
            when: None,
        }],
    );
    config
        .matchers
        .insert(AgenticEvent::BeforeTool, "Bash|Edit".to_string());

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let binding = runtime
        .get_binding(&AgenticEvent::BeforeTool)
        .expect("missing binding");
    assert!(matches!(
        binding.matcher().expect("matcher should be compiled"),
        crate::dispatch::matcher::RuntimeMatcher::Regex(_)
    ));
}

#[test]
fn compile_canonical_runtime_drops_unparseable_matcher() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions.insert(
        AgenticEvent::BeforeTool,
        vec![HookAction::Report {
            handler: None,
            when: None,
        }],
    );
    config
        .matchers
        .insert(AgenticEvent::BeforeTool, "[invalid(regex".to_string());

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let binding = runtime
        .get_binding(&AgenticEvent::BeforeTool)
        .expect("missing binding");
    assert!(binding.matcher().is_none());
}

#[test]
fn invalid_matcher_in_config_compiles_to_unconditional_binding() {
    // End-to-end pin for the production semantics described in
    // [`crate::dispatch::matcher::RuntimeMatcher::compile`]: a matcher
    // string that is neither a valid Darkmatter condition nor a valid
    // regex must drop to `matcher() == None`, and `matcher::matches`
    // must then return `true`, so the binding fires unconditionally
    // rather than silently disappearing.
    //
    // The test-only helper [`matches_with_pattern`] returns `false`
    // for the same input, which is the *opposite* of production
    // behaviour. This test exists so a future contributor reading
    // that helper does not "fix" it in the wrong direction.
    let invalid_matcher = "[invalid(regex";

    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions.insert(
        AgenticEvent::BeforeTool,
        vec![HookAction::Report {
            handler: None,
            when: None,
        }],
    );
    config
        .matchers
        .insert(AgenticEvent::BeforeTool, invalid_matcher.to_string());

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let binding = runtime
        .get_binding(&AgenticEvent::BeforeTool)
        .expect("missing binding");

    assert!(
        binding.matcher().is_none(),
        "invalid matcher string must compile to None",
    );

    let meta = EventMeta {
        provider: Provider::Claude,
        event: AgenticEvent::BeforeTool,
        timestamp: chrono::Utc::now(),
        session_id: None,
        cwd: None,
        tool_name: Some("Bash".to_string()),
        tool_input: None,
        tool_response: None,
        error: None,
        prompt: None,
        agent_type: None,
        notification_type: None,
        notification_message: None,
        agent_pid: None,
        extra: HashMap::new(),
        env: EnvironmentContext::default(),
    };

    assert!(
        crate::dispatch::matcher::matches(binding.matcher(), &meta),
        "binding with no matcher must fire unconditionally",
    );
}

#[test]
fn compile_canonical_runtime_creates_binding_for_matcher_only_event() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config
        .matchers
        .insert(AgenticEvent::BeforeTool, "tool_name == 'Bash'".to_string());

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let binding = runtime
        .get_binding(&AgenticEvent::BeforeTool)
        .expect("matcher-only event should produce a binding");
    assert!(binding.matcher().is_some());
    assert!(binding.actions().is_empty());
}

#[test]
fn compile_canonical_runtime_fails_on_invalid_mapper_regex() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions.insert(
        AgenticEvent::BeforeTool,
        vec![HookAction::Call {
            command: "echo".to_string(),
            args: None,
            mapper: Some(Mapper::Regex {
                pattern: "[invalid(".to_string(),
            }),
            timeout_ms: None,
            when: None,
        }],
    );

    let error = compile_canonical_runtime(config, None).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("invalid mapper regex"));
    assert!(message.contains("before_tool"));
}

#[test]
fn compile_canonical_runtime_bridges_messenger_config() {
    use crate::config::messaging_block::{ClaudineMessengerConfig, MessengerProviderConfig};

    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.messenger = Some(ClaudineMessengerConfig {
        active_config: Some("alerts".to_string()),
        configurations: HashMap::from([(
            "alerts".to_string(),
            MessengerProviderConfig::Discord {
                channel_id: "999".to_string(),
                bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
            },
        )]),
    });

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let messaging = runtime.messaging();
    assert!(messaging.user.is_some());
    assert_eq!(
        messaging.user.as_ref().unwrap().active.as_deref(),
        Some("alerts")
    );
    assert!(
        messaging
            .user
            .as_ref()
            .unwrap()
            .configs
            .contains_key("alerts")
    );
}

#[test]
fn compile_canonical_runtime_no_messenger_gives_empty_messaging() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.messenger = None;

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let messaging = runtime.messaging();
    assert!(messaging.user.is_none());
    assert!(messaging.repo.is_none());
}

#[test]
fn bridge_provider_config_discord() {
    let cfg = MessengerProviderConfig::Discord {
        channel_id: "123".to_string(),
        bot_token_env: "MY_TOKEN".to_string(),
    };
    let route = bridge_provider_config(&cfg);
    match route {
        MessagingRouteConfig::Discord {
            channel_id,
            bot_token,
            bot_token_env,
        } => {
            assert_eq!(channel_id, "123");
            assert_eq!(bot_token, None);
            assert_eq!(bot_token_env, "MY_TOKEN");
        }
        other => panic!("expected Discord, got {other:?}"),
    }
}

#[test]
fn bridge_provider_config_slack() {
    let cfg = MessengerProviderConfig::Slack {
        channel_id: "C456".to_string(),
        bot_token_env: "SLACK_TOKEN".to_string(),
    };
    let route = bridge_provider_config(&cfg);
    match route {
        MessagingRouteConfig::Slack {
            channel_id,
            bot_token,
            bot_token_env,
        } => {
            assert_eq!(channel_id, "C456");
            assert_eq!(bot_token, None);
            assert_eq!(bot_token_env, "SLACK_TOKEN");
        }
        other => panic!("expected Slack, got {other:?}"),
    }
}

#[test]
fn bridge_provider_config_signal() {
    let cfg = MessengerProviderConfig::Signal {
        recipient: "+15551234567".to_string(),
        rpc_url_env: "SIG_RPC".to_string(),
        account_env: "SIG_ACCT".to_string(),
    };
    let route = bridge_provider_config(&cfg);
    match route {
        MessagingRouteConfig::Signal {
            recipient,
            rpc_url,
            rpc_url_env,
            account,
            account_env,
        } => {
            assert_eq!(recipient, "+15551234567");
            assert_eq!(rpc_url, None);
            assert_eq!(rpc_url_env, "SIG_RPC");
            assert_eq!(account, None);
            assert_eq!(account_env, "SIG_ACCT");
        }
        other => panic!("expected Signal, got {other:?}"),
    }
}

#[test]
fn bridge_provider_config_whatsapp() {
    let cfg = MessengerProviderConfig::Whatsapp {
        recipient: "+15559876543".to_string(),
        access_token_env: "WA_TOKEN".to_string(),
        phone_number_id_env: "WA_PHONE".to_string(),
    };
    let route = bridge_provider_config(&cfg);
    match route {
        MessagingRouteConfig::WhatsApp {
            recipient,
            access_token,
            access_token_env,
            phone_number_id,
            phone_number_id_env,
        } => {
            assert_eq!(recipient, "+15559876543");
            assert_eq!(access_token, None);
            assert_eq!(access_token_env, "WA_TOKEN");
            assert_eq!(phone_number_id, None);
            assert_eq!(phone_number_id_env, "WA_PHONE");
        }
        other => panic!("expected WhatsApp, got {other:?}"),
    }
}

#[test]
fn bridge_provider_config_discord_webhook() {
    let cfg = MessengerProviderConfig::DiscordWebhook {
        webhook_url: None,
        webhook_url_env: "MY_DISCORD_URL".to_string(),
    };
    let route = bridge_provider_config(&cfg);
    match route {
        MessagingRouteConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(webhook_url, None);
            assert_eq!(webhook_url_env, "MY_DISCORD_URL");
        }
        other => panic!("expected DiscordWebhook, got {other:?}"),
    }
}

#[test]
fn bridge_provider_config_slack_webhook() {
    let cfg = MessengerProviderConfig::SlackWebhook {
        webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXX".to_string()),
        webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
    };
    let route = bridge_provider_config(&cfg);
    match route {
        MessagingRouteConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(
                webhook_url,
                Some("https://hooks.slack.com/services/T000/B000/XXXX".to_string())
            );
            assert_eq!(webhook_url_env, "SLACK_WEBHOOK_URL");
        }
        other => panic!("expected SlackWebhook, got {other:?}"),
    }
}

#[test]
fn canonical_runtime_exposes_config() {
    let config = ClaudineConfig::default();
    let runtime = compile_canonical_runtime(config.clone(), None).unwrap();
    assert_eq!(runtime.config().preferred_agent, config.preferred_agent);
}

// =====================================================================
// Repo-scoped old-format config migration
// =====================================================================

#[test]
fn repo_old_format_config_backed_up_and_ignored() {
    let dir = tempfile::tempdir().unwrap();

    let user_path = dir.path().join("user-config.json");
    let user_config = ClaudineConfig {
        preferred_agent: Some(Provider::Claude),
        ..ClaudineConfig::default()
    };
    save_claudine_config(&user_config, &user_path).unwrap();

    let repo_dir = dir.path().join("repo");
    let repo_config_dir = repo_dir.join(".claudine");
    std::fs::create_dir_all(&repo_config_dir).unwrap();
    let repo_config_path = repo_config_dir.join("config.json");
    let old_format = serde_json::json!({
        "claude": {},
        "gemini": {}
    });
    std::fs::write(
        &repo_config_path,
        serde_json::to_string(&old_format).unwrap(),
    )
    .unwrap();

    let loaded = load_claudine_config(Some(&user_path), Some(&repo_dir)).unwrap();
    assert_eq!(
        loaded.preferred_agent,
        Some(Provider::Claude),
        "user config should be returned when repo config is old format"
    );

    assert!(
        !repo_config_path.exists(),
        "old-format repo config should have been renamed"
    );
    assert!(
        repo_config_dir.join("config.json.bak").exists(),
        "backup of old-format repo config should exist"
    );
}

#[test]
fn load_repo_override_config_old_format_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let old_format = serde_json::json!({
        "version": "1.0",
        "settings": {},
        "providers": {
            "claude": {
                "events": {}
            }
        }
    });
    std::fs::write(&path, serde_json::to_string(&old_format).unwrap()).unwrap();

    let result = load_repo_override_config(&path).unwrap();
    assert!(result.is_none(), "old-format config should return Ok(None)");

    assert!(!path.exists(), "old-format config should have been renamed");
    assert!(
        dir.path().join("config.json.bak").exists(),
        "backup should exist"
    );
}

// =====================================================================
// preferred_agent tests
// =====================================================================

#[test]
fn load_claudine_config_honors_preferred_agent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let config = ClaudineConfig {
        preferred_agent: Some(Provider::Codex),
        ..ClaudineConfig::default()
    };
    save_claudine_config(&config, &path).unwrap();

    let loaded = load_claudine_config(Some(&path), None).unwrap();
    assert_eq!(
        loaded.preferred_agent,
        Some(Provider::Codex),
        "preferred_agent should be Codex as written"
    );
}

#[test]
fn load_claudine_config_preferred_agent_all_providers() {
    let providers = [
        Provider::Claude,
        Provider::Codex,
        Provider::Gemini,
        Provider::Goose,
        Provider::KimiCode,
        Provider::OpenCode,
        Provider::QwenCode,
    ];

    let dir = tempfile::tempdir().unwrap();
    for provider in providers {
        let path = dir.path().join(format!("{provider:?}.json"));
        let config = ClaudineConfig {
            preferred_agent: Some(provider),
            ..ClaudineConfig::default()
        };
        save_claudine_config(&config, &path).unwrap();
        let loaded = load_claudine_config(Some(&path), None).unwrap();
        assert_eq!(
            loaded.preferred_agent,
            Some(provider),
            "preferred_agent round-trip failed for {provider:?}"
        );
    }
}

#[test]
fn load_claudine_config_preferred_agent_absent_loads_as_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, "{}").unwrap();

    let loaded = load_claudine_config(Some(&path), None).unwrap();
    assert!(loaded.preferred_agent.is_none());
}

#[test]
fn load_claudine_config_preferred_agent_not_overridden_by_repo() {
    let dir = tempfile::tempdir().unwrap();

    let user_path = dir.path().join("user.json");
    let user_config = ClaudineConfig {
        preferred_agent: Some(Provider::Codex),
        ..ClaudineConfig::default()
    };
    save_claudine_config(&user_config, &user_path).unwrap();

    let repo_dir = dir.path().join("repo");
    std::fs::create_dir_all(repo_dir.join(".claudine")).unwrap();
    let repo_override = RepoOverrideConfig {
        canonical_provider: Some(Provider::Gemini),
        ..RepoOverrideConfig::default()
    };
    save_repo_override_config(&repo_override, &repo_dir.join(".claudine/config.json")).unwrap();

    let loaded = load_claudine_config(Some(&user_path), Some(&repo_dir)).unwrap();
    assert_eq!(
        loaded.preferred_agent,
        Some(Provider::Codex),
        "repo should not override user's preferred_agent"
    );
    assert_eq!(
        loaded.canonical_provider,
        Some(Provider::Gemini),
        "repo should override canonical_provider"
    );
}

// =====================================================================
// Repo config migration (old format detection)
// =====================================================================

#[test]
fn load_claudine_config_old_format_creates_backup_and_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".claudine");
    std::fs::create_dir_all(&config_dir).unwrap();

    let old_config = serde_json::json!({
        "version": "1.0",
        "settings": {
            "tts": { "provider": "say" }
        },
        "providers": {
            "claude": {
                "events": {
                    "session_start": {
                        "enabled": true,
                        "actions": [
                            { "type": "speak", "message": "hello" }
                        ]
                    }
                }
            }
        }
    });

    let config_path = config_dir.join("config.json");
    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&old_config).unwrap(),
    )
    .unwrap();

    let result = load_claudine_config(Some(&config_path), None);
    assert!(result.is_err(), "old format should produce an error");
    assert!(
        matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
        "error should be ConfigNotFound"
    );

    assert!(
        !config_path.exists(),
        "original config should have been renamed"
    );
    assert!(
        config_dir.join("config.json.bak").exists(),
        "backup file should exist"
    );
}

#[test]
fn load_claudine_config_after_old_format_backup_returns_config_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".claudine");
    std::fs::create_dir_all(&config_dir).unwrap();

    let old_config = serde_json::json!({
        "version": "1.0",
        "settings": {},
        "providers": {}
    });
    let config_path = config_dir.join("config.json");
    std::fs::write(&config_path, serde_json::to_string(&old_config).unwrap()).unwrap();

    let _ = load_claudine_config(Some(&config_path), None);

    let result = load_claudine_config(Some(&config_path), None);
    assert!(
        matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
        "second load should also return ConfigNotFound"
    );
}

#[test]
fn load_claudine_config_detects_old_provider_keys_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let old_config = serde_json::json!({
        "claude": { "events": {} },
        "gemini": { "events": {} }
    });
    std::fs::write(&path, serde_json::to_string(&old_config).unwrap()).unwrap();

    let result = load_claudine_config(Some(&path), None);
    assert!(
        matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
        "root-level provider keys should be detected as old format"
    );
    assert!(
        dir.path().join("config.json.bak").exists(),
        "backup should be created for root-level provider key format"
    );
}

#[test]
fn load_claudine_config_does_not_backup_new_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let new_config = ClaudineConfig {
        preferred_agent: Some(Provider::Claude),
        ..ClaudineConfig::default()
    };
    save_claudine_config(&new_config, &path).unwrap();

    let loaded = load_claudine_config(Some(&path), None).unwrap();
    assert_eq!(loaded.preferred_agent, Some(Provider::Claude));
    assert!(
        !dir.path().join("config.json.bak").exists(),
        "new format should not produce a backup"
    );
}
