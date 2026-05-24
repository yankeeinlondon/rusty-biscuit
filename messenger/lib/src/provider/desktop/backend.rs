//! Internal [`DesktopBackend`] trait shared by platform implementations.
//!
//! Backends convert a [`DesktopNotificationRequest`] into the platform
//! delivery flow. Each implementation elects installed helpers with
//! `score() > 0` as the primary path, then falls back to the native floor
//! (D-Bus on Linux, `UserNotifications.framework` or AppleScript on macOS,
//! WinRT toasts on Windows). Keeping the trait out of the public surface lets
//! platform crates evolve independently of the `Provider` API.

use async_trait::async_trait;

use crate::error::MessengerError;
use crate::receipt::{DesktopPlatform, ProviderKind};

use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// Internal abstraction over a platform-specific notification backend.
///
/// Implementations own both helper election and native fallback for their
/// platform.
///
/// Implementations must be `Send + Sync` so the outer
/// [`DesktopNotificationProvider`](super::DesktopNotificationProvider)
/// can be shared across tasks.
#[async_trait]
pub(crate) trait DesktopBackend: Send + Sync {
    /// The platform this backend targets. Used to tag receipts.
    fn platform(&self) -> DesktopPlatform;

    /// Deliver the notification request to the host OS.
    async fn send(
        &self,
        request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError>;

    /// Replace an existing notification using its platform identifier.
    ///
    /// Default implementation returns [`MessengerError::UnsupportedFeature`].
    /// Platforms that support replacement (Linux D-Bus, macOS native)
    /// override this method.
    async fn replace(
        &self,
        _id: &str,
        _request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        Err(MessengerError::UnsupportedFeature {
            provider: ProviderKind::Desktop,
            feature: "notification replacement",
        })
    }

    /// Dismiss a delivered notification using its platform identifier.
    ///
    /// Default implementation returns [`MessengerError::UnsupportedFeature`].
    /// Platforms that support dismissal (macOS native) override this method.
    async fn dismiss(&self, _id: &str) -> Result<(), MessengerError> {
        Err(MessengerError::UnsupportedFeature {
            provider: ProviderKind::Desktop,
            feature: "notification dismissal",
        })
    }
}
