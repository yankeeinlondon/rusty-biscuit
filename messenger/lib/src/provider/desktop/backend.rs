//! Internal [`DesktopBackend`] trait shared by platform implementations.
//!
//! Backends convert a [`DesktopNotificationRequest`] into a native call
//! (D-Bus on Linux, `UserNotifications.framework` or AppleScript on macOS,
//! WinRT toasts on Windows). Keeping the trait out of the public surface
//! lets platform crates evolve independently of the `Provider` API.

use async_trait::async_trait;

use crate::error::MessengerError;
use crate::receipt::DesktopPlatform;

use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// Internal abstraction over a platform-specific notification backend.
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
}
