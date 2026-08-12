//! Windows desktop notification backend.
//!
//! Three delivery strategies are wired behind a single backend:
//!
//! - **Helpers** (`snoretoast`, `burnttoast`) — primary delivery path when
//!   installed and scored above zero for the request. Helpers are probed via
//!   `sniff` at construction time, ship interactive action buttons and inline
//!   replies (snoretoast) that the bare WinRT toast path does not surface, and
//!   tolerate hosts that lack a packaged app identity.
//! - **`winrt-notification`** native toast — universal floor when no helper
//!   is installed. Requires a registered App User Model ID and a Start Menu
//!   shortcut (`messenger setup desktop`); unbundled apps will otherwise fail
//!   to render the toast.
//!
//! Helpers with `score() > 0` are tried first. The native WinRT path remains
//! the fallback/floor when no helper can serve the dispatch. Receipt metadata
//! records which path served the notification: `helper_used` for helpers,
//! `delivery=winrt` for the native toast.
//!
//! Current bootstrap gate (only enforced when no helper succeeds):
//!
//! - [`WindowsDesktopConfig::app_id`] must be `Some`.
//! - [`shortcut_bootstrap_state`] checks for a matching `<app_id>.lnk` shortcut
//!   under `%APPDATA%\Microsoft\Windows\Start Menu\Programs\`.
//!
//! When either check fails and no helper handled the send, the backend
//! returns [`MessengerError::MissingConfiguration`] with a stable `field`
//! string that points to `messenger setup desktop`.
//!
//! [App User Model ID]: https://learn.microsoft.com/en-us/windows/win32/shell/appids

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::MessengerError;
use crate::receipt::{DesktopPlatform, ProviderKind};

use super::WindowsDesktopConfig;
use super::backend::DesktopBackend;
use super::helpers::{HelperAttempt, HelperBackend, HelperError, HelperName, elect_helpers};
use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// `field` tag used on [`MessengerError::MissingConfiguration`] errors returned
/// from the Windows backend when bootstrapping is incomplete.
///
/// Kept as a module constant so tests and CLI help text can reference the
/// exact string without drifting.
pub(crate) const WINDOWS_SETUP_REQUIRED: &str = "Windows desktop notifications require `messenger setup desktop` to register the Start Menu shortcut and App User Model ID";

/// Windows toast backend with helper election and native WinRT fallback.
pub(crate) struct WindowsBackend {
    config: WindowsDesktopConfig,
    helpers: Vec<Arc<dyn HelperBackend>>,
}

impl WindowsBackend {
    /// Build a backend with the supplied Windows-specific configuration.
    ///
    /// Detects available notification helpers via `sniff` and constructs
    /// the corresponding [`HelperBackend`] adapters once.
    pub(crate) fn new(config: WindowsDesktopConfig) -> Self {
        let helpers = match config.app_id.as_deref() {
            Some(app_id) => detect_windows_helpers(app_id),
            None => Vec::new(),
        };
        Self::with_helpers(config, helpers)
    }

    /// Test seam: build a backend with explicitly provided helpers.
    ///
    /// Skips sniff detection so unit tests can wire fake helpers without
    /// touching the host filesystem.
    pub(crate) fn with_helpers(
        config: WindowsDesktopConfig,
        helpers: Vec<Arc<dyn HelperBackend>>,
    ) -> Self {
        Self { config, helpers }
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

        Self::evaluate_bootstrap(app_id, shortcut_bootstrap_state(app_id))
    }

    /// Decide whether the supplied bootstrap state allows a native send.
    ///
    /// Split out from [`Self::check_bootstrap`] so tests can inject the
    /// outcome of the shortcut probe without touching the host filesystem.
    fn evaluate_bootstrap(
        app_id: &str,
        state: ShortcutBootstrapState,
    ) -> Result<&str, MessengerError> {
        match state {
            ShortcutBootstrapState::Present | ShortcutBootstrapState::Unknown => Ok(app_id),
            ShortcutBootstrapState::Missing => Err(MessengerError::MissingConfiguration {
                provider: ProviderKind::Desktop,
                field: WINDOWS_SETUP_REQUIRED,
            }),
        }
    }

    fn native_send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        let app_id = self.check_bootstrap()?;
        send_toast(app_id, request)
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
        let mut attempts: Vec<HelperAttempt> = Vec::new();
        let elected = elect_helpers(&self.helpers, &request, &self.config.prefer_helpers);

        for helper in elected {
            match helper.send(&request).await {
                Ok(mut receipt) => {
                    annotate_receipt_helper(&mut receipt, helper.name(), &attempts);
                    return Ok(receipt);
                }
                Err(error) => {
                    tracing::warn!(
                        helper = %helper.name(),
                        %error,
                        "windows helper send failed; trying next attempt",
                    );
                    if !error.is_fallback_eligible() {
                        return Err(helper_error_to_messenger(helper.name(), error));
                    }
                    attempts.push(HelperAttempt::from_error(helper.name(), &error));
                }
            }
        }

        let mut receipt = self.native_send(&request)?;
        annotate_native_receipt(&mut receipt, &attempts);
        Ok(receipt)
    }

    async fn replace(
        &self,
        id: &str,
        request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        if let Some(hint) = request.replace_helper_hint
            && (hint == HelperName::SnoreToast || hint == HelperName::BurntToast)
            && let Some(helper) = self.helpers.iter().find(|helper| helper.name() == hint)
        {
            match helper.replace(id, &request).await {
                Ok(mut receipt) => {
                    receipt
                        .metadata
                        .entry("helper_used".to_string())
                        .or_insert_with(|| helper.name().to_string());
                    return Ok(receipt);
                }
                Err(error) => {
                    if !error.is_fallback_eligible() {
                        return Err(helper_error_to_messenger(helper.name(), error));
                    }
                    tracing::warn!(
                        helper = %helper.name(),
                        %error,
                        "windows helper replace failed; falling back to native",
                    );
                }
            }
        }

        Err(MessengerError::UnsupportedFeature {
            provider: ProviderKind::Desktop,
            feature: "notification replacement",
        })
    }

    async fn dismiss(&self, _id: &str) -> Result<(), MessengerError> {
        Err(MessengerError::UnsupportedFeature {
            provider: ProviderKind::Desktop,
            feature: "notification dismissal",
        })
    }
}

fn annotate_receipt_helper(
    receipt: &mut DesktopNotificationReceipt,
    helper: HelperName,
    attempts: &[HelperAttempt],
) {
    receipt
        .metadata
        .insert("helper_used".to_string(), helper.to_string());
    if !attempts.is_empty() {
        receipt
            .metadata
            .insert("helper_fallbacks".to_string(), summarize(attempts));
    }
}

fn annotate_native_receipt(receipt: &mut DesktopNotificationReceipt, attempts: &[HelperAttempt]) {
    receipt
        .metadata
        .insert("helper_used".to_string(), "native".to_string());
    if !attempts.is_empty() {
        receipt
            .metadata
            .insert("helper_fallbacks".to_string(), summarize(attempts));
    }
}

fn summarize(attempts: &[HelperAttempt]) -> String {
    attempts
        .iter()
        .map(|attempt| attempt.summary())
        .collect::<Vec<_>>()
        .join(",")
}

fn helper_error_to_messenger(name: HelperName, error: HelperError) -> MessengerError {
    match error {
        HelperError::Unsupported(feature) => MessengerError::UnsupportedFeature {
            provider: ProviderKind::Desktop,
            feature,
        },
        other => MessengerError::Provider {
            provider: ProviderKind::Desktop,
            code: Some(other.summary_tag().to_string()),
            message: format!("{name}: {other}"),
        },
    }
}

/// Probe sniff for Windows notification helpers and turn them into
/// [`HelperBackend`] adapters. The helpers reuse the configured AppID; the
/// caller is responsible for ensuring `app_id` is set before invoking this
/// function (an unset AppID skips helper construction so the backend can
/// surface `MissingConfiguration` from the native path instead).
#[cfg(target_os = "windows")]
fn detect_windows_helpers(app_id: &str) -> Vec<Arc<dyn HelperBackend>> {
    use super::helpers::burnttoast::BurntToastHelper;
    use super::helpers::snoretoast::SnoreToastHelper;

    let info = sniff::programs::InstalledNotificationHelpers::new();
    let mut helpers: Vec<Arc<dyn HelperBackend>> = Vec::new();

    if let Some(path) = info.path(sniff::programs::NotificationHelper::SnoreToast) {
        helpers.push(Arc::new(SnoreToastHelper::new(path, app_id.to_string())));
    }

    if info.is_installed(sniff::programs::NotificationHelper::BurntToast)
        && let Some(pwsh) = sniff::programs::find_program::find_program("pwsh")
            .or_else(|| sniff::programs::find_program::find_program("powershell"))
    {
        helpers.push(Arc::new(BurntToastHelper::new(pwsh, app_id.to_string())));
    }

    helpers
}

#[cfg(not(target_os = "windows"))]
fn detect_windows_helpers(_app_id: &str) -> Vec<Arc<dyn HelperBackend>> {
    Vec::new()
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

    toast.show().map_err(|error| {
        ProviderKind::Desktop.transport_error(format!("WinRT toast failed: {error}"))
    })?;

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
    use crate::dispatch::{NotificationAction, NotificationUrgency};
    use crate::provider::desktop::helpers::HelperCapabilities;
    use std::sync::Mutex;

    struct FakeHelper {
        helper_name: HelperName,
        score_for: u8,
        outcome: Mutex<Vec<Result<DesktopNotificationReceipt, HelperError>>>,
    }

    impl FakeHelper {
        #[allow(clippy::new_ret_no_self)]
        fn new(
            name: HelperName,
            score: u8,
            outcomes: Vec<Result<DesktopNotificationReceipt, HelperError>>,
        ) -> Arc<dyn HelperBackend> {
            Arc::new(Self {
                helper_name: name,
                score_for: score,
                outcome: Mutex::new(outcomes),
            })
        }

        fn next_outcome(&self) -> Result<DesktopNotificationReceipt, HelperError> {
            let mut queue = self.outcome.lock().unwrap();
            if queue.is_empty() {
                Err(HelperError::NotPresent)
            } else {
                queue.remove(0)
            }
        }
    }

    #[async_trait]
    impl HelperBackend for FakeHelper {
        fn name(&self) -> HelperName {
            self.helper_name
        }

        fn capabilities(&self) -> HelperCapabilities {
            HelperCapabilities {
                actions: true,
                reply: true,
                image: true,
                sound: true,
                replace: true,
                group: false,
                blocking: true,
            }
        }

        fn score(&self, _request: &DesktopNotificationRequest) -> u8 {
            self.score_for
        }

        async fn send(
            &self,
            _request: &DesktopNotificationRequest,
        ) -> Result<DesktopNotificationReceipt, HelperError> {
            self.next_outcome()
        }

        async fn replace(
            &self,
            _id: &str,
            _request: &DesktopNotificationRequest,
        ) -> Result<DesktopNotificationReceipt, HelperError> {
            self.next_outcome()
        }
    }

    fn config_with_app_id() -> WindowsDesktopConfig {
        WindowsDesktopConfig {
            app_id: Some("RustyBiscuit.Messenger".into()),
            prefer_helpers: Vec::new(),
        }
    }

    fn request() -> DesktopNotificationRequest {
        DesktopNotificationRequest {
            title: "Hi".into(),
            body: None,
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
            actions: Vec::new(),
            progress: None,
            badge_count: None,
            replace_helper_hint: None,
        }
    }

    #[tokio::test]
    async fn send_returns_missing_configuration_without_app_id_or_helpers() {
        let backend = WindowsBackend::with_helpers(
            WindowsDesktopConfig {
                app_id: None,
                prefer_helpers: Vec::new(),
            },
            Vec::new(),
        );

        let error = backend.send(request()).await.unwrap_err();
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
        let backend = WindowsBackend::with_helpers(WindowsDesktopConfig::default(), Vec::new());
        assert_eq!(backend.platform(), DesktopPlatform::Windows);
    }

    #[test]
    fn bootstrap_check_rejects_missing_app_id() {
        let backend = WindowsBackend::with_helpers(WindowsDesktopConfig::default(), Vec::new());
        let error = backend.check_bootstrap().unwrap_err();
        assert!(matches!(
            error,
            MessengerError::MissingConfiguration {
                provider: ProviderKind::Desktop,
                field: WINDOWS_SETUP_REQUIRED,
            }
        ));
    }

    #[tokio::test]
    async fn elected_helper_success_annotates_metadata() {
        let helper = FakeHelper::new(
            HelperName::SnoreToast,
            90,
            vec![Ok(DesktopNotificationReceipt::new("snore-1"))],
        );
        let backend = WindowsBackend::with_helpers(config_with_app_id(), vec![helper]);
        let receipt = backend.send(request()).await.unwrap();
        assert_eq!(receipt.notification_id, "snore-1");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("snore_toast"),
        );
        assert!(!receipt.metadata.contains_key("helper_fallbacks"));
    }

    #[tokio::test]
    async fn helper_failure_falls_through_to_next() {
        let failing = FakeHelper::new(
            HelperName::SnoreToast,
            90,
            vec![Err(HelperError::Exited {
                status: 1,
                stderr: "boom".into(),
            })],
        );
        let working = FakeHelper::new(
            HelperName::BurntToast,
            40,
            vec![Ok(DesktopNotificationReceipt::new("burnt-1"))],
        );
        let backend = WindowsBackend::with_helpers(config_with_app_id(), vec![failing, working]);
        let receipt = backend.send(request()).await.unwrap();
        assert_eq!(receipt.notification_id, "burnt-1");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("burnt_toast"),
        );
        let fallbacks = receipt
            .metadata
            .get("helper_fallbacks")
            .map(String::as_str)
            .unwrap_or("");
        assert!(
            fallbacks.contains("snore_toast:exited"),
            "got `{fallbacks}`"
        );
    }

    #[tokio::test]
    async fn parse_error_propagates_without_fallback() {
        let parse_failing = FakeHelper::new(
            HelperName::SnoreToast,
            90,
            vec![Err(HelperError::Parse("bad".into()))],
        );
        let working = FakeHelper::new(
            HelperName::BurntToast,
            40,
            vec![Ok(DesktopNotificationReceipt::new("ignored"))],
        );
        let backend =
            WindowsBackend::with_helpers(config_with_app_id(), vec![parse_failing, working]);
        let result = backend.send(request()).await;
        assert!(matches!(result, Err(MessengerError::Provider { .. })));
    }

    #[tokio::test]
    async fn replace_routes_to_snoretoast_hint() {
        let helper = FakeHelper::new(
            HelperName::SnoreToast,
            90,
            vec![Ok(DesktopNotificationReceipt::new("snore-1"))],
        );
        let backend = WindowsBackend::with_helpers(config_with_app_id(), vec![helper]);
        let mut req = request();
        req.replace_helper_hint = Some(HelperName::SnoreToast);
        let receipt = backend.replace("99", req).await.unwrap();
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("snore_toast"),
        );
    }

    #[tokio::test]
    async fn replace_without_hinted_helper_returns_unsupported() {
        let backend = WindowsBackend::with_helpers(config_with_app_id(), Vec::new());
        let req = request();
        let result = backend.replace("99", req).await;
        assert!(matches!(
            result,
            Err(MessengerError::UnsupportedFeature { .. })
        ));
    }

    #[tokio::test]
    async fn replace_with_non_windows_hint_falls_through() {
        // A receipt produced by macOS gets handed back here in some
        // cross-platform test scenarios; the Windows backend should ignore
        // the foreign hint and report `UnsupportedFeature` rather than
        // routing to a helper that does not exist in `self.helpers`.
        let backend = WindowsBackend::with_helpers(config_with_app_id(), Vec::new());
        let mut req = request();
        req.replace_helper_hint = Some(HelperName::TerminalNotifier);
        let result = backend.replace("99", req).await;
        assert!(matches!(
            result,
            Err(MessengerError::UnsupportedFeature { .. })
        ));
    }

    #[tokio::test]
    async fn helper_score_zero_skips_to_native_when_app_id_missing() {
        // No usable helper, no app_id → MissingConfiguration is the right
        // error to surface to the caller.
        let unscored = FakeHelper::new(HelperName::BurntToast, 0, Vec::new());
        let backend = WindowsBackend::with_helpers(
            WindowsDesktopConfig {
                app_id: None,
                prefer_helpers: Vec::new(),
            },
            vec![unscored],
        );
        let error = backend.send(request()).await.unwrap_err();
        assert!(matches!(error, MessengerError::MissingConfiguration { .. }));
    }

    #[tokio::test]
    async fn prefer_helpers_breaks_score_ties() {
        let snore = FakeHelper::new(
            HelperName::SnoreToast,
            50,
            vec![Ok(DesktopNotificationReceipt::new("snore-1"))],
        );
        let burnt = FakeHelper::new(
            HelperName::BurntToast,
            50,
            vec![Ok(DesktopNotificationReceipt::new("burnt-1"))],
        );
        let mut config = config_with_app_id();
        config.prefer_helpers = vec![HelperName::BurntToast];
        let backend = WindowsBackend::with_helpers(config, vec![snore, burnt]);
        let receipt = backend.send(request()).await.unwrap();
        assert_eq!(receipt.notification_id, "burnt-1");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("burnt_toast"),
        );
    }

    #[tokio::test]
    async fn replace_with_unsupported_hint_uses_unsupported_feature_error() {
        // An action helper exists but is not the hinted one — the backend
        // should not attempt to use it for a replace dispatched with the
        // wrong hint, but should fall through to the native path which
        // reports `UnsupportedFeature` for replace.
        let helper = FakeHelper::new(
            HelperName::BurntToast,
            40,
            vec![Ok(DesktopNotificationReceipt::new("ignored"))],
        );
        let backend = WindowsBackend::with_helpers(config_with_app_id(), vec![helper]);
        let mut req = request();
        // Hint another helper that does support replace; with no SnoreToast
        // helper installed, the lookup misses and we fall through.
        req.replace_helper_hint = Some(HelperName::SnoreToast);
        let result = backend.replace("99", req).await;
        assert!(matches!(
            result,
            Err(MessengerError::UnsupportedFeature { .. })
        ));
    }

    #[tokio::test]
    async fn interactive_request_uses_helper_when_available() {
        // Confirms that elected helpers fire when actions are present —
        // important for verifying election logic on the Windows backend.
        let helper = FakeHelper::new(
            HelperName::SnoreToast,
            90,
            vec![Ok(DesktopNotificationReceipt::new("snore-1"))],
        );
        let backend = WindowsBackend::with_helpers(config_with_app_id(), vec![helper]);
        let mut req = request();
        req.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let receipt = backend.send(req).await.unwrap();
        assert_eq!(receipt.notification_id, "snore-1");
    }

    #[tokio::test]
    async fn snoretoast_score_zero_falls_through_to_burnttoast() {
        // Step 4.2: when SnoreToast scores 0 for a request (e.g. duplicate
        // action labels make the label→id mapping ambiguous) the election
        // must filter it out and prefer the next eligible helper rather
        // than attempting a send that cannot round-trip.
        let snore = FakeHelper::new(
            HelperName::SnoreToast,
            0,
            vec![Ok(DesktopNotificationReceipt::new("ignored"))],
        );
        let burnt = FakeHelper::new(
            HelperName::BurntToast,
            40,
            vec![Ok(DesktopNotificationReceipt::new("burnt-7"))],
        );
        let backend = WindowsBackend::with_helpers(config_with_app_id(), vec![snore, burnt]);

        let mut req = request();
        req.actions.push(NotificationAction {
            id: "ok".into(),
            label: "Same".into(),
        });
        req.actions.push(NotificationAction {
            id: "cancel".into(),
            label: "Same".into(),
        });

        let receipt = backend.send(req).await.unwrap();
        assert_eq!(receipt.notification_id, "burnt-7");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("burnt_toast"),
        );
    }

    #[test]
    fn evaluate_bootstrap_rejects_missing_shortcut() {
        // Step 4.3: even with a configured AppID, a missing Start Menu
        // shortcut leaves WinRT unable to resolve the AUMID. The backend
        // must surface `MissingConfiguration` with the canonical
        // `WINDOWS_SETUP_REQUIRED` field so callers can guide the user
        // through `messenger setup desktop`.
        let result =
            WindowsBackend::evaluate_bootstrap("Test.App", ShortcutBootstrapState::Missing);
        let error = result.expect_err("missing shortcut should reject bootstrap");
        assert!(matches!(
            error,
            MessengerError::MissingConfiguration {
                provider: ProviderKind::Desktop,
                field: WINDOWS_SETUP_REQUIRED,
            }
        ));
    }

    #[test]
    fn evaluate_bootstrap_accepts_present_or_unknown_states() {
        for state in [
            ShortcutBootstrapState::Present,
            ShortcutBootstrapState::Unknown,
        ] {
            let app_id = WindowsBackend::evaluate_bootstrap("Test.App", state)
                .expect("present/unknown must resolve to the app id");
            assert_eq!(app_id, "Test.App");
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn native_send_returns_transport_error_on_non_windows() {
        // Step 4.5: with no helpers registered on a non-Windows host, the
        // native WinRT path is the only option left. It must report a clear
        // provider error rather than panicking, so the backend stays safe to
        // construct on macOS/Linux for cross-platform tests and CI matrix
        // entries. `shortcut_bootstrap_state` returns `Unknown` off-Windows,
        // so `check_bootstrap` succeeds and we land in `send_toast`.
        let backend = WindowsBackend::with_helpers(config_with_app_id(), Vec::new());
        let result = backend.send(request()).await;
        let error = result.expect_err("expected provider error on non-Windows host");
        match error {
            MessengerError::Provider {
                provider, message, ..
            } => {
                assert_eq!(provider, ProviderKind::Desktop);
                assert!(
                    message.contains("Windows toast delivery is only available on Windows hosts"),
                    "unexpected message: {message}",
                );
            }
            other => panic!("expected MessengerError::Provider, got {other:?}"),
        }
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn native_fallback_attempts_winrt_when_bootstrap_present() {
        // On a real Windows host with a valid Start Menu shortcut, the
        // WinRT native path must be attempted when no helpers are registered.
        // We create a fake shortcut for the configured AppID so bootstrap
        // passes, then assert the backend does not return MissingConfiguration.
        use std::io::Write;

        let app_id = "RustyBiscuit.MessengerTests";
        let temp = tempfile::tempdir().unwrap();
        let shortcut_dir = temp
            .path()
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        std::fs::create_dir_all(&shortcut_dir).unwrap();
        std::fs::File::create(shortcut_dir.join(format!("{app_id}.lnk")))
            .unwrap()
            .write_all(b"fake")
            .unwrap();

        // SAFETY: tests run sequentially in the same test binary, so
        // temporarily overriding APPDATA does not race with other tests.
        unsafe {
            std::env::set_var("APPDATA", temp.path());
        }
        let backend = WindowsBackend::with_helpers(
            WindowsDesktopConfig {
                app_id: Some(app_id.into()),
                prefer_helpers: Vec::new(),
            },
            Vec::new(),
        );
        let result = backend.send(request()).await;
        unsafe {
            std::env::remove_var("APPDATA");
        }

        // Bootstrap must pass — we should never see MissingConfiguration.
        assert!(
            !matches!(result, Err(MessengerError::MissingConfiguration { .. })),
            "expected bootstrap to pass with fake shortcut, got {result:?}"
        );

        match result {
            Ok(receipt) => {
                assert!(
                    receipt.metadata.get("helper_used").map(String::as_str) == Some("native")
                        || receipt.metadata.get("delivery").map(String::as_str) == Some("winrt"),
                    "expected native WinRT receipt metadata, got {:?}",
                    receipt.metadata,
                );
            }
            Err(MessengerError::Transport { provider, message })
                if provider == ProviderKind::Desktop && message.contains("WinRT toast failed") =>
            {
                // OS limitation: this runner passed bootstrap validation but
                // cannot display unbundled WinRT toasts in the test host.
            }
            Err(other) => panic!("expected native receipt or WinRT host limitation, got {other:?}"),
        }
    }
}
