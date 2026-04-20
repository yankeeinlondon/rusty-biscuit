use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use color_eyre::eyre::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// CLI configuration loaded from `~/.messenger.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub default_route: Option<String>,
    #[serde(default)]
    pub routes: HashMap<String, RouteConfig>,
}

/// Supported CLI providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum RouteProvider {
    #[value(name = "discord")]
    Discord,
    #[serde(rename = "discord-webhook")]
    #[value(name = "discord-webhook")]
    DiscordWebhook,
    #[value(name = "slack")]
    Slack,
    #[value(name = "signal")]
    Signal,
    #[value(name = "whatsapp")]
    WhatsApp,
    #[value(name = "telegram")]
    Telegram,
}

impl RouteProvider {
    pub const ALL: [Self; 6] = [
        Self::Discord,
        Self::DiscordWebhook,
        Self::Slack,
        Self::Signal,
        Self::WhatsApp,
        Self::Telegram,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discord => "discord",
            Self::DiscordWebhook => "discord-webhook",
            Self::Slack => "slack",
            Self::Signal => "signal",
            Self::WhatsApp => "whatsapp",
            Self::Telegram => "telegram",
        }
    }
}

impl fmt::Display for RouteProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A named route pointing to a provider and target plus secrets.
///
/// Each secret can be stored directly (e.g. `bot_token`) or referenced
/// via an environment variable name (e.g. `bot_token_env`). Direct values
/// take priority over env var lookups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteConfig {
    Discord {
        channel_id: String,
        bot_token: Option<String>,
        bot_token_env: String,
    },
    DiscordWebhook {
        webhook_url: Option<String>,
        webhook_url_env: String,
    },
    Slack {
        channel_id: String,
        bot_token: Option<String>,
        bot_token_env: String,
    },
    Signal {
        recipient: String,
        rpc_url: Option<String>,
        rpc_url_env: String,
        account: Option<String>,
        account_env: String,
    },
    WhatsApp {
        recipient: String,
        access_token: Option<String>,
        access_token_env: String,
        phone_number_id: Option<String>,
        phone_number_id_env: String,
    },
    Telegram {
        chat_id: String,
        bot_token: Option<String>,
        bot_token_env: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "provider", rename_all = "lowercase")]
enum RouteConfigRepr {
    Discord {
        channel_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        #[serde(default = "default_discord_token_env")]
        bot_token_env: String,
    },
    #[serde(rename = "discord-webhook")]
    DiscordWebhook {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        #[serde(default = "default_discord_webhook_url_env")]
        webhook_url_env: String,
    },
    Slack {
        channel_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        #[serde(default = "default_slack_token_env")]
        bot_token_env: String,
    },
    Signal {
        recipient: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rpc_url: Option<String>,
        #[serde(default = "default_signal_rpc_url_env")]
        rpc_url_env: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account: Option<String>,
        #[serde(default = "default_signal_account_env")]
        account_env: String,
    },
    WhatsApp {
        recipient: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        access_token: Option<String>,
        #[serde(default = "default_whatsapp_access_token_env")]
        access_token_env: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phone_number_id: Option<String>,
        #[serde(default = "default_whatsapp_phone_number_id_env")]
        phone_number_id_env: String,
    },
    Telegram {
        chat_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        #[serde(default = "default_telegram_token_env")]
        bot_token_env: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyRouteConfig {
    provider: RouteProvider,
    channel_id: String,
    #[serde(default)]
    token_env: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RouteConfigSerde {
    New(RouteConfigRepr),
    Legacy(LegacyRouteConfig),
}

impl RouteConfig {
    pub fn provider(&self) -> RouteProvider {
        match self {
            Self::Discord { .. } => RouteProvider::Discord,
            Self::DiscordWebhook { .. } => RouteProvider::DiscordWebhook,
            Self::Slack { .. } => RouteProvider::Slack,
            Self::Signal { .. } => RouteProvider::Signal,
            Self::WhatsApp { .. } => RouteProvider::WhatsApp,
            Self::Telegram { .. } => RouteProvider::Telegram,
        }
    }

    pub fn from_provider_and_target(provider: RouteProvider, target_id: impl Into<String>) -> Self {
        let target_id = target_id.into();
        match provider {
            RouteProvider::Discord => Self::Discord {
                channel_id: target_id,
                bot_token: None,
                bot_token_env: default_discord_token_env(),
            },
            RouteProvider::DiscordWebhook => Self::DiscordWebhook {
                webhook_url: Some(target_id),
                webhook_url_env: default_discord_webhook_url_env(),
            },
            RouteProvider::Slack => Self::Slack {
                channel_id: target_id,
                bot_token: None,
                bot_token_env: default_slack_token_env(),
            },
            RouteProvider::Signal => Self::Signal {
                recipient: target_id,
                rpc_url: None,
                rpc_url_env: default_signal_rpc_url_env(),
                account: None,
                account_env: default_signal_account_env(),
            },
            RouteProvider::WhatsApp => Self::WhatsApp {
                recipient: target_id,
                access_token: None,
                access_token_env: default_whatsapp_access_token_env(),
                phone_number_id: None,
                phone_number_id_env: default_whatsapp_phone_number_id_env(),
            },
            RouteProvider::Telegram => Self::Telegram {
                chat_id: target_id,
                bot_token: None,
                bot_token_env: default_telegram_token_env(),
            },
        }
    }
}

impl Serialize for RouteConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        RouteConfigRepr::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RouteConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RouteConfigSerde::deserialize(deserializer)?;
        Ok(match raw {
            RouteConfigSerde::New(config) => config.into(),
            RouteConfigSerde::Legacy(config) => config.into(),
        })
    }
}

impl From<RouteConfigRepr> for RouteConfig {
    fn from(value: RouteConfigRepr) -> Self {
        match value {
            RouteConfigRepr::Discord {
                channel_id,
                bot_token,
                bot_token_env,
            } => Self::Discord {
                channel_id,
                bot_token,
                bot_token_env,
            },
            RouteConfigRepr::DiscordWebhook {
                webhook_url,
                webhook_url_env,
            } => Self::DiscordWebhook {
                webhook_url,
                webhook_url_env,
            },
            RouteConfigRepr::Slack {
                channel_id,
                bot_token,
                bot_token_env,
            } => Self::Slack {
                channel_id,
                bot_token,
                bot_token_env,
            },
            RouteConfigRepr::Signal {
                recipient,
                rpc_url,
                rpc_url_env,
                account,
                account_env,
            } => Self::Signal {
                recipient,
                rpc_url,
                rpc_url_env,
                account,
                account_env,
            },
            RouteConfigRepr::WhatsApp {
                recipient,
                access_token,
                access_token_env,
                phone_number_id,
                phone_number_id_env,
            } => Self::WhatsApp {
                recipient,
                access_token,
                access_token_env,
                phone_number_id,
                phone_number_id_env,
            },
            RouteConfigRepr::Telegram {
                chat_id,
                bot_token,
                bot_token_env,
            } => Self::Telegram {
                chat_id,
                bot_token,
                bot_token_env,
            },
        }
    }
}

impl From<RouteConfig> for RouteConfigRepr {
    fn from(value: RouteConfig) -> Self {
        match value {
            RouteConfig::Discord {
                channel_id,
                bot_token,
                bot_token_env,
            } => Self::Discord {
                channel_id,
                bot_token,
                bot_token_env,
            },
            RouteConfig::DiscordWebhook {
                webhook_url,
                webhook_url_env,
            } => Self::DiscordWebhook {
                webhook_url,
                webhook_url_env,
            },
            RouteConfig::Slack {
                channel_id,
                bot_token,
                bot_token_env,
            } => Self::Slack {
                channel_id,
                bot_token,
                bot_token_env,
            },
            RouteConfig::Signal {
                recipient,
                rpc_url,
                rpc_url_env,
                account,
                account_env,
            } => Self::Signal {
                recipient,
                rpc_url,
                rpc_url_env,
                account,
                account_env,
            },
            RouteConfig::WhatsApp {
                recipient,
                access_token,
                access_token_env,
                phone_number_id,
                phone_number_id_env,
            } => Self::WhatsApp {
                recipient,
                access_token,
                access_token_env,
                phone_number_id,
                phone_number_id_env,
            },
            RouteConfig::Telegram {
                chat_id,
                bot_token,
                bot_token_env,
            } => Self::Telegram {
                chat_id,
                bot_token,
                bot_token_env,
            },
        }
    }
}

impl From<LegacyRouteConfig> for RouteConfig {
    fn from(value: LegacyRouteConfig) -> Self {
        let token_env = if value.token_env.is_empty() {
            None
        } else {
            Some(value.token_env)
        };

        match value.provider {
            RouteProvider::Discord => Self::Discord {
                channel_id: value.channel_id,
                bot_token: None,
                bot_token_env: token_env.unwrap_or_else(default_discord_token_env),
            },
            RouteProvider::DiscordWebhook => Self::DiscordWebhook {
                webhook_url: None,
                webhook_url_env: token_env.unwrap_or_else(default_discord_webhook_url_env),
            },
            RouteProvider::Slack => Self::Slack {
                channel_id: value.channel_id,
                bot_token: None,
                bot_token_env: token_env.unwrap_or_else(default_slack_token_env),
            },
            RouteProvider::Signal => Self::Signal {
                recipient: value.channel_id,
                rpc_url: None,
                rpc_url_env: token_env.unwrap_or_else(default_signal_rpc_url_env),
                account: None,
                account_env: default_signal_account_env(),
            },
            RouteProvider::WhatsApp => Self::WhatsApp {
                recipient: value.channel_id,
                access_token: None,
                access_token_env: token_env.unwrap_or_else(default_whatsapp_access_token_env),
                phone_number_id: None,
                phone_number_id_env: default_whatsapp_phone_number_id_env(),
            },
            RouteProvider::Telegram => Self::Telegram {
                chat_id: value.channel_id,
                bot_token: None,
                bot_token_env: token_env.unwrap_or_else(default_telegram_token_env),
            },
        }
    }
}

fn default_discord_token_env() -> String {
    "DISCORD_BOT_TOKEN".into()
}

fn default_discord_webhook_url_env() -> String {
    "DISCORD_WEBHOOK_URL".into()
}

fn default_slack_token_env() -> String {
    "SLACK_BOT_TOKEN".into()
}

fn default_signal_rpc_url_env() -> String {
    "SIGNAL_RPC_URL".into()
}

fn default_signal_account_env() -> String {
    "SIGNAL_ACCOUNT".into()
}

fn default_whatsapp_access_token_env() -> String {
    "WHATSAPP_ACCESS_TOKEN".into()
}

fn default_whatsapp_phone_number_id_env() -> String {
    "WHATSAPP_PHONE_NUMBER_ID".into()
}

fn default_telegram_token_env() -> String {
    "TELEGRAM_BOT_TOKEN".into()
}

impl Config {
    /// Load config from `~/.messenger.json`, returning default if not found.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from_path(&path)
    }

    /// Load config from a specific path, returning default if not found.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        tracing::debug!(path = %path.display(), "loading config");
        if !path.exists() {
            tracing::debug!(path = %path.display(), "config file not found, using defaults");
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let config: Config = match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(error) => {
                tracing::warn!(path = %path.display(), error = %error, "failed to parse config");
                return Err(error.into());
            }
        };
        tracing::debug!(
            path = %path.display(),
            route_count = config.routes.len(),
            has_default_route = config.default_route.is_some(),
            "loaded config"
        );
        Ok(config)
    }

    /// Save config to `~/.messenger.json`.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        self.save_to_path(&path)
    }

    /// Save config to a specific path.
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        tracing::debug!(
            path = %path.display(),
            route_count = self.routes.len(),
            has_default_route = self.default_route.is_some(),
            "saving config"
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        tracing::debug!(path = %path.display(), "saved config");
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("could not determine home directory"))?;
        Ok(home.join(".messenger.json"))
    }

    /// Return all route names that use the given provider.
    pub fn routes_for_provider(&self, provider: RouteProvider) -> Vec<String> {
        self.routes
            .iter()
            .filter(|(_, route)| route.provider() == provider)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn legacy_route_config_migrates_to_typed_shape() {
        let raw = r#"{
            "provider": "signal",
            "channel_id": "+15551234567",
            "token_env": "CUSTOM_SIGNAL_RPC_URL"
        }"#;

        let route: RouteConfig = serde_json::from_str(raw).unwrap();
        assert_eq!(
            route,
            RouteConfig::Signal {
                recipient: "+15551234567".into(),
                rpc_url: None,
                rpc_url_env: "CUSTOM_SIGNAL_RPC_URL".into(),
                account: None,
                account_env: "SIGNAL_ACCOUNT".into(),
            }
        );
    }

    #[test]
    fn discord_webhook_route_round_trips_with_both_fields() {
        let cfg = RouteConfig::DiscordWebhook {
            webhook_url: Some("https://discord.com/api/v10/webhooks/1/abc".into()),
            webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: RouteConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn discord_webhook_route_round_trips_with_env_only() {
        let cfg = RouteConfig::DiscordWebhook {
            webhook_url: None,
            webhook_url_env: "CUSTOM_WEBHOOK_ENV".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: RouteConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn discord_webhook_route_applies_default_env_when_absent() {
        let raw = r#"{"provider":"discord-webhook","webhook_url":"https://discord.com/api/v10/webhooks/1/abc"}"#;
        let parsed: RouteConfig = serde_json::from_str(raw).unwrap();
        assert_eq!(
            parsed,
            RouteConfig::DiscordWebhook {
                webhook_url: Some("https://discord.com/api/v10/webhooks/1/abc".into()),
                webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
            }
        );
    }

    #[test]
    fn discord_webhook_route_applies_default_when_both_fields_absent() {
        let raw = r#"{"provider":"discord-webhook"}"#;
        let parsed: RouteConfig = serde_json::from_str(raw).unwrap();
        assert_eq!(
            parsed,
            RouteConfig::DiscordWebhook {
                webhook_url: None,
                webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
            }
        );
    }

    #[test]
    fn config_round_trips_with_typed_route_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("messenger.json");

        let mut config = Config {
            default_route: Some("signal.ops".into()),
            routes: HashMap::new(),
        };
        config.routes.insert(
            "signal.ops".into(),
            RouteConfig::Signal {
                recipient: "+15551234567".into(),
                rpc_url: None,
                rpc_url_env: "SIGNAL_RPC_URL".into(),
                account: None,
                account_env: "SIGNAL_ACCOUNT".into(),
            },
        );

        config.save_to_path(&path).unwrap();
        let loaded = Config::load_from_path(&path).unwrap();

        assert_eq!(loaded, config);
    }
}
