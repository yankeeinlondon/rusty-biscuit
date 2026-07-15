//! Messaging configuration types for outbound notifications.
//!
//! This module defines the configuration structures that users write in their
//! `config.json` files to configure messaging routes (Discord, Slack, Signal, WhatsApp).

use crate::error::{ClaudineError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

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

fn default_discord_webhook_url() -> String {
    "DISCORD_WEBHOOK_URL".to_string()
}

fn default_slack_webhook_url() -> String {
    "SLACK_WEBHOOK_URL".to_string()
}

/// Conservative validator regex for Discord webhook URLs.
///
/// This regex is intentionally stricter than the actual Discord token charset
/// (e.g., it does not include `/`). The authoritative validation happens in
/// `messenger::provider::DiscordWebhookProvider::try_new` at send time.
static DISCORD_WEBHOOK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https://(discord\.com|discordapp\.com)/api/webhooks/[0-9]+/[A-Za-z0-9._-]+$")
        .unwrap()
});

/// Conservative validator regex for Slack webhook URLs.
///
/// Slack may introduce new token formats in the future. This regex is early
/// TUI feedback only; the authoritative check is in
/// `messenger::provider::SlackWebhookProvider::try_new`.
static SLACK_WEBHOOK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https://hooks\.slack\.com/services/[A-Z0-9]+/[A-Z0-9]+/[A-Za-z0-9]+$").unwrap()
});

/// Validate a Discord webhook URL with a conservative regex.
pub fn validate_discord_webhook_url(url: &str) -> bool {
    DISCORD_WEBHOOK_REGEX.is_match(url)
}

/// Validate a Slack webhook URL with a conservative regex.
pub fn validate_slack_webhook_url(url: &str) -> bool {
    SLACK_WEBHOOK_REGEX.is_match(url)
}

/// Messaging configuration for a specific provider/route.
///
/// Each variant represents a different messaging provider with its required
/// credentials and endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "provider", rename_all = "lowercase", deny_unknown_fields)]
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
    /// Discord webhook configuration.
    #[serde(rename = "discord_webhook")]
    #[serde(alias = "discord-webhook")]
    DiscordWebhook {
        /// Optional inline webhook URL.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        /// Environment variable name containing the webhook URL.
        #[serde(default = "default_discord_webhook_url")]
        webhook_url_env: String,
    },
    /// Slack webhook configuration.
    #[serde(rename = "slack_webhook")]
    #[serde(alias = "slack-webhook")]
    SlackWebhook {
        /// Optional inline webhook URL.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        /// Environment variable name containing the webhook URL.
        #[serde(default = "default_slack_webhook_url")]
        webhook_url_env: String,
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
        MessagingRouteConfig::DiscordWebhook {
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
                    "{}: route '{}': webhook_url_env cannot be blank when webhook_url is not set",
                    scope_label, name
                )));
            }
            if let Some(url) = webhook_url
                && !url.trim().is_empty()
                && !validate_discord_webhook_url(url)
            {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': webhook_url is not a valid Discord webhook URL",
                    scope_label, name
                )));
            }
        }
        MessagingRouteConfig::SlackWebhook {
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
                    "{}: route '{}': webhook_url_env cannot be blank when webhook_url is not set",
                    scope_label, name
                )));
            }
            if let Some(url) = webhook_url
                && !url.trim().is_empty()
                && !validate_slack_webhook_url(url)
            {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{}: route '{}': webhook_url is not a valid Slack webhook URL",
                    scope_label, name
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
