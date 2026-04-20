/// A destination for a message dispatch.
#[derive(Debug, Clone)]
pub enum Target {
    #[cfg(feature = "discord")]
    Discord(DiscordTarget),
    #[cfg(feature = "discord")]
    DiscordWebhook(DiscordWebhookTarget),
    #[cfg(feature = "slack")]
    Slack(SlackTarget),
    #[cfg(feature = "signal")]
    Signal(SignalTarget),
    #[cfg(feature = "whatsapp")]
    WhatsApp(WhatsAppTarget),
    #[cfg(feature = "telegram")]
    Telegram(TelegramTarget),
}

/// Discord channel target.
#[cfg(feature = "discord")]
#[derive(Debug, Clone)]
pub struct DiscordTarget {
    pub channel_id: String,
}

/// Discord webhook target.
///
/// The webhook URL (configured on the provider) binds the channel and
/// authentication. Only `thread_id` is a dispatch-time concern: supplying it
/// routes the message into a specific thread within the webhook's channel.
#[cfg(feature = "discord")]
#[derive(Debug, Clone, Default)]
pub struct DiscordWebhookTarget {
    pub thread_id: Option<String>,
}

/// Slack channel target.
#[cfg(feature = "slack")]
#[derive(Debug, Clone)]
pub struct SlackTarget {
    pub channel_id: String,
}

/// Signal destination.
#[cfg(feature = "signal")]
#[derive(Debug, Clone)]
pub enum SignalTarget {
    User(SignalAddress),
    Group { group_id_base64: String },
    NoteToSelf,
}

/// A Signal user address (phone number or UUID).
#[cfg(feature = "signal")]
#[derive(Debug, Clone)]
pub enum SignalAddress {
    Phone(String),
    Uuid(String),
}

/// WhatsApp recipient.
#[cfg(feature = "whatsapp")]
#[derive(Debug, Clone)]
pub struct WhatsAppTarget {
    pub recipient: String,
}

/// Telegram chat target.
#[cfg(feature = "telegram")]
#[derive(Debug, Clone)]
pub struct TelegramTarget {
    pub chat_id: TelegramChatId,
    pub thread_id: Option<i64>,
}

/// Telegram chat identifier (numeric or username).
#[cfg(feature = "telegram")]
#[derive(Debug, Clone)]
pub enum TelegramChatId {
    Id(i64),
    Username(String),
}

// Convenience constructors on Target
impl Target {
    #[cfg(feature = "discord")]
    pub fn discord_channel(channel_id: impl Into<String>) -> Self {
        Self::Discord(DiscordTarget {
            channel_id: channel_id.into(),
        })
    }

    /// Build a Discord webhook target with no thread routing.
    #[cfg(feature = "discord")]
    pub fn discord_webhook() -> Self {
        Self::DiscordWebhook(DiscordWebhookTarget { thread_id: None })
    }

    /// Build a Discord webhook target routed to a specific thread.
    #[cfg(feature = "discord")]
    pub fn discord_webhook_thread(thread_id: impl Into<String>) -> Self {
        Self::DiscordWebhook(DiscordWebhookTarget {
            thread_id: Some(thread_id.into()),
        })
    }

    #[cfg(feature = "slack")]
    pub fn slack_channel(channel_id: impl Into<String>) -> Self {
        Self::Slack(SlackTarget {
            channel_id: channel_id.into(),
        })
    }

    #[cfg(feature = "signal")]
    pub fn signal_user(address: SignalAddress) -> Self {
        Self::Signal(SignalTarget::User(address))
    }

    #[cfg(feature = "signal")]
    pub fn signal_group(group_id_base64: impl Into<String>) -> Self {
        Self::Signal(SignalTarget::Group {
            group_id_base64: group_id_base64.into(),
        })
    }

    #[cfg(feature = "whatsapp")]
    pub fn whatsapp_recipient(recipient: impl Into<String>) -> Self {
        Self::WhatsApp(WhatsAppTarget {
            recipient: recipient.into(),
        })
    }

    #[cfg(feature = "telegram")]
    pub fn telegram_chat(chat_id: TelegramChatId) -> Self {
        Self::Telegram(TelegramTarget {
            chat_id,
            thread_id: None,
        })
    }
}
