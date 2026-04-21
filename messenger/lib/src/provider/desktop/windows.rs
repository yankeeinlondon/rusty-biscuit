//! Windows desktop notification backend.
//!
//! Uses [`winrt-notification`](https://docs.rs/winrt-notification) to emit
//! WinRT toast notifications. Unpackaged Win32 apps need a registered
//! [App User Model ID] and a Start Menu shortcut pointing at the CLI
//! executable for toasts to render. The acceptance matrix requires that the
//! send path refuse to run unless those prerequisites are in place — and that
//! `send` never writes to the host filesystem outside `~/.messenger/`.
//!
//! Current bootstrap gate:
//!
//! - [`WindowsDesktopConfig::app_id`] must be `Some`.
//! - [`shortcut_bootstrap_state`] checks for a matching `<app_id>.lnk` shortcut
//!   under `%APPDATA%\Microsoft\Windows\Start Menu\Programs\`.
//!
//! When either check fails, the backend returns
//! [`MessengerError::MissingConfiguration`] with a stable `field` string that
//! points to `messenger setup desktop`. The actual shortcut/AUMID registration
//! is Phase 6 territory; Phase 4 only enforces the contract.
//!
//! [App User Model ID]: https://learn.microsoft.com/en-us/windows/win32/shell/appids

use async_trait::async_trait;

use crate::error::MessengerError;
use crate::receipt::{DesktopPlatform, ProviderKind};

use super::WindowsDesktopConfig;
use super::backend::DesktopBackend;
use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// `field` tag used on [`MessengerError::MissingConfiguration`] errors returned
/// from the Windows backend when bootstrapping is incomplete.
///
/// Kept as a module constant so tests and CLI help text can reference the
/// exact string without drifting.
pub(crate) const WINDOWS_SETUP_REQUIRED: &str =
    "Windows desktop notifications require `messenger setup desktop` to register the Start Menu shortcut and App User Model ID";

/// Windows WinRT toast backend.
pub(crate) struct WindowsBackend {
    config: WindowsDesktopConfig,
}

impl WindowsBackend {
    /// Build a backend with the supplied Windows-specific configuration.
    pub(crate) fn new(config: WindowsDesktopConfig) -> Self {
        Self { config }
    }

    /// Verify that the host is bootstrapped (AUMID configured, shortcut in place).
    ///
    /// Returns `Err(MessengerError::MissingConfiguration)` if any required piece
    /// is missing; callers should short-circuit before attempting a toast.
    fn check_bootstrap(&self) -> Result<&str, MessengerError> {
        let app_id = self
            .config
            .app_id
            .as_deref()
            .ok_or(MessengerError::MissingConfiguration {
                provider: ProviderKind::Desktop,
                field: WINDOWS_SETUP_REQUIRED,
            })?;

        match shortcut_bootstrap_state(app_id) {
            ShortcutBootstrapState::Present | ShortcutBootstrapState::Unknown => Ok(app_id),
            ShortcutBootstrapState::Missing => Err(MessengerError::MissingConfiguration {
                provider: ProviderKind::Desktop,
                field: WINDOWS_SETUP_REQUIRED,
            }),
        }
    }
}

#[async_trait]
impl DesktopBackend for WindowsBackend {
    fn platform(&self) -> DesktopPlatform {
        DesktopPlatform::Windows
    }

    async fn send(
        &self,
        request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        let app_id = self.check_bootstrap()?;
        send_toast(app_id, &request)
    }
}

/// Outcome of the Start Menu shortcut bootstrap check.
///
/// `Unknown` is reserved for platforms where we cannot cheaply inspect the
/// Start Menu (e.g. non-Windows test runs). In that case the caller should
/// accept the configured AUMID and defer to whatever the WinRT subsystem
/// returns at send time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutBootstrapState {
    Present,
    Missing,
    Unknown,
}

/// Check whether the Start Menu shortcut `<app_id>.lnk` exists.
///
/// On Windows we consult `%APPDATA%\Microsoft\Windows\Start Menu\Programs\`.
/// On other platforms we return [`ShortcutBootstrapState::Unknown`] — the
/// backend will not be instantiated at runtime on non-Windows hosts, but keeping
/// the function cross-platform simplifies testing and conditional compilation.
#[cfg(target_os = "windows")]
pub(crate) fn shortcut_bootstrap_state(app_id: &str) -> ShortcutBootstrapState {
    let Ok(appdata) = std::env::var("APPDATA") else {
        return ShortcutBootstrapState::Unknown;
    };

    let mut path = std::path::PathBuf::from(appdata);
    path.push("Microsoft");
    path.push("Windows");
    path.push("Start Menu");
    path.push("Programs");
    path.push(format!("{app_id}.lnk"));

    if path.is_file() {
        ShortcutBootstrapState::Present
    } else {
        ShortcutBootstrapState::Missing
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn shortcut_bootstrap_state(_app_id: &str) -> ShortcutBootstrapState {
    ShortcutBootstrapState::Unknown
}

#[cfg(target_os = "windows")]
fn send_toast(
    app_id: &str,
    request: &DesktopNotificationRequest,
) -> Result<DesktopNotificationReceipt, MessengerError> {
    use winrt_notification::{Sound, Toast};

    let mut toast = Toast::new(app_id).title(&request.title);

    if let Some(body) = request.body.as_deref() {
        let mut lines = body.split('\n');
        if let Some(first) = lines.next() {
            toast = toast.text1(first);
        }
        let rest: Vec<&str> = lines.collect();
        if !rest.is_empty() {
            toast = toast.text2(&rest.join("\n"));
        }
    }

    if let Some(image_path) = request.image.as_ref() {
        toast = toast.image(image_path, "");
    }

    if request.silent {
        toast = toast.sound(None);
    } else {
        toast = toast.sound(Some(Sound::Default));
    }

    toast
        .show()
        .map_err(|error| ProviderKind::Desktop.transport_error(format!("WinRT toast failed: {error}")))?;

    let notification_id = uuid::Uuid::new_v4().to_string();
    Ok(DesktopNotificationReceipt::new(notification_id).with_metadata("delivery", "winrt"))
}

#[cfg(not(target_os = "windows"))]
fn send_toast(
    _app_id: &str,
    _request: &DesktopNotificationRequest,
) -> Result<DesktopNotificationReceipt, MessengerError> {
    Err(MessengerError::Provider {
        provider: ProviderKind::Desktop,
        code: None,
        message: "Windows toast delivery is only available on Windows hosts".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn send_returns_missing_configuration_without_app_id() {
        let backend = WindowsBackend::new(WindowsDesktopConfig { app_id: None });
        let request = DesktopNotificationRequest {
            title: "Hi".into(),
            body: None,
            subtitle: None,
            app_name: "Messenger".into(),
            icon: None,
            image: None,
            silent: false,
            category: None,
            urgency: crate::dispatch::NotificationUrgency::Normal,
            timeout_ms: None,
            replace_id: None,
        };

        let error = backend.send(request).await.unwrap_err();
        assert!(matches!(
            error,
            MessengerError::MissingConfiguration {
                provider: ProviderKind::Desktop,
                field: WINDOWS_SETUP_REQUIRED,
            }
        ));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn non_windows_host_reports_unknown_bootstrap_state() {
        assert_eq!(
            shortcut_bootstrap_state("RustyBiscuit.Messenger"),
            ShortcutBootstrapState::Unknown
        );
    }

    #[test]
    fn backend_reports_windows_platform() {
        let backend = WindowsBackend::new(WindowsDesktopConfig::default());
        assert_eq!(backend.platform(), DesktopPlatform::Windows);
    }

    #[test]
    fn bootstrap_check_rejects_missing_app_id() {
        let backend = WindowsBackend::new(WindowsDesktopConfig { app_id: None });
        let error = backend.check_bootstrap().unwrap_err();
        assert!(matches!(
            error,
            MessengerError::MissingConfiguration {
                provider: ProviderKind::Desktop,
                field: WINDOWS_SETUP_REQUIRED,
            }
        ));
    }
}
