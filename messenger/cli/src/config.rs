use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use color_eyre::eyre::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Desktop notification action persisted in route config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
}

/// Desktop notification progress indicator persisted in route config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationProgress {
    pub current: u32,
    pub total: u32,
}

/// Portable urgency levels that match `messenger::NotificationUrgency`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum RouteUrgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl RouteUrgency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::Critical => "critical",
        }
    }
}

impl fmt::Display for RouteUrgency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Selector for the macOS notification delivery strategy in route config.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum RouteMacOsStrategy {
    #[default]
    Auto,
    NativeUserNotifications,
    AppleScript,
}

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
    #[serde(rename = "slack-webhook")]
    #[value(name = "slack-webhook")]
    SlackWebhook,
    #[value(name = "signal")]
    Signal,
    #[value(name = "whatsapp")]
    WhatsApp,
    #[value(name = "telegram")]
    Telegram,
    #[value(name = "desktop")]
    Desktop,
}

impl RouteProvider {
    pub const ALL: [Self; 8] = [
        Self::Discord,
        Self::DiscordWebhook,
        Self::Slack,
        Self::SlackWebhook,
        Self::Signal,
        Self::WhatsApp,
        Self::Telegram,
        Self::Desktop,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discord => "discord",
            Self::DiscordWebhook => "discord-webhook",
            Self::Slack => "slack",
            Self::SlackWebhook => "slack-webhook",
            Self::Signal => "signal",
            Self::WhatsApp => "whatsapp",
            Self::Telegram => "telegram",
            Self::Desktop => "desktop",
        }
    }

    /// Whether this provider requires a `--channel` / target identifier when
    /// used as an ad-hoc route via `--provider`.
    ///
    /// Desktop notifications are always delivered to the host OS notification
    /// center and therefore have no target identifier. Every chat provider
    /// still needs an explicit channel, recipient, or chat id.
    pub fn requires_target(self) -> bool {
        !matches!(self, Self::Desktop)
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
    SlackWebhook {
        webhook_url: Option<String>,
        webhook_url_env: String,
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
    Desktop {
        app_name: String,
        default_title: Option<String>,
        icon: Option<String>,
        category: Option<String>,
        urgency: RouteUrgency,
        timeout_ms: Option<u32>,
        actions: Vec<NotificationAction>,
        progress: Option<NotificationProgress>,
        badge_count: Option<u32>,
        windows: DesktopWindowsConfig,
        macos: DesktopMacOsConfig,
        linux: DesktopLinuxConfig,
    },
}

/// Windows-specific fields persisted in the desktop route config.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopWindowsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    /// Caller-supplied helper preference order (e.g. `["snore_toast"]`).
    ///
    /// Names are parsed via [`sniff::programs::NotificationHelper`]'s
    /// `FromStr` impl. Unknown names are dropped at conversion time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefer_helpers: Vec<String>,
}

/// macOS-specific fields persisted in the desktop route config.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopMacOsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(default)]
    pub strategy: RouteMacOsStrategy,
    /// Caller-supplied helper preference order (e.g. `["alerter"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefer_helpers: Vec<String>,
}

/// Linux-specific fields persisted in the desktop route config.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopLinuxConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_entry: Option<String>,
    /// Caller-supplied helper preference order (e.g. `["dunstify"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefer_helpers: Vec<String>,
}

/// Environment variable that overrides desktop `prefer_helpers` for all OSes.
///
/// Values are comma-separated (e.g. `"dunstify,notify_send"`). Names that do
/// not match a known [`sniff::programs::NotificationHelper`] variant are
/// silently dropped. When set, the env-derived list **prepends** the
/// per-OS config list during conversion to the messenger library config.
pub const PREFER_HELPERS_ENV: &str = "MESSENGER_DESKTOP_PREFER_HELPERS";

/// Parse a helper name string, accepting both the strum snake_case form and
/// the helper's [`binary_name`](sniff::programs::ProgramMetadata::binary_name)
/// (e.g. both `"terminal_notifier"` and `"terminal-notifier"` resolve to
/// [`sniff::programs::NotificationHelper::TerminalNotifier`]).
pub fn parse_helper_name(name: &str) -> Option<sniff::programs::NotificationHelper> {
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;

    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(helper) = trimmed.parse::<sniff::programs::NotificationHelper>() {
        return Some(helper);
    }
    let lower = trimmed.to_lowercase();
    sniff::programs::NotificationHelper::iter()
        .find(|h| h.binary_name().to_lowercase() == lower)
}

/// Resolve the helper preference list for a desktop send.
///
/// Combines the env-var override (parsed once) with the per-OS config list,
/// dropping names that do not parse as a known
/// [`sniff::programs::NotificationHelper`]. Order: env-var entries first,
/// then config entries; duplicates are removed (first occurrence wins).
pub fn resolve_prefer_helpers(
    config_helpers: &[String],
) -> Vec<sniff::programs::NotificationHelper> {
    let env_helpers = std::env::var(PREFER_HELPERS_ENV)
        .ok()
        .map(|raw| {
            raw.split(',')
                .filter_map(parse_helper_name)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut combined: Vec<sniff::programs::NotificationHelper> = Vec::new();
    for helper in env_helpers
        .into_iter()
        .chain(config_helpers.iter().filter_map(|s| parse_helper_name(s)))
    {
        if !combined.contains(&helper) {
            combined.push(helper);
        }
    }
    combined
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
    #[serde(rename = "slack-webhook")]
    SlackWebhook {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        #[serde(default = "default_slack_webhook_url_env")]
        webhook_url_env: String,
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
    Desktop {
        #[serde(default = "default_desktop_app_name")]
        app_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default_title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        category: Option<String>,
        #[serde(default)]
        urgency: RouteUrgency,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        actions: Vec<NotificationAction>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        progress: Option<NotificationProgress>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        badge_count: Option<u32>,
        #[serde(default, skip_serializing_if = "is_default_windows")]
        windows: DesktopWindowsConfig,
        #[serde(default, skip_serializing_if = "is_default_macos")]
        macos: DesktopMacOsConfig,
        #[serde(default, skip_serializing_if = "is_default_linux")]
        linux: DesktopLinuxConfig,
    },
}

fn default_desktop_app_name() -> String {
    "Messenger".into()
}

fn is_default_windows(value: &DesktopWindowsConfig) -> bool {
    value == &DesktopWindowsConfig::default()
}

fn is_default_macos(value: &DesktopMacOsConfig) -> bool {
    value == &DesktopMacOsConfig::default()
}

fn is_default_linux(value: &DesktopLinuxConfig) -> bool {
    value == &DesktopLinuxConfig::default()
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyRouteConfig {
    provider: RouteProvider,
    channel_id: String,
    #[serde(default)]
    token_env: String,
}

// `RouteConfigSerde` exists only to bridge the legacy single-shape config
// into the typed enum via serde's untagged deserialization. The variants
// have very different sizes by design (the legacy shape is a tiny migration
// shim) and the enum is constructed once per deserialize call, so the size
// asymmetry has no runtime cost.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
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
            Self::SlackWebhook { .. } => RouteProvider::SlackWebhook,
            Self::Signal { .. } => RouteProvider::Signal,
            Self::WhatsApp { .. } => RouteProvider::WhatsApp,
            Self::Telegram { .. } => RouteProvider::Telegram,
            Self::Desktop { .. } => RouteProvider::Desktop,
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
            RouteProvider::SlackWebhook => Self::SlackWebhook {
                webhook_url: Some(target_id),
                webhook_url_env: default_slack_webhook_url_env(),
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
            RouteProvider::Desktop => Self::desktop_default(),
        }
    }

    /// Build a desktop route filled with platform defaults.
    pub fn desktop_default() -> Self {
        Self::Desktop {
            app_name: default_desktop_app_name(),
            default_title: None,
            icon: None,
            category: None,
            urgency: RouteUrgency::default(),
            timeout_ms: None,
            actions: Vec::new(),
            progress: None,
            badge_count: None,
            windows: DesktopWindowsConfig::default(),
            macos: DesktopMacOsConfig::default(),
            linux: DesktopLinuxConfig::default(),
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
            RouteConfigRepr::SlackWebhook {
                webhook_url,
                webhook_url_env,
            } => Self::SlackWebhook {
                webhook_url,
                webhook_url_env,
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
            RouteConfigRepr::Desktop {
                app_name,
                default_title,
                icon,
                category,
                urgency,
                timeout_ms,
                actions,
                progress,
                badge_count,
                windows,
                macos,
                linux,
            } => Self::Desktop {
                app_name,
                default_title,
                icon,
                category,
                urgency,
                timeout_ms,
                actions,
                progress,
                badge_count,
                windows,
                macos,
                linux,
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
            RouteConfig::SlackWebhook {
                webhook_url,
                webhook_url_env,
            } => Self::SlackWebhook {
                webhook_url,
                webhook_url_env,
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
            RouteConfig::Desktop {
                app_name,
                default_title,
                icon,
                category,
                urgency,
                timeout_ms,
                actions,
                progress,
                badge_count,
                windows,
                macos,
                linux,
            } => Self::Desktop {
                app_name,
                default_title,
                icon,
                category,
                urgency,
                timeout_ms,
                actions,
                progress,
                badge_count,
                windows,
                macos,
                linux,
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
            RouteProvider::SlackWebhook => Self::SlackWebhook {
                webhook_url: None,
                webhook_url_env: token_env.unwrap_or_else(default_slack_webhook_url_env),
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
            // The legacy single-shape config never carried a desktop route, so
            // we simply materialize a defaulted desktop route here; any legacy
            // field values like `channel_id` are irrelevant for desktop.
            RouteProvider::Desktop => Self::desktop_default(),
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

fn default_slack_webhook_url_env() -> String {
    "SLACK_WEBHOOK_URL".into()
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
    fn slack_webhook_route_round_trips_with_both_fields() {
        let cfg = RouteConfig::SlackWebhook {
            webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXXX".into()),
            webhook_url_env: "SLACK_WEBHOOK_URL".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: RouteConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn slack_webhook_route_round_trips_with_env_only() {
        let cfg = RouteConfig::SlackWebhook {
            webhook_url: None,
            webhook_url_env: "CUSTOM_SLACK_WEBHOOK_URL".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: RouteConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn slack_webhook_route_applies_default_env_when_absent() {
        let raw = r#"{"provider":"slack-webhook","webhook_url":"https://hooks.slack.com/services/T000/B000/XXXXX"}"#;
        let parsed: RouteConfig = serde_json::from_str(raw).unwrap();
        assert_eq!(
            parsed,
            RouteConfig::SlackWebhook {
                webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXXX".into()),
                webhook_url_env: "SLACK_WEBHOOK_URL".into(),
            }
        );
    }

    #[test]
    fn slack_webhook_route_applies_default_when_both_fields_absent() {
        let raw = r#"{"provider":"slack-webhook"}"#;
        let parsed: RouteConfig = serde_json::from_str(raw).unwrap();
        assert_eq!(
            parsed,
            RouteConfig::SlackWebhook {
                webhook_url: None,
                webhook_url_env: "SLACK_WEBHOOK_URL".into(),
            }
        );
    }

    #[test]
    fn slack_webhook_route_reports_provider_kind() {
        let route = RouteConfig::SlackWebhook {
            webhook_url: None,
            webhook_url_env: "SLACK_WEBHOOK_URL".into(),
        };
        assert_eq!(route.provider(), RouteProvider::SlackWebhook);
    }

    #[test]
    fn slack_webhook_route_builds_from_provider_and_target() {
        let route = RouteConfig::from_provider_and_target(
            RouteProvider::SlackWebhook,
            "https://hooks.slack.com/services/T000/B000/XXXXX",
        );
        assert_eq!(
            route,
            RouteConfig::SlackWebhook {
                webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXXX".into()),
                webhook_url_env: "SLACK_WEBHOOK_URL".into(),
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

    #[test]
    fn parse_helper_name_accepts_strum_form() {
        assert_eq!(
            parse_helper_name("dunstify"),
            Some(sniff::programs::NotificationHelper::Dunstify)
        );
        assert_eq!(
            parse_helper_name("notify_send"),
            Some(sniff::programs::NotificationHelper::NotifySend)
        );
        assert_eq!(
            parse_helper_name("terminal_notifier"),
            Some(sniff::programs::NotificationHelper::TerminalNotifier)
        );
    }

    #[test]
    fn parse_helper_name_accepts_binary_form() {
        // `binary_name()` uses dashes for some helpers — accept that too so
        // users can configure the same string they'd type at the shell.
        assert_eq!(
            parse_helper_name("notify-send"),
            Some(sniff::programs::NotificationHelper::NotifySend)
        );
        assert_eq!(
            parse_helper_name("terminal-notifier"),
            Some(sniff::programs::NotificationHelper::TerminalNotifier)
        );
    }

    #[test]
    fn parse_helper_name_returns_none_for_unknown() {
        assert!(parse_helper_name("not-a-helper").is_none());
        assert!(parse_helper_name("").is_none());
        assert!(parse_helper_name("   ").is_none());
    }

    #[test]
    fn desktop_route_parses_prefer_helpers_array() {
        let raw = r#"{
            "provider": "desktop",
            "linux": { "prefer_helpers": ["dunstify", "notify_send"] }
        }"#;
        let parsed: RouteConfig = serde_json::from_str(raw).unwrap();
        match parsed {
            RouteConfig::Desktop { linux, .. } => {
                assert_eq!(linux.prefer_helpers, vec!["dunstify", "notify_send"]);
            }
            other => panic!("expected RouteConfig::Desktop, got {other:?}"),
        }
    }

    #[test]
    fn desktop_route_round_trips_prefer_helpers_for_each_os() {
        let route = RouteConfig::Desktop {
            app_name: "Messenger".into(),
            default_title: None,
            icon: None,
            category: None,
            urgency: RouteUrgency::Normal,
            timeout_ms: None,
            actions: Vec::new(),
            progress: None,
            badge_count: None,
            windows: DesktopWindowsConfig {
                app_id: None,
                prefer_helpers: vec!["snore_toast".into()],
            },
            macos: DesktopMacOsConfig {
                bundle_id: None,
                strategy: RouteMacOsStrategy::Auto,
                prefer_helpers: vec!["alerter".into()],
            },
            linux: DesktopLinuxConfig {
                desktop_entry: None,
                prefer_helpers: vec!["dunstify".into()],
            },
        };
        let json = serde_json::to_string(&route).unwrap();
        let parsed: RouteConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, route);
    }

    #[test]
    fn resolve_prefer_helpers_drops_unknown_names() {
        // Run with the env var unset so only the config list is considered.
        unsafe {
            std::env::remove_var(PREFER_HELPERS_ENV);
        }
        let resolved = resolve_prefer_helpers(&[
            "dunstify".into(),
            "not-a-helper".into(),
            "notify-send".into(),
        ]);
        assert_eq!(
            resolved,
            vec![
                sniff::programs::NotificationHelper::Dunstify,
                sniff::programs::NotificationHelper::NotifySend,
            ]
        );
    }

    #[test]
    #[serial_test::serial]
    fn resolve_prefer_helpers_env_var_takes_precedence() {
        unsafe {
            std::env::set_var(PREFER_HELPERS_ENV, "alerter, terminal_notifier");
        }
        let resolved = resolve_prefer_helpers(&["dunstify".into()]);
        unsafe {
            std::env::remove_var(PREFER_HELPERS_ENV);
        }

        // Env entries appear first; config entry is appended afterwards.
        assert_eq!(
            resolved,
            vec![
                sniff::programs::NotificationHelper::Alerter,
                sniff::programs::NotificationHelper::TerminalNotifier,
                sniff::programs::NotificationHelper::Dunstify,
            ]
        );
    }

    #[test]
    #[serial_test::serial]
    fn resolve_prefer_helpers_dedupes_env_and_config_entries() {
        unsafe {
            std::env::set_var(PREFER_HELPERS_ENV, "dunstify");
        }
        let resolved = resolve_prefer_helpers(&["dunstify".into(), "notify_send".into()]);
        unsafe {
            std::env::remove_var(PREFER_HELPERS_ENV);
        }

        assert_eq!(
            resolved,
            vec![
                sniff::programs::NotificationHelper::Dunstify,
                sniff::programs::NotificationHelper::NotifySend,
            ]
        );
    }
}
