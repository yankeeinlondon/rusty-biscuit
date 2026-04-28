//! Convenient re-exports for the most common `messenger` workflows.
//!
//! Import this module when you want the core message-building, dispatch,
//! receipt, and provider registration types in scope without a long import list.
//!
//! ## Examples
//!
//! ```rust
//! use messenger::prelude::*;
//!
//! let message = Message::markdown("**Deploy succeeded**")
//!     .metadata("service", "api");
//! let dispatch = Dispatch::to(Target::slack_channel("C01234567"));
//!
//! let _ = (message, dispatch);
//! ```
//!
//! ```rust
//! # #[cfg(feature = "slack")]
//! # {
//! use messenger::prelude::*;
//! use secrecy::SecretString;
//!
//! let provider = SlackProvider::new(SlackConfig {
//!     bot_token: SecretString::from("token".to_string()),
//!     api_base_url: None,
//! });
//!
//! let mut messenger = Messenger::new();
//! messenger.register(Box::new(provider));
//! # }
//! ```

pub use crate::attachment::{Attachment, AttachmentKind, AttachmentSource};
pub use crate::capabilities::CapabilitySet;
pub use crate::dispatch::{CompatibilityMode, DeliveryOptions, Dispatch, ProviderOverrides};
#[cfg(feature = "desktop")]
pub use crate::dispatch::{DesktopOverrides, NotificationIcon, NotificationUrgency};
pub use crate::error::MessengerError;
pub use crate::message::{Location, Message, MessageBody};
pub use crate::provider::{Messenger, Provider, SendPlan};
#[cfg(feature = "telegram")]
pub use crate::receipt::TelegramChatRef;
pub use crate::receipt::{Activation, DesktopPlatform, MessageRef, ProviderKind, SendReceipt};
#[cfg(feature = "signal")]
pub use crate::receipt::{SignalAuthor, SignalThreadKey};
#[cfg(feature = "apns")]
pub use crate::target::ApnsTarget;
#[cfg(feature = "desktop")]
pub use crate::target::DesktopTarget;
#[cfg(feature = "fcm")]
pub use crate::target::FcmTarget;
pub use crate::target::Target;
#[cfg(feature = "signal")]
pub use crate::target::{SignalAddress, SignalTarget};
#[cfg(feature = "telegram")]
pub use crate::target::{TelegramChatId, TelegramTarget};
pub use crate::validate::{
    CompatibilityWarning, normalize_dispatch, validate_dispatch, validate_message,
};

#[cfg(feature = "apns")]
pub use crate::provider::apns::{ApnsConfig, ApnsProvider};
#[cfg(feature = "desktop")]
pub use crate::provider::desktop::{
    DesktopConfig, DesktopNotificationProvider, LinuxDesktopConfig, MacOsDesktopConfig,
    MacOsNotificationStrategy, WindowsDesktopConfig,
};
#[cfg(feature = "discord")]
pub use crate::provider::discord::{DiscordConfig, DiscordProvider};
#[cfg(feature = "fcm")]
pub use crate::provider::fcm::{FcmConfig, FcmProvider};
#[cfg(feature = "signal")]
pub use crate::provider::signal::{SignalConfig, SignalProvider};
#[cfg(feature = "slack")]
pub use crate::provider::slack::{SlackConfig, SlackProvider};
#[cfg(feature = "telegram")]
pub use crate::provider::telegram::{TelegramConfig, TelegramProvider};
#[cfg(feature = "whatsapp")]
pub use crate::provider::whatsapp::{WhatsAppConfig, WhatsAppProvider};
