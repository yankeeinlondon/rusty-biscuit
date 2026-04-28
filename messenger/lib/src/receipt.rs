//! Receipt and reply-reference types returned from a successful send.
//!
//! [`SendReceipt`] is the canonical proof of delivery. It carries the
//! provider kind, a typed [`MessageRef`] for replying, the provider's raw
//! identifier string, and a free-form `metadata` map populated by the
//! adapter (delivery path, helper name, dropped-attachment hints, captured
//! activations).
//!
//! Most callers only need `provider`, `message_ref`, and `raw_id`. The
//! `metadata` map carries provider-specific signals that are easier to read
//! through the typed accessors:
//!
//! - [`SendReceipt::helper_used`] — name of the desktop helper that
//!   delivered the notification (or `None` when the native backend handled
//!   it).
//! - [`SendReceipt::activation`] — typed [`Activation`] decoded from
//!   `activation_type` / `activation_key` / `reply_text` metadata.
//! - [`SendReceipt::reply_text`] — convenience for the inline-reply case.

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
    #[serde(rename = "apns")]
    Apns,
    #[serde(rename = "fcm")]
    Fcm,
}

impl ProviderKind {
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
            Self::Apns => "apns",
            Self::Fcm => "fcm",
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
            Self::Apns => write!(f, "APNs"),
            Self::Fcm => write!(f, "FCM"),
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
    Apns {
        apns_id: String,
    },
    Fcm {
        message_id: String,
        project_id: String,
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
            Self::Apns { .. } => ProviderKind::Apns,
            Self::Fcm { .. } => ProviderKind::Fcm,
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

    /// Name of the notification helper that delivered this receipt.
    ///
    /// Returns `Some(name)` when a third-party desktop helper (such as
    /// `dunstify`, `terminal-notifier`, or `snoretoast`) was elected. Returns
    /// `None` for non-desktop providers, for the native desktop backend, or
    /// when the metadata key is absent.
    pub fn helper_used(&self) -> Option<&str> {
        self.metadata
            .get("helper_used")
            .map(String::as_str)
            .filter(|value| *value != "native")
    }

    /// User activation captured by the notification helper.
    ///
    /// Returns `None` when the receipt has no `activation_type` metadata —
    /// either because the provider is not a desktop helper or because the
    /// helper did not capture an activation (e.g. notice-only sends).
    pub fn activation(&self) -> Option<Activation<'_>> {
        let kind = self.metadata.get("activation_type")?.as_str();
        let activation = match kind {
            "action" => Activation::Action(
                self.metadata
                    .get("activation_key")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            "reply" => Activation::Reply(
                self.metadata
                    .get("reply_text")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            "dismissed" => Activation::Dismissed,
            "timeout" => Activation::Timeout,
            "content_clicked" => Activation::ContentClicked,
            _ => return None,
        };
        Some(activation)
    }

    /// Inline reply text captured by an interactive helper.
    ///
    /// Convenience for the common case of extracting a reply payload without
    /// matching on [`Activation`]. Returns `None` when the activation was not
    /// a reply or no `reply_text` metadata was recorded.
    pub fn reply_text(&self) -> Option<&str> {
        if self.metadata.get("activation_type")?.as_str() != "reply" {
            return None;
        }
        self.metadata.get("reply_text").map(String::as_str)
    }
}

/// User activation captured by a desktop notification helper.
///
/// Helpers that block until the user interacts with a toast (alerter on
/// macOS, dunstify on Linux, snoretoast / BurntToast on Windows) record the
/// outcome in receipt metadata. [`SendReceipt::activation`] decodes those
/// metadata fields into this typed enum so callers can match on the result
/// without parsing strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation<'a> {
    /// The user clicked an action button. The contained string is the action
    /// id supplied in the dispatch (`activation_key`).
    Action(&'a str),
    /// The user typed an inline reply. The contained string is the user's
    /// text.
    Reply(&'a str),
    /// The user dismissed the notification without activating it.
    Dismissed,
    /// The notification expired without user interaction.
    Timeout,
    /// The user clicked the body of the notification.
    ContentClicked,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helper_receipt(metadata: &[(&str, &str)]) -> SendReceipt {
        SendReceipt {
            provider: ProviderKind::Desktop,
            message_ref: MessageRef::Desktop {
                platform: DesktopPlatform::Linux,
                notification_id: "id-1".to_string(),
            },
            raw_id: "id-1".to_string(),
            metadata: metadata
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    #[test]
    fn helper_used_returns_helper_name() {
        let receipt = helper_receipt(&[("helper_used", "dunstify")]);
        assert_eq!(receipt.helper_used(), Some("dunstify"));
    }

    #[test]
    fn helper_used_treats_native_as_none() {
        let receipt = helper_receipt(&[("helper_used", "native")]);
        assert_eq!(receipt.helper_used(), None);
    }

    #[test]
    fn helper_used_absent_returns_none() {
        let receipt = helper_receipt(&[]);
        assert_eq!(receipt.helper_used(), None);
    }

    #[test]
    fn activation_action_with_key() {
        let receipt = helper_receipt(&[
            ("activation_type", "action"),
            ("activation_key", "approve"),
        ]);
        assert_eq!(receipt.activation(), Some(Activation::Action("approve")));
    }

    #[test]
    fn activation_reply_with_text() {
        let receipt = helper_receipt(&[
            ("activation_type", "reply"),
            ("reply_text", "ship it"),
        ]);
        assert_eq!(receipt.activation(), Some(Activation::Reply("ship it")));
        assert_eq!(receipt.reply_text(), Some("ship it"));
    }

    #[test]
    fn activation_dismissed() {
        let receipt = helper_receipt(&[("activation_type", "dismissed")]);
        assert_eq!(receipt.activation(), Some(Activation::Dismissed));
        assert_eq!(receipt.reply_text(), None);
    }

    #[test]
    fn activation_timeout() {
        let receipt = helper_receipt(&[("activation_type", "timeout")]);
        assert_eq!(receipt.activation(), Some(Activation::Timeout));
    }

    #[test]
    fn activation_content_clicked() {
        let receipt = helper_receipt(&[("activation_type", "content_clicked")]);
        assert_eq!(receipt.activation(), Some(Activation::ContentClicked));
    }

    #[test]
    fn activation_absent_returns_none() {
        let receipt = helper_receipt(&[]);
        assert_eq!(receipt.activation(), None);
        assert_eq!(receipt.reply_text(), None);
    }

    #[test]
    fn activation_unknown_kind_returns_none() {
        let receipt = helper_receipt(&[("activation_type", "mystery")]);
        assert_eq!(receipt.activation(), None);
    }

    #[test]
    fn reply_text_only_returned_for_reply_activation() {
        let receipt = helper_receipt(&[
            ("activation_type", "action"),
            ("reply_text", "leftover"),
        ]);
        assert_eq!(receipt.reply_text(), None);
    }
}
