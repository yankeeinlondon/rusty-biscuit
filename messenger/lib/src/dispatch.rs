use crate::receipt::MessageRef;
use crate::target::Target;

/// A destination-bound send request wrapping a target, reply context, and options.
#[derive(Debug, Clone)]
pub struct Dispatch {
    pub target: Target,
    pub reply_to: Option<MessageRef>,
    pub options: DeliveryOptions,
    pub overrides: ProviderOverrides,
}

/// Options that affect delivery behavior.
#[derive(Debug, Clone)]
pub struct DeliveryOptions {
    pub silent: bool,
    pub disable_link_preview: bool,
    pub compatibility: CompatibilityMode,
}

impl Default for DeliveryOptions {
    fn default() -> Self {
        Self {
            silent: false,
            disable_link_preview: false,
            compatibility: CompatibilityMode::BestEffort,
        }
    }
}

/// How to handle features unsupported by the target provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityMode {
    BestEffort,
    Strict,
}

/// Provider-specific overrides that escape the shared API.
#[derive(Debug, Clone)]
pub enum ProviderOverrides {
    None,
    #[cfg(feature = "discord")]
    Discord(DiscordOverrides),
    #[cfg(feature = "discord")]
    DiscordWebhook(DiscordWebhookOverrides),
    #[cfg(feature = "slack")]
    Slack(SlackOverrides),
    #[cfg(feature = "signal")]
    Signal(SignalOverrides),
    #[cfg(feature = "whatsapp")]
    WhatsApp(WhatsAppOverrides),
    #[cfg(feature = "telegram")]
    Telegram(TelegramOverrides),
}

#[cfg(feature = "discord")]
#[derive(Debug, Clone, Default)]
pub struct DiscordOverrides {}

#[cfg(feature = "discord")]
#[derive(Debug, Clone, Default)]
pub struct DiscordWebhookOverrides {}

#[cfg(feature = "slack")]
#[derive(Debug, Clone, Default)]
pub struct SlackOverrides {}

#[cfg(feature = "signal")]
#[derive(Debug, Clone, Default)]
pub struct SignalOverrides {}

#[cfg(feature = "whatsapp")]
#[derive(Debug, Clone, Default)]
pub struct WhatsAppOverrides {}

#[cfg(feature = "telegram")]
#[derive(Debug, Clone, Default)]
pub struct TelegramOverrides {}

impl Dispatch {
    /// Create a dispatch targeting the given destination.
    pub fn to(target: Target) -> Self {
        Self {
            target,
            reply_to: None,
            options: DeliveryOptions::default(),
            overrides: ProviderOverrides::None,
        }
    }

    /// Set the reply-to reference.
    pub fn reply_to(mut self, message_ref: MessageRef) -> Self {
        self.reply_to = Some(message_ref);
        self
    }

    /// Enable silent delivery (no notification sound).
    pub fn silent(mut self) -> Self {
        self.options.silent = true;
        self
    }

    /// Disable link preview generation.
    pub fn disable_link_preview(mut self) -> Self {
        self.options.disable_link_preview = true;
        self
    }

    /// Use strict compatibility mode (error on unsupported features).
    pub fn strict(mut self) -> Self {
        self.options.compatibility = CompatibilityMode::Strict;
        self
    }

    /// Attach provider-specific overrides.
    pub fn with_overrides(mut self, overrides: ProviderOverrides) -> Self {
        self.overrides = overrides;
        self
    }
}
