#[cfg(feature = "desktop")]
use std::path::PathBuf;

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
#[allow(clippy::large_enum_variant)]
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
    #[cfg(feature = "desktop")]
    Desktop(DesktopOverrides),
}

#[cfg(feature = "discord")]
#[derive(Debug, Clone, Default)]
pub struct DiscordOverrides {}

#[cfg(feature = "discord")]
/// Placeholder for future Discord webhook-specific options.
///
/// This remains exhaustively constructible for now because adding
/// `#[non_exhaustive]` would be a breaking change for downstream callers that
/// currently instantiate it as `DiscordWebhookOverrides {}`.
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

/// Desktop-notification-specific overrides.
///
/// All fields are optional; callers supply only the values they want to
/// override from the route's [`DesktopConfig`](crate::provider::desktop::DesktopConfig)
/// defaults.
#[cfg(feature = "desktop")]
#[derive(Debug, Clone, Default)]
pub struct DesktopOverrides {
    pub subtitle: Option<String>,
    pub app_name: Option<String>,
    pub category: Option<String>,
    pub urgency: Option<NotificationUrgency>,
    pub timeout_ms: Option<u32>,
    pub icon: Option<NotificationIcon>,
    pub replace_id: Option<String>,
    pub group_id: Option<String>,
    /// Notification actions for backends that support interactive buttons.
    pub actions: Vec<NotificationAction>,
    /// Progress indicator for backends that support it.
    pub progress: Option<NotificationProgress>,
    /// Badge count for app icon badges on supported platforms.
    pub badge_count: Option<u32>,
}

/// An interactive action attached to a desktop notification.
///
/// Actions are presented as buttons or contextual menu items depending on the
/// host OS and notification center. Callback handling requires a packaged,
/// signed application with a running event loop; CLI sends will surface the
/// action UI but cannot receive callbacks.
#[cfg(feature = "desktop")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationAction {
    /// Machine-readable identifier returned when the action is triggered.
    pub id: String,
    /// Human-readable label shown in the notification UI.
    pub label: String,
}

/// Progress indicator for long-running operations delivered as a desktop
/// notification.
///
/// Backends that do not support progress UI will drop this silently.
#[cfg(feature = "desktop")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationProgress {
    /// Current progress value.
    pub current: u32,
    /// Maximum progress value.
    pub total: u32,
}

/// Portable urgency level for desktop notifications.
///
/// Backends map this onto their native notion of urgency, scenario, or
/// interruption level. Mapping is best-effort and may be dropped by
/// backends that do not expose a comparable concept.
#[cfg(feature = "desktop")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotificationUrgency {
    Low,
    #[default]
    Normal,
    Critical,
}

/// Portable icon reference for desktop notifications.
///
/// `Named` matches a freedesktop.org-style icon name (e.g. `dialog-information`);
/// `Path` points to an absolute filesystem path that the backend should read
/// at send time.
#[cfg(feature = "desktop")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationIcon {
    Named(String),
    Path(PathBuf),
}

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
