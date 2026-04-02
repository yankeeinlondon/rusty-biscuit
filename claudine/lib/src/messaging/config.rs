//! Messaging configuration types for outbound notifications.
//!
//! This module defines the configuration structures that users write in their
//! `config.json` files to configure messaging routes (Discord, Slack, Signal, WhatsApp).

use crate::error::{ClaudineError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Default environment variable names for each provider
fn default_discord_bot_token() -> String {
    "DISCORD_BOT_TOKEN".to_string()
}

fn default_slack_bot_token() -> String {
    "SLACK_BOT_TOKEN".to_string()
}

fn default_signal_rpc_url() -> String {
    "SIGNAL_RPC_URL".to_string()
}

fn default_signal_account() -> String {
    "SIGNAL_ACCOUNT".to_string()
}

fn default_whatsapp_access_token() -> String {
    "WHATSAPP_ACCESS_TOKEN".to_string()
}

fn default_whatsapp_phone_number_id() -> String {
    "WHATSAPP_PHONE_NUMBER_ID".to_string()
}

/// Messaging configuration for a specific provider/route.
///
/// Each variant represents a different messaging provider with its required
/// credentials and endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum MessagingRouteConfig {
    /// Discord bot configuration.
    Discord {
        /// Discord channel ID to send messages to.
        channel_id: String,
        /// Optional inline bot token (discouraged - use env var instead).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        /// Environment variable name containing the bot token.
        #[serde(default = "default_discord_bot_token")]
        bot_token_env: String,
    },
    /// Slack bot configuration.
    Slack {
        /// Slack channel ID to send messages to.
        channel_id: String,
        /// Optional inline bot token (discouraged - use env var instead).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        /// Environment variable name containing the bot token.
        #[serde(default = "default_slack_bot_token")]
        bot_token_env: String,
    },
    /// Signal messenger configuration.
    Signal {
        /// Recipient phone number or group ID.
        recipient: String,
        /// Optional inline RPC URL (can use env var instead).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rpc_url: Option<String>,
        /// Environment variable name containing the RPC URL.
        #[serde(default = "default_signal_rpc_url")]
        rpc_url_env: String,
        /// Optional account identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account: Option<String>,
        /// Environment variable name containing the account.
        #[serde(default = "default_signal_account")]
        account_env: String,
    },
    /// WhatsApp Business API configuration.
    WhatsApp {
        /// Recipient phone number.
        recipient: String,
        /// Optional inline access token (discouraged - use env var instead).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        access_token: Option<String>,
        /// Environment variable name containing the access token.
        #[serde(default = "default_whatsapp_access_token")]
        access_token_env: String,
        /// Optional inline phone number ID (can use env var instead).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phone_number_id: Option<String>,
        /// Environment variable name containing the phone number ID.
        #[serde(default = "default_whatsapp_phone_number_id")]
        phone_number_id_env: String,
    },
}

/// Scoped messaging settings containing named routes and an active selection.
///
/// Users can define multiple messaging routes (e.g., "urgent", "monitoring", "alerts")
/// and designate one as active for a given scope (global, repo, or project).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScopedMessagingSettings {
    /// Name of the currently active route (must exist in `configs` if present).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    /// Named messaging route configurations.
    #[serde(default)]
    pub configs: HashMap<String, MessagingRouteConfig>,
}

impl ScopedMessagingSettings {
    /// Validates the messaging settings for semantic correctness.
    ///
    /// ## Errors
    ///
    /// Returns `ClaudineError::ConfigValidation` if:
    /// - Any config name is blank (after trimming)
    /// - `active` is set but blank or not found in `configs`
    /// - Any route config has blank required fields (channel_id, recipient, env var names)
    pub fn validate(&self, scope_label: &str) -> Result<()> {
        // Validate config names are not blank
        for key in self.configs.keys() {
            if key.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: config name cannot be blank",
                    scope_label
                )));
            }
        }

        // Validate active route exists if specified
        if let Some(active) = &self.active {
            if active.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: active route name cannot be blank",
                    scope_label
                )));
            }
            if !self.configs.contains_key(active) {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: active route '{}' not found in configs",
                    scope_label, active
                )));
            }
        }

        // Validate each route config
        for (name, config) in &self.configs {
            validate_route_config(config, name, scope_label)?;
        }

        Ok(())
    }
}

/// Validates a single route configuration.
fn validate_route_config(
    config: &MessagingRouteConfig,
    name: &str,
    scope_label: &str,
) -> Result<()> {
    match config {
        MessagingRouteConfig::Discord {
            channel_id,
            bot_token_env,
            ..
        } => {
            if channel_id.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': channel_id cannot be blank",
                    scope_label, name
                )));
            }
            if bot_token_env.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': bot_token_env cannot be blank",
                    scope_label, name
                )));
            }
        }
        MessagingRouteConfig::Slack {
            channel_id,
            bot_token_env,
            ..
        } => {
            if channel_id.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': channel_id cannot be blank",
                    scope_label, name
                )));
            }
            if bot_token_env.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': bot_token_env cannot be blank",
                    scope_label, name
                )));
            }
        }
        MessagingRouteConfig::Signal {
            recipient,
            rpc_url_env,
            account_env,
            ..
        } => {
            if recipient.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': recipient cannot be blank",
                    scope_label, name
                )));
            }
            if rpc_url_env.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': rpc_url_env cannot be blank",
                    scope_label, name
                )));
            }
            if account_env.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': account_env cannot be blank",
                    scope_label, name
                )));
            }
        }
        MessagingRouteConfig::WhatsApp {
            recipient,
            access_token_env,
            phone_number_id_env,
            ..
        } => {
            if recipient.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': recipient cannot be blank",
                    scope_label, name
                )));
            }
            if access_token_env.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': access_token_env cannot be blank",
                    scope_label, name
                )));
            }
            if phone_number_id_env.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': phone_number_id_env cannot be blank",
                    scope_label, name
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
    fn validate_rejects_missing_active_route() {
        let settings = ScopedMessagingSettings {
            active: Some("nonexistent".to_string()),
            configs: HashMap::new(),
        };

        let result = settings.validate("global");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("active route 'nonexistent' not found"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("channel_id cannot be blank"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("bot_token_env cannot be blank"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("config name cannot be blank"));
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
}
