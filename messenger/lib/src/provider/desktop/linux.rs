//! Linux desktop notification backend.
//!
//! Delivers notifications through the freedesktop.org D-Bus Notifications
//! interface via the [`notify-rust`](https://docs.rs/notify-rust) crate. The
//! backend uses the crate's async `zbus` path so the outer [`Provider`] send
//! is non-blocking on the tokio runtime.
//!
//! ## Notes
//!
//! - Returns the daemon-assigned notification ID as the `MessageRef::Desktop`
//!   `notification_id` for future replacement or dismissal support.
//! - Uses D-Bus hints (`urgency`, `category`, `desktop-entry`, `image-path`,
//!   `suppress-sound`) for best-effort enrichment. Hints are silently dropped
//!   by servers that do not support them.

use async_trait::async_trait;
use notify_rust::{Hint, Notification, Timeout, Urgency as NotifyUrgency};

use crate::dispatch::{NotificationIcon, NotificationUrgency};
use crate::error::MessengerError;
use crate::receipt::{DesktopPlatform, ProviderKind};

use super::LinuxDesktopConfig;
use super::backend::DesktopBackend;
use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// D-Bus notification backend for Linux / freedesktop.org desktops.
pub(crate) struct LinuxBackend {
    desktop_entry: Option<String>,
}

impl LinuxBackend {
    /// Build a backend with the supplied Linux-specific configuration.
    pub(crate) fn new(config: LinuxDesktopConfig) -> Self {
        Self {
            desktop_entry: config.desktop_entry,
        }
    }
}

#[async_trait]
impl DesktopBackend for LinuxBackend {
    fn platform(&self) -> DesktopPlatform {
        DesktopPlatform::Linux
    }

    async fn send(
        &self,
        request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        let mut notification = Notification::new();
        notification.summary(&request.title);
        notification.appname(&request.app_name);

        if let Some(body) = request.body.as_deref() {
            notification.body(body);
        }

        if let Some(icon) = request.icon.as_ref() {
            match icon {
                NotificationIcon::Named(name) => {
                    notification.icon(name);
                }
                NotificationIcon::Path(path) => {
                    notification.icon(&path.to_string_lossy());
                }
            }
        }

        notification.urgency(map_urgency(request.urgency));

        if let Some(category) = request.category.as_deref() {
            notification.hint(Hint::Category(category.to_string()));
        }

        if let Some(entry) = self.desktop_entry.as_deref() {
            notification.hint(Hint::DesktopEntry(entry.to_string()));
        }

        if let Some(image_path) = request.image.as_ref() {
            notification.hint(Hint::ImagePath(image_path.to_string_lossy().into_owned()));
        }

        if request.silent {
            notification.hint(Hint::SuppressSound(true));
        }

        if let Some(timeout_ms) = request.timeout_ms {
            notification.timeout(Timeout::Milliseconds(timeout_ms));
        }

        let handle = notification.show_async().await.map_err(|error| {
            ProviderKind::Desktop.transport_error(format!("D-Bus notification failed: {error}"))
        })?;

        Ok(DesktopNotificationReceipt::new(handle.id().to_string()))
    }
}

fn map_urgency(urgency: NotificationUrgency) -> NotifyUrgency {
    match urgency {
        NotificationUrgency::Low => NotifyUrgency::Low,
        NotificationUrgency::Normal => NotifyUrgency::Normal,
        NotificationUrgency::Critical => NotifyUrgency::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_mapping_covers_all_variants() {
        assert!(matches!(
            map_urgency(NotificationUrgency::Low),
            NotifyUrgency::Low
        ));
        assert!(matches!(
            map_urgency(NotificationUrgency::Normal),
            NotifyUrgency::Normal
        ));
        assert!(matches!(
            map_urgency(NotificationUrgency::Critical),
            NotifyUrgency::Critical
        ));
    }

    #[test]
    fn backend_reports_linux_platform() {
        let backend = LinuxBackend::new(LinuxDesktopConfig::default());
        assert_eq!(backend.platform(), DesktopPlatform::Linux);
    }
}
