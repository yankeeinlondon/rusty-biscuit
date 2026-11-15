use super::*;
use crate::config::claudine_config::ClaudineConfig;

#[test]
fn messenger_discord_deserializes_with_env_default() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "active_config": "work",
            "configurations": {
                "work": {
                    "discord": {
                        "channel_id": "123456789"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json).unwrap();
    let messenger = config.messenger.unwrap();
    assert_eq!(messenger.active_config.as_deref(), Some("work"));
    match messenger.configurations.get("work").unwrap() {
        MessengerProviderConfig::Discord {
            channel_id,
            bot_token_env,
            ..
        } => {
            assert_eq!(channel_id, "123456789");
            assert_eq!(bot_token_env, "DISCORD_BOT_TOKEN");
        }
        other => panic!("expected Discord, got {other:?}"),
    }
}

#[test]
fn messenger_slack_deserializes() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "alerts": {
                    "slack": {
                        "channel_id": "C0ABC"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json).unwrap();
    let messenger = config.messenger.unwrap();
    assert!(messenger.active_config.is_none());
    match messenger.configurations.get("alerts").unwrap() {
        MessengerProviderConfig::Slack {
            channel_id,
            bot_token_env,
            ..
        } => {
            assert_eq!(channel_id, "C0ABC");
            assert_eq!(bot_token_env, "SLACK_BOT_TOKEN");
        }
        other => panic!("expected Slack, got {other:?}"),
    }
}

#[test]
fn messenger_signal_deserializes_with_defaults() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "personal": {
                    "signal": {
                        "recipient": "+15551234567"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json).unwrap();
    let messenger = config.messenger.unwrap();
    match messenger.configurations.get("personal").unwrap() {
        MessengerProviderConfig::Signal {
            recipient,
            rpc_url_env,
            account_env,
            ..
        } => {
            assert_eq!(recipient, "+15551234567");
            assert_eq!(rpc_url_env, "SIGNAL_RPC_URL");
            assert_eq!(account_env, "SIGNAL_ACCOUNT");
        }
        other => panic!("expected Signal, got {other:?}"),
    }
}

#[test]
fn messenger_whatsapp_deserializes_with_defaults() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "biz": {
                    "whatsapp": {
                        "recipient": "+15559876543"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json).unwrap();
    let messenger = config.messenger.unwrap();
    match messenger.configurations.get("biz").unwrap() {
        MessengerProviderConfig::Whatsapp {
            recipient,
            access_token_env,
            phone_number_id_env,
            ..
        } => {
            assert_eq!(recipient, "+15559876543");
            assert_eq!(access_token_env, "WHATSAPP_ACCESS_TOKEN");
            assert_eq!(phone_number_id_env, "WHATSAPP_PHONE_NUMBER_ID");
        }
        other => panic!("expected Whatsapp, got {other:?}"),
    }
}

#[test]
fn messenger_round_trip() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "active_config": "main",
            "configurations": {
                "main": {
                    "slack": {
                        "channel_id": "C999"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json.clone()).unwrap();
    let serialized = serde_json::to_value(&config).unwrap();
    let back: ClaudineConfig = ClaudineConfig::from_user_document(serialized).unwrap();
    let messenger = back.messenger.unwrap();
    assert_eq!(messenger.active_config.as_deref(), Some("main"));
}

#[test]
fn messenger_discord_webhook_deserializes_with_defaults() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "alerts": {
                    "discord_webhook": {
                        "webhook_url_env": "MY_DISCORD_URL"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json).unwrap();
    let messenger = config.messenger.unwrap();
    match messenger.configurations.get("alerts").unwrap() {
        MessengerProviderConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(webhook_url, &None);
            assert_eq!(webhook_url_env, "MY_DISCORD_URL");
        }
        other => panic!("expected DiscordWebhook, got {other:?}"),
    }
}

#[test]
fn a_hyphenated_webhook_key_is_not_a_route_kind() {
    // The flat form accepted `provider: discord-webhook`; the keyed form
    // has one spelling per kind, and the legacy converter normalizes it.
    let json = serde_json::json!({
        "messenger": {
            "configurations": {
                "alerts": {
                    "discord-webhook": {
                        "webhook_url_env": "MY_DISCORD_URL"
                    }
                }
            }
        }
    });
    assert!(ClaudineConfig::from_user_document(json).is_err());
}

#[test]
fn messenger_slack_webhook_deserializes_with_inline_url() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "deploys": {
                    "slack_webhook": {
                        "webhook_url": "https://hooks.slack.com/services/T000/B000/XXXX",
                        "webhook_url_env": "SLACK_WEBHOOK_URL"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json).unwrap();
    let messenger = config.messenger.unwrap();
    match messenger.configurations.get("deploys").unwrap() {
        MessengerProviderConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(
                webhook_url.as_deref(),
                Some("https://hooks.slack.com/services/T000/B000/XXXX")
            );
            assert_eq!(webhook_url_env, "SLACK_WEBHOOK_URL");
        }
        other => panic!("expected SlackWebhook, got {other:?}"),
    }
}

#[test]
fn messenger_webhook_round_trip() {
    let json = serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "active_config": "deploys",
            "configurations": {
                "deploys": {
                    "slack_webhook": {
                        "webhook_url_env": "DEPLOY_SLACK_WEBHOOK_URL"
                    }
                },
                "personal-alerts": {
                    "discord_webhook": {
                        "webhook_url_env": "DISCORD_WEBHOOK_URL"
                    }
                }
            }
        }
    });
    let config: ClaudineConfig = ClaudineConfig::from_user_document(json.clone()).unwrap();
    let serialized = serde_json::to_value(&config).unwrap();
    let back: ClaudineConfig = ClaudineConfig::from_user_document(serialized).unwrap();
    let messenger = back.messenger.unwrap();
    assert_eq!(messenger.active_config.as_deref(), Some("deploys"));
    assert!(messenger.configurations.contains_key("deploys"));
    assert!(messenger.configurations.contains_key("personal-alerts"));
}

#[test]
fn validate_rejects_invalid_discord_webhook_url() {
    let config: ClaudineConfig = ClaudineConfig::from_user_document(serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "bad": {
                    "discord_webhook": {
                        "webhook_url": "not-a-url"
                    }
                }
            }
        }
    }))
    .unwrap();
    let result = config.validate();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("bad"), "error: {msg}");
    assert!(
        msg.contains("not a valid Discord webhook URL"),
        "error: {msg}"
    );
}

#[test]
fn validate_rejects_invalid_slack_webhook_url() {
    let config: ClaudineConfig = ClaudineConfig::from_user_document(serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "bad": {
                    "slack_webhook": {
                        "webhook_url": "https://example.com/hook"
                    }
                }
            }
        }
    }))
    .unwrap();
    let result = config.validate();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("bad"), "error: {msg}");
    assert!(
        msg.contains("not a valid Slack webhook URL"),
        "error: {msg}"
    );
}

#[test]
fn validate_accepts_env_only_webhook_config() {
    let config: ClaudineConfig = ClaudineConfig::from_user_document(serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "env-only": {
                    "discord_webhook": {
                        "webhook_url_env": "MY_DISCORD_URL"
                    }
                }
            }
        }
    }))
    .unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn validate_accepts_valid_inline_webhook_url() {
    let config: ClaudineConfig = ClaudineConfig::from_user_document(serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "inline": {
                    "discord_webhook": {
                        "webhook_url": "https://discord.com/api/webhooks/123456/abcDEF"
                    }
                }
            }
        }
    }))
    .unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn validate_rejects_blank_webhook_url_env_when_no_inline() {
    let config: ClaudineConfig = ClaudineConfig::from_user_document(serde_json::json!({
        "preferred_agent": "claude",
        "messenger": {
            "configurations": {
                "bad": {
                    "slack_webhook": {
                        "webhook_url_env": ""
                    }
                }
            }
        }
    }))
    .unwrap();
    let result = config.validate();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("bad"), "error: {msg}");
}

/// A messenger config with configurations but no `active_config`
/// passes validation. This is the safe state the TUI should create
/// when adding a new messenger route without filling required fields.
#[test]
fn messenger_without_active_config_passes_validation() {
    let config = ClaudineConfig {
        messenger: Some(ClaudineMessengerConfig {
            active_config: None,
            configurations: HashMap::from([(
                "wip".to_string(),
                MessengerProviderConfig::Discord {
                    channel_id: String::new(), // intentionally empty (WIP)
                    bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                },
            )]),
        }),
        ..Default::default()
    };
    assert!(
        config.validate().is_ok(),
        "config with no active_config should validate even when configs have empty fields"
    );
}
