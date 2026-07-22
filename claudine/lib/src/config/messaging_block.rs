//! Messenger configuration types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{ClaudineError, Result};

// ============================================================================
// Default env var name helpers
// ============================================================================

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

fn default_discord_webhook_url() -> String {
    "DISCORD_WEBHOOK_URL".to_string()
}

fn default_slack_webhook_url() -> String {
    "SLACK_WEBHOOK_URL".to_string()
}

// ============================================================================
// MessengerProviderConfig
// ============================================================================

/// Provider-specific configuration for a single messaging route.
///
/// Serializes/deserializes with a `"provider"` tag and snake_case variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case", deny_unknown_fields)]
pub enum MessengerProviderConfig {
    /// Discord bot configuration.
    Discord {
        /// Discord channel ID to send messages to.
        channel_id: String,
        /// Environment variable holding the bot token.
        #[serde(default = "default_discord_bot_token")]
        bot_token_env: String,
    },
    /// Slack bot configuration.
    Slack {
        /// Slack channel ID to send messages to.
        channel_id: String,
        /// Environment variable holding the bot token.
        #[serde(default = "default_slack_bot_token")]
        bot_token_env: String,
    },
    /// Signal messenger configuration.
    Signal {
        /// Recipient phone number or group ID.
        recipient: String,
        /// Environment variable holding the RPC URL.
        #[serde(default = "default_signal_rpc_url")]
        rpc_url_env: String,
        /// Environment variable holding the account.
        #[serde(default = "default_signal_account")]
        account_env: String,
    },
    /// WhatsApp Business API configuration.
    Whatsapp {
        /// Recipient phone number.
        recipient: String,
        /// Environment variable holding the access token.
        #[serde(default = "default_whatsapp_access_token")]
        access_token_env: String,
        /// Environment variable holding the phone number ID.
        #[serde(default = "default_whatsapp_phone_number_id")]
        phone_number_id_env: String,
    },
    /// Discord webhook configuration.
    #[serde(alias = "discord-webhook")]
    DiscordWebhook {
        /// Optional inline webhook URL.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        /// Environment variable holding the webhook URL.
        #[serde(default = "default_discord_webhook_url")]
        webhook_url_env: String,
    },
    /// Slack webhook configuration.
    #[serde(alias = "slack-webhook")]
    SlackWebhook {
        /// Optional inline webhook URL.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        /// Environment variable holding the webhook URL.
        #[serde(default = "default_slack_webhook_url")]
        webhook_url_env: String,
    },
}

// ============================================================================
// ClaudineMessengerConfig
// ============================================================================

/// Messenger settings: named configurations and an active selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudineMessengerConfig {
    /// Key of the currently active messenger configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_config: Option<String>,

    /// Named messenger route configurations.
    #[serde(default)]
    pub configurations: HashMap<String, MessengerProviderConfig>,
}

impl ClaudineMessengerConfig {
    /// Validate that `active_config` references an existing key in `configurations`.
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(active) = &self.active_config {
            if active.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(
                    "messenger.active_config cannot be blank".to_string(),
                ));
            }
            if !self.configurations.contains_key(active) {
                return Err(ClaudineError::ConfigValidation(format!(
                    "messenger.active_config '{active}' not found in messenger.configurations"
                )));
            }
        }

        for (name, config) in &self.configurations {
            validate_provider_config(config, name)?;
        }

        Ok(())
    }
}

fn validate_provider_config(config: &MessengerProviderConfig, name: &str) -> Result<()> {
    match config {
        // Existing bot-token routes are validated at the runtime level;
        // leave them unvalidated here so the TUI can create WIP routes.
        MessengerProviderConfig::Discord { .. }
        | MessengerProviderConfig::Slack { .. }
        | MessengerProviderConfig::Signal { .. }
        | MessengerProviderConfig::Whatsapp { .. } => {}
        MessengerProviderConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            if webhook_url
                .as_ref()
                .map(|s| s.trim())
                .is_none_or(|s| s.is_empty())
                && webhook_url_env.trim().is_empty()
            {
                return Err(ClaudineError::ConfigValidation(format!(
                    "messenger configuration '{name}': webhook_url_env cannot be blank when webhook_url is not set"
                )));
            }
            if let Some(url) = webhook_url
                && !url.trim().is_empty()
                && !crate::messaging::validate_discord_webhook_url(url)
            {
                return Err(ClaudineError::ConfigValidation(format!(
                    "messenger configuration '{name}': webhook_url is not a valid Discord webhook URL"
                )));
            }
        }
        MessengerProviderConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            if webhook_url
                .as_ref()
                .map(|s| s.trim())
                .is_none_or(|s| s.is_empty())
                && webhook_url_env.trim().is_empty()
            {
                return Err(ClaudineError::ConfigValidation(format!(
                    "messenger configuration '{name}': webhook_url_env cannot be blank when webhook_url is not set"
                )));
            }
            if let Some(url) = webhook_url
                && !url.trim().is_empty()
                && !crate::messaging::validate_slack_webhook_url(url)
            {
                return Err(ClaudineError::ConfigValidation(format!(
                    "messenger configuration '{name}': webhook_url is not a valid Slack webhook URL"
                )));
            }
        }
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests;
