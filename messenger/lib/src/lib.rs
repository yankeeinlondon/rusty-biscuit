pub mod attachment;
pub mod capabilities;
pub mod dispatch;
pub mod error;
pub mod markdown;
pub mod message;
pub mod prelude;
#[doc(hidden)]
pub mod prepared;
pub mod provider;
pub mod receipt;
pub mod target;
pub mod validate;

pub use attachment::{Attachment, AttachmentKind, AttachmentSource};
pub use capabilities::CapabilitySet;
pub use dispatch::{CompatibilityMode, DeliveryOptions, Dispatch, ProviderOverrides};
#[cfg(feature = "desktop")]
pub use dispatch::{
    DesktopOverrides, NotificationAction, NotificationIcon, NotificationProgress,
    NotificationUrgency,
};
pub use error::MessengerError;
pub use message::{Location, Message, MessageBody};
#[doc(hidden)]
pub use prepared::PreparedMessage;
#[cfg(feature = "apns")]
pub use provider::apns::{ApnsConfig, ApnsProvider};
#[cfg(feature = "desktop")]
pub use provider::desktop::{
    DesktopConfig, DesktopNotificationProvider, DesktopNotificationReceipt,
    DesktopNotificationRequest, LinuxDesktopConfig, MacOsDesktopConfig, MacOsNotificationStrategy,
    WindowsDesktopConfig,
};
#[cfg(feature = "fcm")]
pub use provider::fcm::{FcmConfig, FcmProvider};
pub use provider::{Messenger, Provider, SendPlan};
pub use receipt::{DesktopPlatform, MessageRef, ProviderKind, SendReceipt};
#[cfg(feature = "apns")]
pub use target::ApnsTarget;
#[cfg(feature = "desktop")]
pub use target::DesktopTarget;
#[cfg(feature = "fcm")]
pub use target::FcmTarget;
pub use target::Target;
pub use validate::{
    CompatibilityWarning, normalize_dispatch, validate_dispatch, validate_message,
    validate_message_for_provider,
};

#[cfg(test)]
mod tests;
