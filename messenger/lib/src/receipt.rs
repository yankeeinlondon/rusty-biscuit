use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::MessengerError;

/// Identifies which messaging provider was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Discord,
    #[serde(rename = "discord-webhook")]
    DiscordWebhook,
    Slack,
    #[serde(rename = "slack-webhook")]
    SlackWebhook,
    Signal,
    WhatsApp,
    Telegram,
    Desktop,
}

impl ProviderKind {
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

    /// Create a [`MessengerError::Transport`] for this provider.
    pub fn transport_error(self, message: impl fmt::Display) -> MessengerError {
        MessengerError::Transport {
            provider: self,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discord => write!(f, "Discord"),
            Self::DiscordWebhook => write!(f, "Discord-Webhook"),
            Self::Slack => write!(f, "Slack"),
            Self::SlackWebhook => write!(f, "Slack-Webhook"),
            Self::Signal => write!(f, "Signal"),
            Self::WhatsApp => write!(f, "WhatsApp"),
            Self::Telegram => write!(f, "Telegram"),
            Self::Desktop => write!(f, "Desktop"),
        }
    }
}

/// Host OS targeted by a desktop notification delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DesktopPlatform {
    #[serde(rename = "macos")]
    MacOS,
    Linux,
    Windows,
}

impl fmt::Display for DesktopPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MacOS => write!(f, "macOS"),
            Self::Linux => write!(f, "Linux"),
            Self::Windows => write!(f, "Windows"),
        }
    }
}

/// A provider-typed reference to a sent message, usable for replies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRef {
    Discord {
        channel_id: String,
        message_id: String,
    },
    #[serde(rename = "discord-webhook")]
    DiscordWebhook {
        webhook_id: String,
        channel_id: String,
        message_id: String,
        thread_id: Option<String>,
    },
    Slack {
        channel_id: String,
        thread_ts: String,
    },
    #[serde(rename = "slack-webhook")]
    SlackWebhook {
        thread_ts: Option<String>,
    },
    Signal {
        thread: SignalThreadKey,
        author: SignalAuthor,
        timestamp_ms: i64,
    },
    WhatsApp {
        message_id: String,
    },
    Telegram {
        chat_id: TelegramChatRef,
        message_id: i64,
        thread_id: Option<i64>,
    },
    Desktop {
        platform: DesktopPlatform,
        notification_id: String,
    },
}

impl MessageRef {
    /// Return the provider kind for this reference.
    pub fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::Discord { .. } => ProviderKind::Discord,
            Self::DiscordWebhook { .. } => ProviderKind::DiscordWebhook,
            Self::Slack { .. } => ProviderKind::Slack,
            Self::SlackWebhook { .. } => ProviderKind::SlackWebhook,
            Self::Signal { .. } => ProviderKind::Signal,
            Self::WhatsApp { .. } => ProviderKind::WhatsApp,
            Self::Telegram { .. } => ProviderKind::Telegram,
            Self::Desktop { .. } => ProviderKind::Desktop,
        }
    }

    /// Parse a message reference from JSON.
    pub fn from_json_str(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    /// Serialize a message reference to pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Signal thread identifier for replies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalThreadKey {
    Direct(String),
    Group(String),
}

/// Signal message author for reply quoting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalAuthor {
    Phone(String),
    Uuid(String),
}

/// Telegram chat reference for receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TelegramChatRef {
    Id(i64),
    Username(String),
}

/// Proof of delivery returned after a successful send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendReceipt {
    pub provider: ProviderKind,
    pub message_ref: MessageRef,
    pub raw_id: String,
    pub metadata: BTreeMap<String, String>,
}

impl SendReceipt {
    /// Parse a send receipt from JSON.
    pub fn from_json_str(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    /// Serialize a send receipt to pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
