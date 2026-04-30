//! Linux desktop notification backend.
//!
//! Delivers notifications through the freedesktop.org D-Bus Notifications
//! interface via the [`notify-rust`](https://docs.rs/notify-rust) crate, with
//! an opportunistic helper layer (`dunstify`, `notify-send`) that ships
//! interactive features the native `notify-rust` path does not surface.
//!
//! ## Notes
//!
//! - Detected helpers are tried before the native fallback in score order.
//!   Native delivery remains the universal floor.
//! - Returns the daemon-assigned notification ID (or helper-issued id) as
//!   the `MessageRef::Desktop` `notification_id`.
//! - Uses D-Bus hints (`urgency`, `category`, `desktop-entry`, `image-path`,
//!   `suppress-sound`) for best-effort enrichment.

use std::sync::Arc;

use async_trait::async_trait;
use notify_rust::{Hint, Notification, Timeout, Urgency as NotifyUrgency};

use crate::dispatch::{NotificationIcon, NotificationUrgency};
use crate::error::MessengerError;
use crate::receipt::{DesktopPlatform, ProviderKind};

use super::LinuxDesktopConfig;
use super::backend::DesktopBackend;
use super::helpers::dunstify::DunstifyHelper;
use super::helpers::notify_send::NotifySendHelper;
use super::helpers::{HelperAttempt, HelperBackend, HelperError, HelperName, elect_helpers};
use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// D-Bus notification backend for Linux / freedesktop.org desktops.
pub(crate) struct LinuxBackend {
    desktop_entry: Option<String>,
    prefer_helpers: Vec<HelperName>,
    helpers: Vec<Arc<dyn HelperBackend>>,
}

impl LinuxBackend {
    /// Build a backend with the supplied Linux-specific configuration.
    ///
    /// Detects available notification helpers via `sniff` and constructs
    /// the corresponding [`HelperBackend`] adapters once.
    pub(crate) fn new(config: LinuxDesktopConfig) -> Self {
        let helpers = detect_linux_helpers();
        Self::with_helpers(config, helpers)
    }

    /// Test seam: build a backend with explicitly provided helpers.
    ///
    /// Skips sniff detection so unit tests can wire fake helpers without
    /// touching the host filesystem or D-Bus.
    pub(crate) fn with_helpers(
        config: LinuxDesktopConfig,
        helpers: Vec<Arc<dyn HelperBackend>>,
    ) -> Self {
        Self {
            desktop_entry: config.desktop_entry,
            prefer_helpers: config.prefer_helpers,
            helpers,
        }
    }

    /// Build the native (notify-rust) notification object from the request.
    fn build_native_notification(&self, request: &DesktopNotificationRequest) -> Notification {
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

        if let Some(group_id) = request.group_id.as_deref() {
            notification.hint(Hint::Custom("group".to_string(), group_id.to_string()));
        }

        if let Some(progress) = request.progress {
            notification.hint(Hint::Custom(
                "progress".to_string(),
                format!("{}/{}", progress.current, progress.total),
            ));
            notification.hint(Hint::Custom(
                "progress-value".to_string(),
                format!(
                    "{:.2}",
                    progress.current as f64 / progress.total.max(1) as f64
                ),
            ));
        }

        if let Some(badge_count) = request.badge_count {
            notification.hint(Hint::Custom(
                "badge-count".to_string(),
                badge_count.to_string(),
            ));
        }

        for action in &request.actions {
            notification.action(&action.id, &action.label);
        }

        notification
    }

    async fn native_send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        let mut notification = self.build_native_notification(request);
        if let Some(replace_id) = request.replace_id.as_deref()
            && let Ok(id_u32) = replace_id.parse::<u32>()
        {
            notification.id(id_u32);
        }
        let handle = notification.show_async().await.map_err(|error| {
            ProviderKind::Desktop.transport_error(format!("D-Bus notification failed: {error}"))
        })?;
        Ok(DesktopNotificationReceipt::new(handle.id().to_string()))
    }

    async fn native_replace(
        &self,
        id: &str,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        let mut notification = self.build_native_notification(request);
        let id_u32 = id.parse::<u32>().map_err(|_| {
            ProviderKind::Desktop.transport_error(format!(
                "invalid Linux notification id for replacement: {id}"
            ))
        })?;
        notification.id(id_u32);
        let handle = notification.show_async().await.map_err(|error| {
            ProviderKind::Desktop
                .transport_error(format!("D-Bus notification replace failed: {error}"))
        })?;
        Ok(DesktopNotificationReceipt::new(handle.id().to_string()).with_metadata("replaced", id))
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
                        "linux helper send failed; trying next attempt",
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
            && hint != HelperName::NotifySend
            && hint != HelperName::Dunstify
        {
            // Hint names a helper from another OS — silently fall through.
        } else if let Some(hint) = request.replace_helper_hint
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
                        "linux helper replace failed; falling back to native",
                    );
                }
            }
        }

        self.native_replace(id, &request).await
    }
}

fn map_urgency(urgency: NotificationUrgency) -> NotifyUrgency {
    match urgency {
        NotificationUrgency::Low => NotifyUrgency::Low,
        NotificationUrgency::Normal => NotifyUrgency::Normal,
        NotificationUrgency::Critical => NotifyUrgency::Critical,
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

/// Probe sniff for Linux notification helpers and turn them into
/// [`HelperBackend`] adapters.
fn detect_linux_helpers() -> Vec<Arc<dyn HelperBackend>> {
    let info = sniff::programs::InstalledNotificationHelpers::new();
    let daemon_is_dunst = info
        .active_daemon
        .as_ref()
        .map(|daemon| daemon.name.eq_ignore_ascii_case("dunst"))
        .unwrap_or(false);

    let mut helpers: Vec<Arc<dyn HelperBackend>> = Vec::new();

    if let Some(path) = info.path(sniff::programs::NotificationHelper::Dunstify) {
        helpers.push(Arc::new(DunstifyHelper::new(path, daemon_is_dunst)));
    }

    if let Some(path) = info.path(sniff::programs::NotificationHelper::NotifySend) {
        let version = info
            .version(sniff::programs::NotificationHelper::NotifySend)
            .ok()
            .and_then(|raw| parse_libnotify_version(&raw));
        helpers.push(Arc::new(NotifySendHelper::new(path, version)));
    }

    helpers
}

/// Parse libnotify's `notify-send --version` output into a `(major, minor, patch)` tuple.
///
/// libnotify prints lines like `notify-send 0.8.3`. We accept any leading
/// non-numeric prefix and grab the first dotted-numeric token. Unparseable
/// strings yield `None` so the caller defaults to "no actions" behavior.
fn parse_libnotify_version(raw: &str) -> Option<(u32, u32, u32)> {
    let token = raw
        .split_whitespace()
        .find(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts
        .next()
        .map(|chunk| {
            chunk
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .unwrap_or("0")
                .parse::<u32>()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::desktop::helpers::{HelperBackend, HelperCapabilities};
    use std::sync::Mutex;

    struct FakeHelper {
        helper_name: HelperName,
        score_for: u8,
        outcome: Mutex<Vec<Result<DesktopNotificationReceipt, HelperError>>>,
    }

    impl FakeHelper {
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
                reply: false,
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

    fn request() -> DesktopNotificationRequest {
        DesktopNotificationRequest {
            title: "Hi".into(),
            body: None,
            subtitle: None,
            app_name: "App".into(),
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
    async fn elected_helper_success_annotates_metadata() {
        let helper = FakeHelper::new(
            HelperName::Dunstify,
            70,
            vec![Ok(DesktopNotificationReceipt::new("dunst-1"))],
        );
        let backend = LinuxBackend::with_helpers(LinuxDesktopConfig::default(), vec![helper]);
        let receipt = backend.send(request()).await.unwrap();
        assert_eq!(receipt.notification_id, "dunst-1");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("Dunstify"),
        );
        assert!(!receipt.metadata.contains_key("helper_fallbacks"));
    }

    #[tokio::test]
    async fn helper_failure_falls_through_to_next() {
        let failing = FakeHelper::new(
            HelperName::Dunstify,
            70,
            vec![Err(HelperError::Exited {
                status: 1,
                stderr: "boom".into(),
            })],
        );
        let working = FakeHelper::new(
            HelperName::NotifySend,
            60,
            vec![Ok(DesktopNotificationReceipt::new("send-1"))],
        );
        let backend =
            LinuxBackend::with_helpers(LinuxDesktopConfig::default(), vec![failing, working]);
        let receipt = backend.send(request()).await.unwrap();
        assert_eq!(receipt.notification_id, "send-1");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("NotifySend"),
        );
        let fallbacks = receipt
            .metadata
            .get("helper_fallbacks")
            .map(String::as_str)
            .unwrap_or("");
        assert!(fallbacks.contains("Dunstify:exited"), "got `{fallbacks}`");
    }

    #[tokio::test]
    async fn parse_error_propagates_without_fallback() {
        let parse_failing = FakeHelper::new(
            HelperName::Dunstify,
            70,
            vec![Err(HelperError::Parse("bad".into()))],
        );
        let working = FakeHelper::new(
            HelperName::NotifySend,
            60,
            vec![Ok(DesktopNotificationReceipt::new("ignored"))],
        );
        let backend =
            LinuxBackend::with_helpers(LinuxDesktopConfig::default(), vec![parse_failing, working]);
        let result = backend.send(request()).await;
        assert!(matches!(result, Err(MessengerError::Provider { .. })));
    }

    #[tokio::test]
    async fn replace_routes_to_hinted_helper() {
        let working = FakeHelper::new(
            HelperName::NotifySend,
            60,
            vec![Ok(DesktopNotificationReceipt::new("send-1"))],
        );
        let backend = LinuxBackend::with_helpers(LinuxDesktopConfig::default(), vec![working]);
        let mut request = request();
        request.replace_helper_hint = Some(HelperName::NotifySend);
        let receipt = backend.replace("99", request).await.unwrap();
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("NotifySend"),
        );
    }

    #[test]
    fn parse_libnotify_version_handles_canonical_output() {
        assert_eq!(
            super::parse_libnotify_version("notify-send 0.8.3"),
            Some((0, 8, 3)),
        );
        assert_eq!(super::parse_libnotify_version("0.7.8"), Some((0, 7, 8)),);
        assert_eq!(super::parse_libnotify_version("notify-send"), None);
        assert_eq!(
            super::parse_libnotify_version("notify-send 1.0"),
            Some((1, 0, 0))
        );
    }

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
        let backend = LinuxBackend::with_helpers(LinuxDesktopConfig::default(), Vec::new());
        assert_eq!(backend.platform(), DesktopPlatform::Linux);
    }
}
