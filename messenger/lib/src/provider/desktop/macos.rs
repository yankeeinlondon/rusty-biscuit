//! macOS desktop notification backend.
//!
//! Two delivery strategies are wired behind a single backend:
//!
//! - **AppleScript** (`osascript display notification`) — the v1 default. Works
//!   from any process (including `cargo install`ed CLI binaries) and does not
//!   trigger a notification authorization prompt. Limited feature surface.
//! - **Native `UserNotifications.framework`** via [`objc2-user-notifications`]
//!   — opt-in path for bundled/signed apps that want access to richer
//!   notification features (subtitles, body, categories, interruption level).
//!   Requires bundle identity; the system silently drops the notification
//!   otherwise.
//!
//! Backend selection happens per-request from [`MacOsDesktopConfig::strategy`]:
//!
//! | Strategy | Delivery |
//! |----------|----------|
//! | `Auto` (default) | AppleScript |
//! | `AppleScript` | AppleScript |
//! | `NativeUserNotifications` | Native |
//!
//! Both paths generate a UUID for the `notification_id` (AppleScript does not
//! return a handle; native could, but the crate's submit path is fire-and-forget
//! without a completion handler). Receipt metadata distinguishes the delivery
//! path with `delivery=applescript` or `delivery=native`.

use async_trait::async_trait;
use std::process::Command;

use crate::error::MessengerError;
use crate::receipt::{DesktopPlatform, ProviderKind};

use super::MacOsNotificationStrategy;
use super::backend::DesktopBackend;
use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// macOS notification backend.
pub(crate) struct MacOsBackend {
    strategy: MacOsNotificationStrategy,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    bundle_id: Option<String>,
}

impl MacOsBackend {
    /// Build a backend with the supplied macOS-specific configuration.
    pub(crate) fn new(strategy: MacOsNotificationStrategy, bundle_id: Option<String>) -> Self {
        Self {
            strategy,
            bundle_id,
        }
    }

    /// Resolve `Auto` into a concrete strategy.
    ///
    /// Phase 1 pins `Auto` to AppleScript unconditionally (see spec rationale);
    /// promoting to native delivery requires bundled app identity and a
    /// persistent authorization story.
    fn resolved_strategy(&self) -> MacOsNotificationStrategy {
        match self.strategy {
            MacOsNotificationStrategy::Auto => MacOsNotificationStrategy::AppleScript,
            other => other,
        }
    }
}

#[async_trait]
impl DesktopBackend for MacOsBackend {
    fn platform(&self) -> DesktopPlatform {
        DesktopPlatform::MacOS
    }

    async fn send(
        &self,
        request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        match self.resolved_strategy() {
            MacOsNotificationStrategy::AppleScript | MacOsNotificationStrategy::Auto => {
                send_applescript(&request)
            }
            MacOsNotificationStrategy::NativeUserNotifications => {
                send_native(&request, self.bundle_id.as_deref())
            }
        }
    }

    async fn replace(
        &self,
        id: &str,
        request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        match self.resolved_strategy() {
            MacOsNotificationStrategy::AppleScript | MacOsNotificationStrategy::Auto => {
                Err(MessengerError::UnsupportedFeature {
                    provider: ProviderKind::Desktop,
                    feature: "notification replacement",
                })
            }
            MacOsNotificationStrategy::NativeUserNotifications => {
                send_native_with_id(&request, self.bundle_id.as_deref(), id)
            }
        }
    }

    async fn dismiss(&self, id: &str) -> Result<(), MessengerError> {
        match self.resolved_strategy() {
            MacOsNotificationStrategy::AppleScript | MacOsNotificationStrategy::Auto => {
                Err(MessengerError::UnsupportedFeature {
                    provider: ProviderKind::Desktop,
                    feature: "notification dismissal",
                })
            }
            MacOsNotificationStrategy::NativeUserNotifications => {
                dismiss_native(id)
            }
        }
    }
}

fn send_applescript(
    request: &DesktopNotificationRequest,
) -> Result<DesktopNotificationReceipt, MessengerError> {
    let script = applescript_command(request);
    let notification_id = uuid::Uuid::new_v4().to_string();

    let status = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .status()
        .map_err(|error| {
            ProviderKind::Desktop.transport_error(format!("failed to invoke osascript: {error}"))
        })?;

    if !status.success() {
        return Err(ProviderKind::Desktop.transport_error(format!(
            "osascript exited with status {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "<unknown>".into())
        )));
    }

    Ok(DesktopNotificationReceipt::new(notification_id).with_metadata("delivery", "applescript"))
}

/// Build an AppleScript `display notification` command for the given request.
///
/// Exposed for unit tests so we can verify escaping of embedded quotes and
/// backslashes. The CLI never echoes the raw script string to users.
fn applescript_command(request: &DesktopNotificationRequest) -> String {
    let body = request.body.as_deref().unwrap_or("");
    let title = &request.title;
    let subtitle = request.subtitle.as_deref();

    let mut script = format!(
        "display notification \"{}\" with title \"{}\"",
        escape_applescript(body),
        escape_applescript(title)
    );
    if let Some(subtitle) = subtitle {
        script.push_str(&format!(" subtitle \"{}\"", escape_applescript(subtitle)));
    }
    script
}

fn escape_applescript(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn send_native(
    request: &DesktopNotificationRequest,
    bundle_id: Option<&str>,
) -> Result<DesktopNotificationReceipt, MessengerError> {
    let id = request
        .replace_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    send_native_with_id(request, bundle_id, &id)
}

#[cfg(target_os = "macos")]
fn send_native_with_id(
    request: &DesktopNotificationRequest,
    _bundle_id: Option<&str>,
    id: &str,
) -> Result<DesktopNotificationReceipt, MessengerError> {
    use objc2_foundation::NSString;
    use objc2_user_notifications::{
        UNMutableNotificationContent, UNNotificationRequest, UNUserNotificationCenter,
    };

    // The native path is intentionally fire-and-forget: we submit the
    // request to the system center without a completion handler. This keeps
    // the code free of block2 glue while still exercising the native API. A
    // future phase that ships a signed, bundled app can wire completion
    // callbacks for authoritative delivery status.
    let center = UNUserNotificationCenter::currentNotificationCenter();

    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(&request.title));
    if let Some(subtitle) = request.subtitle.as_deref() {
        content.setSubtitle(&NSString::from_str(subtitle));
    }
    if let Some(body) = request.body.as_deref() {
        content.setBody(&NSString::from_str(body));
    }
    if let Some(category) = request.category.as_deref() {
        content.setCategoryIdentifier(&NSString::from_str(category));
    }

    let identifier = NSString::from_str(id);
    let notification_request =
        UNNotificationRequest::requestWithIdentifier_content_trigger(&identifier, &content, None);

    center.addNotificationRequest_withCompletionHandler(&notification_request, None);

    Ok(DesktopNotificationReceipt::new(id.to_string())
        .with_metadata("delivery", "native")
        .with_metadata("delivery_confirmed", "false"))
}

#[cfg(target_os = "macos")]
fn dismiss_native(id: &str) -> Result<(), MessengerError> {
    use objc2_foundation::{NSArray, NSString};
    use objc2_user_notifications::UNUserNotificationCenter;

    let center = UNUserNotificationCenter::currentNotificationCenter();
    let identifier = NSString::from_str(id);
    let identifiers = NSArray::from_retained_slice(&[identifier]);
    center.removeDeliveredNotificationsWithIdentifiers(&identifiers);
    center.removePendingNotificationRequestsWithIdentifiers(&identifiers);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn send_native(
    _request: &DesktopNotificationRequest,
    _bundle_id: Option<&str>,
) -> Result<DesktopNotificationReceipt, MessengerError> {
    Err(MessengerError::Provider {
        provider: ProviderKind::Desktop,
        code: None,
        message: "native macOS notification delivery is only available on macOS hosts".into(),
    })
}

#[cfg(not(target_os = "macos"))]
fn send_native_with_id(
    _request: &DesktopNotificationRequest,
    _bundle_id: Option<&str>,
    _id: &str,
) -> Result<DesktopNotificationReceipt, MessengerError> {
    Err(MessengerError::Provider {
        provider: ProviderKind::Desktop,
        code: None,
        message: "native macOS notification delivery is only available on macOS hosts".into(),
    })
}

#[cfg(not(target_os = "macos"))]
fn dismiss_native(_id: &str) -> Result<(), MessengerError> {
    Err(MessengerError::Provider {
        provider: ProviderKind::Desktop,
        code: None,
        message: "native macOS notification dismissal is only available on macOS hosts".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::NotificationUrgency;

    fn request(title: &str, body: Option<&str>) -> DesktopNotificationRequest {
        DesktopNotificationRequest {
            title: title.into(),
            body: body.map(str::to_string),
            subtitle: None,
            app_name: "Messenger".into(),
            icon: None,
            image: None,
            silent: false,
            category: None,
            urgency: NotificationUrgency::Normal,
            timeout_ms: None,
            replace_id: None,
            group_id: None,
        }
    }

    #[test]
    fn applescript_escapes_double_quotes_and_backslashes() {
        let req = request("Title \"with quotes\"", Some("body with \\ backslash"));
        let script = applescript_command(&req);
        assert!(script.contains("Title \\\"with quotes\\\""));
        assert!(script.contains("body with \\\\ backslash"));
    }

    #[test]
    fn applescript_includes_subtitle_when_present() {
        let mut req = request("t", Some("b"));
        req.subtitle = Some("sub".into());
        let script = applescript_command(&req);
        assert!(script.contains("subtitle \"sub\""));
    }

    #[test]
    fn applescript_omits_subtitle_when_missing() {
        let req = request("t", Some("b"));
        let script = applescript_command(&req);
        assert!(!script.contains("subtitle"));
    }

    #[test]
    fn resolved_strategy_maps_auto_to_applescript() {
        let backend = MacOsBackend::new(MacOsNotificationStrategy::Auto, None);
        assert_eq!(
            backend.resolved_strategy(),
            MacOsNotificationStrategy::AppleScript
        );
    }

    #[test]
    fn resolved_strategy_preserves_explicit_choices() {
        let backend = MacOsBackend::new(MacOsNotificationStrategy::AppleScript, None);
        assert_eq!(
            backend.resolved_strategy(),
            MacOsNotificationStrategy::AppleScript
        );
        let backend = MacOsBackend::new(MacOsNotificationStrategy::NativeUserNotifications, None);
        assert_eq!(
            backend.resolved_strategy(),
            MacOsNotificationStrategy::NativeUserNotifications
        );
    }

    #[test]
    fn backend_reports_macos_platform() {
        let backend = MacOsBackend::new(MacOsNotificationStrategy::Auto, None);
        assert_eq!(backend.platform(), DesktopPlatform::MacOS);
    }
}
