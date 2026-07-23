use super::*;

#[test]
fn discord_config_deserializes_with_env_default() {
    let json = r#"{
        "provider": "discord",
        "channel_id": "123456789"
    }"#;

    let config: MessagingRouteConfig = serde_json::from_str(json).unwrap();

    match config {
        MessagingRouteConfig::Discord {
            channel_id,
            bot_token,
            bot_token_env,
        } => {
            assert_eq!(channel_id, "123456789");
            assert_eq!(bot_token, None);
            assert_eq!(bot_token_env, "DISCORD_BOT_TOKEN");
        }
        _ => panic!("Expected Discord variant"),
    }
}

#[test]
fn slack_config_deserializes_with_inline_token() {
    let json = r#"{
        "provider": "slack",
        "channel_id": "C0123456789",
        "bot_token": "xoxb-secret-token"
    }"#;

    let config: MessagingRouteConfig = serde_json::from_str(json).unwrap();

    match config {
        MessagingRouteConfig::Slack {
            channel_id,
            bot_token,
            bot_token_env,
        } => {
            assert_eq!(channel_id, "C0123456789");
            assert_eq!(bot_token, Some("xoxb-secret-token".to_string()));
            assert_eq!(bot_token_env, "SLACK_BOT_TOKEN");
        }
        _ => panic!("Expected Slack variant"),
    }
}

#[test]
fn signal_config_deserializes_with_defaults() {
    let json = r#"{
        "provider": "signal",
        "recipient": "+15551234567"
    }"#;

    let config: MessagingRouteConfig = serde_json::from_str(json).unwrap();

    match config {
        MessagingRouteConfig::Signal {
            recipient,
            rpc_url,
            rpc_url_env,
            account,
            account_env,
        } => {
            assert_eq!(recipient, "+15551234567");
            assert_eq!(rpc_url, None);
            assert_eq!(rpc_url_env, "SIGNAL_RPC_URL");
            assert_eq!(account, None);
            assert_eq!(account_env, "SIGNAL_ACCOUNT");
        }
        _ => panic!("Expected Signal variant"),
    }
}

#[test]
fn whatsapp_config_deserializes_with_defaults() {
    let json = r#"{
        "provider": "whatsapp",
        "recipient": "+15551234567"
    }"#;

    let config: MessagingRouteConfig = serde_json::from_str(json).unwrap();

    match config {
        MessagingRouteConfig::WhatsApp {
            recipient,
            access_token,
            access_token_env,
            phone_number_id,
            phone_number_id_env,
        } => {
            assert_eq!(recipient, "+15551234567");
            assert_eq!(access_token, None);
            assert_eq!(access_token_env, "WHATSAPP_ACCESS_TOKEN");
            assert_eq!(phone_number_id, None);
            assert_eq!(phone_number_id_env, "WHATSAPP_PHONE_NUMBER_ID");
        }
        _ => panic!("Expected WhatsApp variant"),
    }
}

#[test]
fn scoped_settings_round_trip() {
    let mut configs = HashMap::new();
    configs.insert(
        "urgent".to_string(),
        MessagingRouteConfig::Discord {
            channel_id: "987654321".to_string(),
            bot_token: None,
            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: Some("urgent".to_string()),
        configs,
    };

    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: ScopedMessagingSettings = serde_json::from_str(&json).unwrap();

    assert_eq!(settings, deserialized);
}

#[test]
fn scoped_settings_deserializes_with_no_active() {
    let json = r#"{
        "configs": {
            "test": {
                "provider": "slack",
                "channel_id": "C999"
            }
        }
    }"#;

    let settings: ScopedMessagingSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.active, None);
    assert_eq!(settings.configs.len(), 1);
}

#[test]
fn route_config_rejects_unknown_fields() {
    let json = r#"{
        "provider": "slack",
        "channel_id": "C0123456789",
        "bot_toke_env": "SLACK_BOT_TOKEN"
    }"#;

    let error = serde_json::from_str::<MessagingRouteConfig>(json).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("unknown field"));
    assert!(message.contains("bot_toke_env"));
}

#[test]
fn validate_rejects_missing_active_route() {
    let settings = ScopedMessagingSettings {
        active: Some("nonexistent".to_string()),
        configs: HashMap::new(),
    };

    let result = settings.validate("global");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("active route 'nonexistent' not found")
    );
}

#[test]
fn validate_rejects_blank_channel_id() {
    let mut configs = HashMap::new();
    configs.insert(
        "broken".to_string(),
        MessagingRouteConfig::Discord {
            channel_id: "".to_string(),
            bot_token: None,
            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    let result = settings.validate("repo");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("channel_id cannot be blank")
    );
}

#[test]
fn validate_rejects_blank_env_var_name() {
    let mut configs = HashMap::new();
    configs.insert(
        "broken".to_string(),
        MessagingRouteConfig::Slack {
            channel_id: "C123".to_string(),
            bot_token: None,
            bot_token_env: "".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    let result = settings.validate("project");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("bot_token_env cannot be blank")
    );
}

#[test]
fn validate_rejects_blank_config_name() {
    let mut configs = HashMap::new();
    configs.insert(
        "  ".to_string(),
        MessagingRouteConfig::Discord {
            channel_id: "123".to_string(),
            bot_token: None,
            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    let result = settings.validate("global");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("config name cannot be blank")
    );
}

#[test]
fn validate_accepts_valid_config() {
    let mut configs = HashMap::new();
    configs.insert(
        "valid".to_string(),
        MessagingRouteConfig::Slack {
            channel_id: "C123456".to_string(),
            bot_token: Some("xoxb-token".to_string()),
            bot_token_env: "SLACK_BOT_TOKEN".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: Some("valid".to_string()),
        configs,
    };

    assert!(settings.validate("test").is_ok());
}

#[test]
fn validate_accepts_no_active_with_configs() {
    let mut configs = HashMap::new();
    configs.insert(
        "route1".to_string(),
        MessagingRouteConfig::Discord {
            channel_id: "789".to_string(),
            bot_token: None,
            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    assert!(settings.validate("repo").is_ok());
}

#[test]
fn discord_webhook_config_deserializes_with_explicit_name() {
    let json = r#"{
        "provider": "discord_webhook",
        "webhook_url_env": "MY_DISCORD_URL"
    }"#;

    let config: MessagingRouteConfig = serde_json::from_str(json).unwrap();

    match config {
        MessagingRouteConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(webhook_url, None);
            assert_eq!(webhook_url_env, "MY_DISCORD_URL");
        }
        _ => panic!("Expected DiscordWebhook variant"),
    }
}

#[test]
fn discord_webhook_config_accepts_hyphenated_provider_alias() {
    let json = r#"{
        "provider": "discord-webhook",
        "webhook_url_env": "MY_DISCORD_URL"
    }"#;

    let config: MessagingRouteConfig = serde_json::from_str(json).unwrap();

    assert!(matches!(
        config,
        MessagingRouteConfig::DiscordWebhook { .. }
    ));
}

#[test]
fn slack_webhook_config_deserializes_with_inline_url() {
    let json = r#"{
        "provider": "slack_webhook",
        "webhook_url": "https://hooks.slack.com/services/T000/B000/XXXX",
        "webhook_url_env": "SLACK_WEBHOOK_URL"
    }"#;

    let config: MessagingRouteConfig = serde_json::from_str(json).unwrap();

    match config {
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
        _ => panic!("Expected SlackWebhook variant"),
    }
}

#[test]
fn webhook_configs_round_trip() {
    let mut configs = HashMap::new();
    configs.insert(
        "alerts".to_string(),
        MessagingRouteConfig::DiscordWebhook {
            webhook_url: None,
            webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
        },
    );
    configs.insert(
        "deploys".to_string(),
        MessagingRouteConfig::SlackWebhook {
            webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXX".to_string()),
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: Some("alerts".to_string()),
        configs,
    };

    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: ScopedMessagingSettings = serde_json::from_str(&json).unwrap();

    assert_eq!(settings, deserialized);
}

#[test]
fn validate_rejects_blank_webhook_url_env_when_no_inline() {
    let mut configs = HashMap::new();
    configs.insert(
        "broken".to_string(),
        MessagingRouteConfig::DiscordWebhook {
            webhook_url: None,
            webhook_url_env: "".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    let result = settings.validate("global");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("webhook_url_env cannot be blank")
    );
}

#[test]
fn validate_rejects_invalid_discord_webhook_url() {
    let mut configs = HashMap::new();
    configs.insert(
        "bad".to_string(),
        MessagingRouteConfig::DiscordWebhook {
            webhook_url: Some("not-a-url".to_string()),
            webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    let result = settings.validate("test");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("not a valid Discord webhook URL")
    );
}

#[test]
fn validate_rejects_invalid_slack_webhook_url() {
    let mut configs = HashMap::new();
    configs.insert(
        "bad".to_string(),
        MessagingRouteConfig::SlackWebhook {
            webhook_url: Some("https://example.com/hook".to_string()),
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    let result = settings.validate("test");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("not a valid Slack webhook URL")
    );
}

#[test]
fn validate_accepts_env_only_webhook_config() {
    let mut configs = HashMap::new();
    configs.insert(
        "env-only".to_string(),
        MessagingRouteConfig::SlackWebhook {
            webhook_url: None,
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    assert!(settings.validate("test").is_ok());
}

#[test]
fn validate_accepts_valid_inline_discord_webhook_url() {
    let mut configs = HashMap::new();
    configs.insert(
        "inline".to_string(),
        MessagingRouteConfig::DiscordWebhook {
            webhook_url: Some("https://discord.com/api/webhooks/123456/abcDEF".to_string()),
            webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    assert!(settings.validate("test").is_ok());
}

#[test]
fn validate_accepts_valid_inline_slack_webhook_url() {
    let mut configs = HashMap::new();
    configs.insert(
        "inline".to_string(),
        MessagingRouteConfig::SlackWebhook {
            webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXX".to_string()),
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let settings = ScopedMessagingSettings {
        active: None,
        configs,
    };

    assert!(settings.validate("test").is_ok());
}

#[test]
fn validate_discord_webhook_url_accepts_production_urls() {
    assert!(validate_discord_webhook_url(
        "https://discord.com/api/webhooks/123456/abcDEF"
    ));
    assert!(validate_discord_webhook_url(
        "https://discordapp.com/api/webhooks/999999/xyz_123.ABC"
    ));
}

#[test]
fn validate_discord_webhook_url_rejects_malformed() {
    assert!(!validate_discord_webhook_url("not-a-url"));
    assert!(!validate_discord_webhook_url(
        "http://discord.com/api/webhooks/123/abc"
    ));
    assert!(!validate_discord_webhook_url(
        "https://example.com/api/webhooks/123/abc"
    ));
    assert!(!validate_discord_webhook_url(""));
}

#[test]
fn validate_slack_webhook_url_accepts_production_urls() {
    assert!(validate_slack_webhook_url(
        "https://hooks.slack.com/services/T000/B000/XXXX"
    ));
    assert!(validate_slack_webhook_url(
        "https://hooks.slack.com/services/T123ABC/B456DEF/ghi789JKL"
    ));
}

#[test]
fn validate_slack_webhook_url_rejects_malformed() {
    assert!(!validate_slack_webhook_url("not-a-url"));
    assert!(!validate_slack_webhook_url(
        "http://hooks.slack.com/services/T000/B000/XXXX"
    ));
    assert!(!validate_slack_webhook_url(
        "https://example.com/services/T000/B000/XXXX"
    ));
    assert!(!validate_slack_webhook_url(""));
}
