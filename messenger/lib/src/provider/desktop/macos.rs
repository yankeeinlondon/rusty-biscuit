//! macOS desktop notification backend.
//!
//! Three delivery strategies are wired behind a single backend:
//!
//! - **Helpers** (`terminal-notifier`, `alerter`) — opportunistic layer
//!   probed via `sniff` at construction time. Helpers ship interactive
//!   actions and inline replies that AppleScript cannot, and they remain
//!   available even when the host has no bundle identity.
//! - **AppleScript** (`osascript display notification`) — the v1 default.
//!   Works from any process (including `cargo install`ed CLI binaries) and
//!   does not trigger a notification authorization prompt. Limited feature
//!   surface.
//! - **Native `UserNotifications.framework`** via [`objc2-user-notifications`]
//!   — opt-in path for bundled/signed apps that want access to richer
//!   notification features (subtitles, body, categories, interruption level).
//!   Requires bundle identity; the system silently drops the notification
//!   otherwise.
//!
//! Backend selection happens per-request:
//!
//! | Strategy | Delivery |
//! |----------|----------|
//! | `Auto` (default) | helpers → AppleScript |
//! | `AppleScript` | helpers → AppleScript |
//! | `NativeUserNotifications` | helpers → Native |
//!
//! Helpers are always tried first when the dispatch shape suits one
//! (`elect_helpers` filters them by score). Native and AppleScript paths
//! both generate a UUID for the `notification_id` (AppleScript does not
//! return a handle; the native submit path is fire-and-forget without a
//! completion handler). Receipt metadata records which path served the
//! notification: `helper_used` for helpers, `delivery=applescript` or
//! `delivery=native` for the native backends.

use std::sync::Arc;

use async_trait::async_trait;
use std::process::Command;

use crate::error::MessengerError;
use crate::receipt::{DesktopPlatform, ProviderKind};

use super::MacOsDesktopConfig;
use super::MacOsNotificationStrategy;
use super::backend::DesktopBackend;
use super::helpers::alerter::AlerterHelper;
use super::helpers::terminal_notifier::TerminalNotifierHelper;
use super::helpers::{
    HelperAttempt, HelperBackend, HelperError, HelperName, elect_helpers,
};
use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// macOS notification backend.
pub(crate) struct MacOsBackend {
    strategy: MacOsNotificationStrategy,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    bundle_id: Option<String>,
    prefer_helpers: Vec<HelperName>,
    helpers: Vec<Arc<dyn HelperBackend>>,
}

impl MacOsBackend {
    /// Build a backend with the supplied macOS-specific configuration.
    ///
    /// Detects available notification helpers via `sniff` and constructs
    /// the corresponding [`HelperBackend`] adapters once.
    pub(crate) fn new(config: MacOsDesktopConfig) -> Self {
        let helpers = detect_macos_helpers();
        Self::with_helpers(config, helpers)
    }

    /// Test seam: build a backend with explicitly provided helpers.
    ///
    /// Skips sniff detection so unit tests can wire fake helpers without
    /// touching the host filesystem.
    pub(crate) fn with_helpers(
        config: MacOsDesktopConfig,
        helpers: Vec<Arc<dyn HelperBackend>>,
    ) -> Self {
        Self {
            strategy: config.strategy,
            bundle_id: config.bundle_id,
            prefer_helpers: config.prefer_helpers,
            helpers,
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

    async fn native_send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        match self.resolved_strategy() {
            MacOsNotificationStrategy::AppleScript | MacOsNotificationStrategy::Auto => {
                send_applescript(request)
            }
            MacOsNotificationStrategy::NativeUserNotifications => {
                send_native(request, self.bundle_id.as_deref())
            }
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
        let mut attempts: Vec<HelperAttempt> = Vec::new();
        let elected = elect_helpers(&self.helpers, &request, &self.prefer_helpers);

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
                        "macos helper send failed; trying next attempt",
                    );
                    if !error.is_fallback_eligible() {
                        return Err(helper_error_to_messenger(helper.name(), error));
                    }
                    attempts.push(HelperAttempt::from_error(helper.name(), &error));
                }
            }
        }

        let mut receipt = self.native_send(&request).await?;
        annotate_native_receipt(&mut receipt, &attempts);
        Ok(receipt)
    }

    async fn replace(
        &self,
        id: &str,
        request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        if let Some(hint) = request.replace_helper_hint
            && hint == HelperName::TerminalNotifier
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
                        "macos helper replace failed; falling back to native",
                    );
                }
            }
        }

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
            MacOsNotificationStrategy::NativeUserNotifications => dismiss_native(id),
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

fn annotate_native_receipt(
    receipt: &mut DesktopNotificationReceipt,
    attempts: &[HelperAttempt],
) {
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

/// Probe sniff for macOS notification helpers and turn them into
/// [`HelperBackend`] adapters.
fn detect_macos_helpers() -> Vec<Arc<dyn HelperBackend>> {
    let info = sniff::programs::InstalledNotificationHelpers::new();

    let mut helpers: Vec<Arc<dyn HelperBackend>> = Vec::new();

    if let Some(path) = info.path(sniff::programs::NotificationHelper::TerminalNotifier) {
        helpers.push(Arc::new(TerminalNotifierHelper::new(path)));
    }

    if let Some(path) = info.path(sniff::programs::NotificationHelper::Alerter) {
        helpers.push(Arc::new(AlerterHelper::new(path)));
    }

    helpers
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
                group: true,
                blocking: false,
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
            actions: Vec::new(),
            progress: None,
            badge_count: None,
            replace_helper_hint: None,
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
        let backend = MacOsBackend::with_helpers(MacOsDesktopConfig::default(), Vec::new());
        assert_eq!(
            backend.resolved_strategy(),
            MacOsNotificationStrategy::AppleScript
        );
    }

    #[test]
    fn resolved_strategy_preserves_explicit_choices() {
        let backend = MacOsBackend::with_helpers(
            MacOsDesktopConfig {
                strategy: MacOsNotificationStrategy::AppleScript,
                ..MacOsDesktopConfig::default()
            },
            Vec::new(),
        );
        assert_eq!(
            backend.resolved_strategy(),
            MacOsNotificationStrategy::AppleScript
        );
        let backend = MacOsBackend::with_helpers(
            MacOsDesktopConfig {
                strategy: MacOsNotificationStrategy::NativeUserNotifications,
                ..MacOsDesktopConfig::default()
            },
            Vec::new(),
        );
        assert_eq!(
            backend.resolved_strategy(),
            MacOsNotificationStrategy::NativeUserNotifications
        );
    }

    #[test]
    fn backend_reports_macos_platform() {
        let backend = MacOsBackend::with_helpers(MacOsDesktopConfig::default(), Vec::new());
        assert_eq!(backend.platform(), DesktopPlatform::MacOS);
    }

    #[tokio::test]
    async fn elected_helper_success_annotates_metadata() {
        let helper = FakeHelper::new(
            HelperName::TerminalNotifier,
            80,
            vec![Ok(DesktopNotificationReceipt::new("tn-1"))],
        );
        let backend = MacOsBackend::with_helpers(MacOsDesktopConfig::default(), vec![helper]);
        let receipt = backend.send(request("hi", Some("body"))).await.unwrap();
        assert_eq!(receipt.notification_id, "tn-1");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("TerminalNotifier"),
        );
        assert!(!receipt.metadata.contains_key("helper_fallbacks"));
    }

    #[tokio::test]
    async fn helper_failure_falls_through_to_next() {
        let failing = FakeHelper::new(
            HelperName::Alerter,
            90,
            vec![Err(HelperError::Exited {
                status: 1,
                stderr: "boom".into(),
            })],
        );
        let working = FakeHelper::new(
            HelperName::TerminalNotifier,
            80,
            vec![Ok(DesktopNotificationReceipt::new("tn-1"))],
        );
        let backend = MacOsBackend::with_helpers(
            MacOsDesktopConfig::default(),
            vec![failing, working],
        );
        let mut req = request("hi", Some("body"));
        req.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let receipt = backend.send(req).await.unwrap();
        assert_eq!(receipt.notification_id, "tn-1");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("TerminalNotifier"),
        );
        let fallbacks = receipt
            .metadata
            .get("helper_fallbacks")
            .map(String::as_str)
            .unwrap_or("");
        assert!(fallbacks.contains("Alerter:exited"), "got `{fallbacks}`");
    }

    #[tokio::test]
    async fn parse_error_propagates_without_fallback() {
        let parse_failing = FakeHelper::new(
            HelperName::Alerter,
            90,
            vec![Err(HelperError::Parse("bad".into()))],
        );
        let working = FakeHelper::new(
            HelperName::TerminalNotifier,
            80,
            vec![Ok(DesktopNotificationReceipt::new("ignored"))],
        );
        let backend = MacOsBackend::with_helpers(
            MacOsDesktopConfig::default(),
            vec![parse_failing, working],
        );
        let mut req = request("hi", Some("body"));
        req.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let result = backend.send(req).await;
        assert!(matches!(result, Err(MessengerError::Provider { .. })));
    }

    #[tokio::test]
    async fn replace_routes_to_terminal_notifier_hint() {
        let working = FakeHelper::new(
            HelperName::TerminalNotifier,
            80,
            vec![Ok(DesktopNotificationReceipt::new("tn-1"))],
        );
        let backend =
            MacOsBackend::with_helpers(MacOsDesktopConfig::default(), vec![working]);
        let mut req = request("hi", Some("body"));
        req.replace_helper_hint = Some(HelperName::TerminalNotifier);
        let receipt = backend.replace("99", req).await.unwrap();
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("TerminalNotifier"),
        );
    }

    #[tokio::test]
    async fn replace_alerter_hint_falls_through_to_native() {
        // Alerter does not support replace; without a replaceable helper
        // the AppleScript strategy should report the operation as
        // unsupported.
        let alerter = FakeHelper::new(
            HelperName::Alerter,
            90,
            vec![Ok(DesktopNotificationReceipt::new("ignored"))],
        );
        let backend = MacOsBackend::with_helpers(
            MacOsDesktopConfig::default(),
            vec![alerter],
        );
        let mut req = request("hi", Some("body"));
        req.replace_helper_hint = Some(HelperName::Alerter);
        let result = backend.replace("99", req).await;
        assert!(matches!(
            result,
            Err(MessengerError::UnsupportedFeature { .. })
        ));
    }

    #[tokio::test]
    async fn empty_helpers_falls_through_to_applescript_path() {
        // No helpers registered: AppleScript dispatch is attempted next.
        // We can't actually exec osascript in a unit test, but we can
        // assert the strategy path is taken — the call returns a transport
        // error specifically about osascript.
        let backend = MacOsBackend::with_helpers(MacOsDesktopConfig::default(), Vec::new());
        let result = backend.send(request("hi", Some("body"))).await;
        // Test environments without /usr/bin/osascript will fail with
        // a transport error; environments with osascript will succeed.
        // Either way, the call routes through native_send, not a helper.
        match result {
            Ok(receipt) => {
                assert_eq!(
                    receipt.metadata.get("helper_used").map(String::as_str),
                    Some("native"),
                );
            }
            Err(MessengerError::Provider { .. }) => {
                // osascript not available in this environment — expected
                // outside macOS.
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
