/// A destination for a message dispatch.
#[derive(Debug, Clone)]
pub enum Target {
    #[cfg(feature = "discord")]
    Discord(DiscordTarget),
    #[cfg(feature = "discord")]
    DiscordWebhook(DiscordWebhookTarget),
    #[cfg(feature = "slack")]
    Slack(SlackTarget),
    #[cfg(feature = "slack")]
    SlackWebhook(SlackWebhookTarget),
    #[cfg(feature = "signal")]
    Signal(SignalTarget),
    #[cfg(feature = "whatsapp")]
    WhatsApp(WhatsAppTarget),
    #[cfg(feature = "telegram")]
    Telegram(TelegramTarget),
    #[cfg(feature = "desktop")]
    Desktop(DesktopTarget),
    #[cfg(feature = "apns")]
    Apns(ApnsTarget),
    #[cfg(feature = "fcm")]
    Fcm(FcmTarget),
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

/// Slack webhook target.
///
/// The webhook URL (configured on the provider) binds the channel.
/// Reply threading comes from `dispatch.reply_to`.
#[cfg(feature = "slack")]
#[derive(Debug, Clone, Default)]
pub struct SlackWebhookTarget {}

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

/// Desktop notification destination.
///
/// Carries no channel or recipient — desktop sends always deliver to the
/// current host OS notification center. The empty struct preserves the
/// enum-variant-plus-typed-struct pattern used by the other targets so
/// future fields (e.g. transient routing hints) can land without breaking
/// pattern matches.
#[cfg(feature = "desktop")]
#[derive(Debug, Clone, Default)]
pub struct DesktopTarget {}

/// Apple Push Notification service target.
#[cfg(feature = "apns")]
#[derive(Debug, Clone)]
pub struct ApnsTarget {
    /// The hex-encoded device token for the iOS device.
    pub device_token: String,
}

/// Firebase Cloud Messaging target.
#[cfg(feature = "fcm")]
#[derive(Debug, Clone)]
pub struct FcmTarget {
    /// The FCM registration token for the Android device.
    pub device_token: String,
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

    #[cfg(feature = "slack")]
    pub fn slack_webhook() -> Self {
        Self::SlackWebhook(SlackWebhookTarget {})
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

    /// Build a desktop notification target for the current host OS.
    #[cfg(feature = "desktop")]
    pub fn desktop() -> Self {
        Self::Desktop(DesktopTarget::default())
    }

    /// Build an Apple Push Notification target for the given device token.
    #[cfg(feature = "apns")]
    pub fn apns(device_token: impl Into<String>) -> Self {
        Self::Apns(ApnsTarget {
            device_token: device_token.into(),
        })
    }

    /// Build a Firebase Cloud Messaging target for the given device token.
    #[cfg(feature = "fcm")]
    pub fn fcm(device_token: impl Into<String>) -> Self {
        Self::Fcm(FcmTarget {
            device_token: device_token.into(),
        })
    }
}
