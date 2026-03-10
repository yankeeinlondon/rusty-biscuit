use std::collections::BTreeMap;
use std::fmt;

/// Identifies which messaging provider was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    Discord,
    Slack,
    Signal,
    WhatsApp,
    Telegram,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discord => write!(f, "Discord"),
            Self::Slack => write!(f, "Slack"),
            Self::Signal => write!(f, "Signal"),
            Self::WhatsApp => write!(f, "WhatsApp"),
            Self::Telegram => write!(f, "Telegram"),
        }
    }
}

/// A provider-typed reference to a sent message, usable for replies.
#[derive(Debug, Clone)]
pub enum MessageRef {
    Discord {
        channel_id: String,
        message_id: String,
    },
    Slack {
        channel_id: String,
        thread_ts: String,
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
}

impl MessageRef {
    /// Return the provider kind for this reference.
    pub fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::Discord { .. } => ProviderKind::Discord,
            Self::Slack { .. } => ProviderKind::Slack,
            Self::Signal { .. } => ProviderKind::Signal,
            Self::WhatsApp { .. } => ProviderKind::WhatsApp,
            Self::Telegram { .. } => ProviderKind::Telegram,
        }
    }
}

/// Signal thread identifier for replies.
#[derive(Debug, Clone)]
pub enum SignalThreadKey {
    Direct(String),
    Group(String),
}

/// Signal message author for reply quoting.
#[derive(Debug, Clone)]
pub enum SignalAuthor {
    Phone(String),
    Uuid(String),
}

/// Telegram chat reference for receipts.
#[derive(Debug, Clone)]
pub enum TelegramChatRef {
    Id(i64),
    Username(String),
}

/// Proof of delivery returned after a successful send.
#[derive(Debug, Clone)]
pub struct SendReceipt {
    pub provider: ProviderKind,
    pub message_ref: MessageRef,
    pub raw_id: String,
    pub metadata: BTreeMap<String, String>,
}
