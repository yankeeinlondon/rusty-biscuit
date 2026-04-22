//! Normalized internal request model consumed by platform [`DesktopBackend`]s.
//!
//! The public API exposes `Message` + `Dispatch` + `DesktopConfig` +
//! `DesktopOverrides`. Before a send can execute, those four inputs are
//! collapsed into a single [`DesktopNotificationRequest`] by the provider.
//! Platform backends operate on this one type, which keeps per-OS code
//! focused on the native API surface instead of reimplementing field
//! resolution and precedence rules.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::dispatch::{NotificationIcon, NotificationUrgency};
use crate::receipt::DesktopPlatform;

/// Normalized desktop notification request ready for backend delivery.
///
/// All fields have already been resolved against [`DesktopConfig`](super::DesktopConfig),
/// [`DesktopOverrides`](crate::dispatch::DesktopOverrides), and compatibility
/// normalization by the time a backend receives one. Backends should treat
/// unsupported fields as best-effort hints and drop them silently rather
/// than fail the send.
#[derive(Debug, Clone)]
pub struct DesktopNotificationRequest {
    /// Resolved title shown as the notification summary line.
    pub title: String,
    /// Optional body content rendered as plain text.
    pub body: Option<String>,
    /// Optional subtitle for backends that distinguish summary/subtitle/body.
    pub subtitle: Option<String>,
    /// The application name the notification center attributes this notification to.
    pub app_name: String,
    /// Optional icon reference for the notification.
    pub icon: Option<NotificationIcon>,
    /// First image attachment path, if any survived normalization.
    pub image: Option<PathBuf>,
    /// Suppress sound / haptic alert on delivery.
    pub silent: bool,
    /// Optional free-form category identifier (D-Bus category, macOS
    /// `categoryIdentifier`, Windows tag — backend-specific).
    pub category: Option<String>,
    /// Portable urgency / interruption level hint.
    pub urgency: NotificationUrgency,
    /// Optional expiry timeout in milliseconds.
    pub timeout_ms: Option<u32>,
    /// Optional identifier of a prior notification to replace.
    pub replace_id: Option<String>,
    /// Optional group identifier for backends that support notification
    /// grouping or threading.
    pub group_id: Option<String>,
}

/// Receipt returned by a [`DesktopBackend`] after a successful send.
///
/// The provider converts this into a [`SendReceipt`](crate::receipt::SendReceipt)
/// with a typed [`MessageRef::Desktop`](crate::receipt::MessageRef::Desktop).
#[derive(Debug, Clone, Default)]
pub struct DesktopNotificationReceipt {
    /// Platform-specific notification identifier (D-Bus ID, request
    /// identifier, tag, or generated UUID depending on the backend).
    pub notification_id: String,
    /// Extra free-form backend metadata (e.g. AppleScript fallback marker).
    pub metadata: BTreeMap<String, String>,
}

impl DesktopNotificationReceipt {
    /// Build a receipt with the given notification id and no metadata.
    pub fn new(notification_id: impl Into<String>) -> Self {
        Self {
            notification_id: notification_id.into(),
            metadata: BTreeMap::new(),
        }
    }

    /// Attach a metadata key/value pair to the receipt.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Platform label attached to a backend's request/receipt for diagnostics
/// and typed receipt wiring.
///
/// This is a thin re-export of [`DesktopPlatform`] kept in this module so
/// backend code can depend on `request::DesktopPlatform` without pulling in
/// the wider receipt surface.
pub type BackendPlatform = DesktopPlatform;
