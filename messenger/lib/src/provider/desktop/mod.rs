//! Desktop notifications provider.
//!
//! Provides a single [`Provider`](crate::provider::Provider) that delivers a
//! [`Message`](crate::message::Message) to the host operating system's
//! notification center. Runtime backend selection (Linux, macOS, Windows)
//! lands in Phase 4; this module wires up the request-construction and
//! provider-send surface so that tests and the CLI can exercise the
//! public provider API with a swappable in-memory backend.

pub(crate) mod backend;
pub(crate) mod helpers;
pub(crate) mod linux;
pub(crate) mod macos;
pub mod request;
pub(crate) mod windows;

use std::path::PathBuf;
use std::sync::Arc;

use crate::attachment::{AttachmentKind, AttachmentSource};
use crate::capabilities::CapabilitySet;
use crate::dispatch::{Dispatch, NotificationIcon, NotificationUrgency, ProviderOverrides};
use crate::error::MessengerError;
use crate::prepared::PreparedMessage;
use crate::receipt::{DesktopPlatform, MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

pub(crate) use backend::DesktopBackend;
pub use helpers::HelperName;
pub use request::{DesktopNotificationReceipt, DesktopNotificationRequest};

/// Portable configuration for the desktop provider.
///
/// Platform-specific tweaks live in nested structs; the top-level fields
/// describe what every backend can consume.
#[derive(Debug, Clone)]
pub struct DesktopConfig {
    /// Application name shown by the notification center.
    pub app_name: String,
    /// Default title used when a message does not supply its own.
    pub default_title: Option<String>,
    /// Default category / thread identifier. Maps to D-Bus `category` on
    /// Linux, `categoryIdentifier` on macOS native, and `tag` on Windows.
    pub category: Option<String>,
    /// Default urgency/interruption level.
    pub urgency: NotificationUrgency,
    /// Default expiry timeout in milliseconds (backend-dependent accuracy).
    pub timeout_ms: Option<u32>,
    /// Default icon reference.
    pub icon: Option<NotificationIcon>,
    /// Default notification actions.
    pub actions: Vec<crate::dispatch::NotificationAction>,
    /// Default progress indicator.
    pub progress: Option<crate::dispatch::NotificationProgress>,
    /// Default badge count.
    pub badge_count: Option<u32>,
    /// Windows-specific configuration.
    pub windows: WindowsDesktopConfig,
    /// macOS-specific configuration.
    pub macos: MacOsDesktopConfig,
    /// Linux-specific configuration.
    pub linux: LinuxDesktopConfig,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            app_name: "Messenger".to_string(),
            default_title: None,
            category: None,
            urgency: NotificationUrgency::default(),
            timeout_ms: None,
            icon: None,
            actions: Vec::new(),
            progress: None,
            badge_count: None,
            windows: WindowsDesktopConfig::default(),
            macos: MacOsDesktopConfig::default(),
            linux: LinuxDesktopConfig::default(),
        }
    }
}

/// Windows-specific desktop configuration.
#[derive(Debug, Clone, Default)]
pub struct WindowsDesktopConfig {
    /// App User Model ID used for toast notifications. Set by
    /// `messenger setup desktop`; unset sends on Windows must fail with
    /// [`MessengerError::MissingConfiguration`].
    pub app_id: Option<String>,
    /// Caller-supplied helper preference order.
    ///
    /// Helpers named here win score ties during election. Names that do
    /// not apply on Windows (e.g. `dunstify`) are silently ignored. Empty
    /// means "use the library default order".
    pub prefer_helpers: Vec<helpers::HelperName>,
}

/// macOS-specific desktop configuration.
#[derive(Debug, Clone, Default)]
pub struct MacOsDesktopConfig {
    /// Bundle identifier used when invoking the native framework.
    pub bundle_id: Option<String>,
    /// How the macOS backend should pick between AppleScript and native
    /// `UserNotifications.framework` delivery.
    pub strategy: MacOsNotificationStrategy,
    /// Caller-supplied helper preference order.
    ///
    /// Helpers named here win score ties during election. Names that do
    /// not apply on macOS (e.g. `dunstify`) are silently ignored. Empty
    /// means "use the library default order".
    pub prefer_helpers: Vec<helpers::HelperName>,
}

/// macOS notification delivery strategy.
///
/// v1 intentionally leaves [`Auto`](Self::Auto) mapped to AppleScript
/// delivery. Promoting native delivery to the default requires a bundled,
/// signed app identity and a persistent authorization story; those arrive
/// in a later phase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MacOsNotificationStrategy {
    /// Default v1 strategy; maps to AppleScript delivery unconditionally.
    #[default]
    Auto,
    /// Explicit opt-in for native `UserNotifications.framework`.
    NativeUserNotifications,
    /// Explicit AppleScript delivery via `osascript`.
    AppleScript,
}

/// Linux-specific desktop configuration.
#[derive(Debug, Clone, Default)]
pub struct LinuxDesktopConfig {
    /// Optional desktop entry name used as the `desktop-entry` D-Bus hint.
    pub desktop_entry: Option<String>,
    /// Caller-supplied helper preference order.
    ///
    /// Helpers named here win score ties during election. Names that do
    /// not apply on Linux (e.g. `terminal-notifier`) are silently ignored.
    /// Empty means "use the library default order".
    pub prefer_helpers: Vec<helpers::HelperName>,
}

/// Cross-platform desktop notification provider.
///
/// Backend selection happens at construction time via
/// [`DesktopNotificationProvider::new`] (Phase 4 will wire the host-OS
/// backends). Tests and integration code can use
/// [`DesktopNotificationProvider::with_backend`] to inject a fake backend.
pub struct DesktopNotificationProvider {
    config: DesktopConfig,
    backend: Arc<dyn DesktopBackend>,
}

impl DesktopNotificationProvider {
    /// Build a provider bound to the host-OS backend chosen at runtime.
    ///
    /// Backend selection matches the compile target:
    ///
    /// - `target_os = "linux"` → `notify-rust` D-Bus backend
    /// - `target_os = "macos"` → AppleScript / native `UserNotifications`
    /// - `target_os = "windows"` → `winrt-notification` toast backend
    /// - other targets → an unimplemented backend that fails every send with
    ///   [`MessengerError::MissingConfiguration`], so desktop-aware callers
    ///   can still register the provider and plan sends in best-effort tests.
    ///
    /// Use [`DesktopNotificationProvider::with_backend`] when you need to
    /// inject a custom or fake backend (e.g. from unit tests).
    pub fn new(config: DesktopConfig) -> Self {
        let backend: Arc<dyn DesktopBackend> = select_backend(&config);
        Self::with_backend(config, backend)
    }

    /// Build a provider with an explicit [`DesktopBackend`] implementation.
    ///
    /// This is the integration point used by host-OS bootstrapping and by
    /// tests that need to inspect the request a backend receives.
    pub(crate) fn with_backend(config: DesktopConfig, backend: Arc<dyn DesktopBackend>) -> Self {
        Self { config, backend }
    }

    /// Read access to the provider's configuration.
    pub fn config(&self) -> &DesktopConfig {
        &self.config
    }

    /// Replace an existing desktop notification.
    ///
    /// The `receipt` must be a [`SendReceipt`] from a prior desktop send.
    /// Backends that support replacement (Linux D-Bus, macOS native) will
    /// update the notification in-place; others return
    /// [`MessengerError::UnsupportedFeature`].
    pub async fn replace(
        &self,
        receipt: &SendReceipt,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        if receipt.provider != ProviderKind::Desktop {
            return Err(MessengerError::InvalidMessage(
                "replace requires a Desktop receipt".into(),
            ));
        }

        let platform = self.backend.platform();
        tracing::Span::current().record("platform", tracing::field::display(platform));

        let mut request = build_request(&self.config, dispatch, message)?;
        request.replace_helper_hint = receipt
            .metadata
            .get("helper_used")
            .and_then(|s| s.parse().ok());
        let backend_receipt = self.backend.replace(&receipt.raw_id, request).await?;

        let mut metadata = backend_receipt.metadata;
        if !metadata.contains_key("platform") {
            metadata.insert("platform".to_string(), platform.to_string());
        }

        Ok(SendReceipt {
            provider: ProviderKind::Desktop,
            message_ref: MessageRef::Desktop {
                platform,
                notification_id: backend_receipt.notification_id.clone(),
            },
            raw_id: backend_receipt.notification_id,
            metadata,
        })
    }

    /// Dismiss a delivered desktop notification.
    ///
    /// The `receipt` must be a [`SendReceipt`] from a prior desktop send.
    /// Backends that support dismissal (macOS native) will remove the
    /// notification from the notification center; others return
    /// [`MessengerError::UnsupportedFeature`].
    pub async fn dismiss(&self, receipt: &SendReceipt) -> Result<(), MessengerError> {
        if receipt.provider != ProviderKind::Desktop {
            return Err(MessengerError::InvalidMessage(
                "dismiss requires a Desktop receipt".into(),
            ));
        }

        self.backend.dismiss(&receipt.raw_id).await
    }

    /// Capability set advertised by the desktop provider.
    ///
    /// The set is a module-level constant so both
    /// [`DesktopNotificationProvider::capabilities`] and library consumers
    /// doing their own normalization can see the same answer.
    pub fn capability_set() -> CapabilitySet {
        CapabilitySet {
            supports_markdown_rendering: false,
            supports_reply: false,
            supported_attachment_kinds: std::collections::BTreeSet::from([AttachmentKind::Image]),
            supports_location: false,
            supports_silent_delivery: true,
            supports_link_preview_control: false,
        }
    }
}

#[async_trait::async_trait]
impl super::Provider for DesktopNotificationProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Desktop
    }

    fn capabilities(&self) -> CapabilitySet {
        Self::capability_set()
    }

    #[tracing::instrument(skip_all, fields(provider = "desktop", platform = tracing::field::Empty))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        #[allow(unreachable_patterns)]
        match &dispatch.target {
            Target::Desktop(_) => {}
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected Desktop target".into(),
                ));
            }
        }

        let platform = self.backend.platform();
        tracing::Span::current().record("platform", tracing::field::display(platform));

        let request = build_request(&self.config, dispatch, message)?;
        tracing::debug!(
            title_len = request.title.len(),
            body_len = request.body.as_deref().map(|s| s.len()).unwrap_or(0),
            has_image = request.image.is_some(),
            silent = request.silent,
            "prepared desktop notification request"
        );

        let backend_receipt = self.backend.send(request).await?;

        let mut metadata = backend_receipt.metadata;
        if !metadata.contains_key("platform") {
            metadata.insert("platform".to_string(), platform.to_string());
        }

        Ok(SendReceipt {
            provider: ProviderKind::Desktop,
            message_ref: MessageRef::Desktop {
                platform,
                notification_id: backend_receipt.notification_id.clone(),
            },
            raw_id: backend_receipt.notification_id,
            metadata,
        })
    }
}

/// Build a [`DesktopNotificationRequest`] from message/dispatch/config.
///
/// The caller is responsible for routing capability-level normalization
/// (markdown downgrade, unsupported attachment drop, location drop, reply
/// drop, silent drop, link-preview drop) through the shared
/// [`normalize_dispatch`](crate::validate::normalize_dispatch) pipeline
/// before reaching this function. This keeps the desktop provider focused
/// on platform-specific request shape rather than re-implementing the
/// shared compatibility rules.
pub(crate) fn build_request(
    config: &DesktopConfig,
    dispatch: &Dispatch,
    message: &PreparedMessage,
) -> Result<DesktopNotificationRequest, MessengerError> {
    let overrides = desktop_overrides(&dispatch.overrides);

    let title = message
        .title()
        .map(str::to_string)
        .or_else(|| config.default_title.clone())
        .unwrap_or_else(|| config.app_name.clone());

    let body = {
        let rendered = message.render_body_for_provider(ProviderKind::Desktop);
        if rendered.is_empty() {
            None
        } else {
            Some(rendered)
        }
    };

    let image = first_image_path(message);

    let app_name = overrides
        .and_then(|o| o.app_name.clone())
        .unwrap_or_else(|| config.app_name.clone());

    let category = overrides
        .and_then(|o| o.category.clone())
        .or_else(|| config.category.clone());

    let urgency = overrides.and_then(|o| o.urgency).unwrap_or(config.urgency);

    let timeout_ms = overrides.and_then(|o| o.timeout_ms).or(config.timeout_ms);

    let icon = overrides
        .and_then(|o| o.icon.clone())
        .or_else(|| config.icon.clone());

    let subtitle = overrides.and_then(|o| o.subtitle.clone());
    let replace_id = overrides.and_then(|o| o.replace_id.clone());
    let group_id = overrides.and_then(|o| o.group_id.clone());

    let actions = overrides
        .and_then(|o| {
            if o.actions.is_empty() {
                None
            } else {
                Some(o.actions.clone())
            }
        })
        .unwrap_or_else(|| config.actions.clone());

    let progress = overrides.and_then(|o| o.progress).or(config.progress);

    let badge_count = overrides.and_then(|o| o.badge_count).or(config.badge_count);

    Ok(DesktopNotificationRequest {
        title,
        body,
        subtitle,
        app_name,
        icon,
        image,
        silent: dispatch.options.silent,
        category,
        urgency,
        timeout_ms,
        replace_id,
        group_id,
        actions,
        progress,
        badge_count,
        replace_helper_hint: None,
    })
}

fn desktop_overrides(overrides: &ProviderOverrides) -> Option<&crate::dispatch::DesktopOverrides> {
    match overrides {
        ProviderOverrides::Desktop(desktop) => Some(desktop),
        _ => None,
    }
}

fn first_image_path(message: &PreparedMessage) -> Option<PathBuf> {
    message
        .attachments()
        .iter()
        .find(|attachment| attachment.kind == AttachmentKind::Image)
        .and_then(|attachment| match &attachment.source {
            AttachmentSource::Path(path) => Some(path.clone()),
            _ => None,
        })
}

/// Choose the host-OS backend that best matches the compile target.
///
/// The function intentionally runs at construction time so each provider
/// instance captures its backend eagerly. Backends are cheap — they hold
/// their config snapshot and no live OS handles — so constructing one per
/// [`DesktopNotificationProvider`] is fine.
fn select_backend(config: &DesktopConfig) -> Arc<dyn DesktopBackend> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxBackend::new(config.linux.clone()))
    }
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOsBackend::new(config.macos.clone()))
    }
    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WindowsBackend::new(config.windows.clone()))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = config;
        Arc::new(UnimplementedBackend)
    }
}

/// Fallback backend used on operating systems without a wired platform
/// implementation.
///
/// Returning `MissingConfiguration` rather than panicking keeps
/// [`DesktopNotificationProvider::new`] viable as a library entry point
/// while providing a clear signal to callers that platform support is
/// unavailable on the current target.
#[cfg_attr(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    allow(dead_code)
)]
struct UnimplementedBackend;

#[async_trait::async_trait]
impl DesktopBackend for UnimplementedBackend {
    fn platform(&self) -> DesktopPlatform {
        #[cfg(target_os = "linux")]
        {
            DesktopPlatform::Linux
        }
        #[cfg(target_os = "macos")]
        {
            DesktopPlatform::MacOS
        }
        #[cfg(target_os = "windows")]
        {
            DesktopPlatform::Windows
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            DesktopPlatform::Linux
        }
    }

    async fn send(
        &self,
        _request: DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, MessengerError> {
        Err(MessengerError::MissingConfiguration {
            provider: ProviderKind::Desktop,
            field: "platform backend",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::DesktopOverrides;
    use crate::message::Message;
    use crate::target::Target;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    struct CapturingBackend {
        captured: Mutex<Option<DesktopNotificationRequest>>,
        platform: DesktopPlatform,
        response: Mutex<Option<DesktopNotificationReceipt>>,
    }

    impl CapturingBackend {
        fn with_platform(platform: DesktopPlatform) -> Self {
            Self {
                captured: Mutex::new(None),
                platform,
                response: Mutex::new(None),
            }
        }

        fn last_request(&self) -> DesktopNotificationRequest {
            self.captured
                .lock()
                .unwrap()
                .clone()
                .expect("no request captured")
        }
    }

    #[async_trait::async_trait]
    impl DesktopBackend for CapturingBackend {
        fn platform(&self) -> DesktopPlatform {
            self.platform
        }

        async fn send(
            &self,
            request: DesktopNotificationRequest,
        ) -> Result<DesktopNotificationReceipt, MessengerError> {
            *self.captured.lock().unwrap() = Some(request);
            let receipt = self
                .response
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| DesktopNotificationReceipt::new("fake-id-1"));
            Ok(receipt)
        }

        async fn replace(
            &self,
            _id: &str,
            request: DesktopNotificationRequest,
        ) -> Result<DesktopNotificationReceipt, MessengerError> {
            *self.captured.lock().unwrap() = Some(request);
            let receipt = self
                .response
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| DesktopNotificationReceipt::new("fake-id-replaced"));
            Ok(receipt)
        }

        async fn dismiss(&self, _id: &str) -> Result<(), MessengerError> {
            Ok(())
        }
    }

    fn default_config() -> DesktopConfig {
        DesktopConfig {
            app_name: "Messenger".into(),
            ..DesktopConfig::default()
        }
    }

    #[test]
    fn title_precedence_prefers_message_title() {
        let message = Message::text("body").title("Explicit Title");
        let prepared = PreparedMessage::new(&message);
        let config = DesktopConfig {
            default_title: Some("Default".into()),
            ..default_config()
        };

        let dispatch = Dispatch::to(Target::desktop());
        let request = build_request(&config, &dispatch, &prepared).unwrap();
        assert_eq!(request.title, "Explicit Title");
    }

    #[test]
    fn title_precedence_falls_back_to_default_title() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);
        let config = DesktopConfig {
            default_title: Some("Default Title".into()),
            ..default_config()
        };

        let dispatch = Dispatch::to(Target::desktop());
        let request = build_request(&config, &dispatch, &prepared).unwrap();
        assert_eq!(request.title, "Default Title");
    }

    #[test]
    fn title_precedence_falls_back_to_app_name() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);
        let config = DesktopConfig {
            app_name: "AppOnly".into(),
            default_title: None,
            ..default_config()
        };

        let dispatch = Dispatch::to(Target::desktop());
        let request = build_request(&config, &dispatch, &prepared).unwrap();
        assert_eq!(request.title, "AppOnly");
    }

    #[test]
    fn markdown_body_is_rendered_as_plain_text() {
        let message = Message::markdown("**bold** and _italic_");
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop());

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert_eq!(request.body.as_deref(), Some("bold and italic"));
    }

    #[test]
    fn overrides_win_over_config_values() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let config = DesktopConfig {
            category: Some("config-cat".into()),
            urgency: NotificationUrgency::Low,
            timeout_ms: Some(1000),
            icon: Some(NotificationIcon::Named("config-icon".into())),
            ..default_config()
        };

        let mut dispatch = Dispatch::to(Target::desktop());
        dispatch.overrides = ProviderOverrides::Desktop(DesktopOverrides {
            subtitle: Some("sub".into()),
            app_name: Some("OverrideApp".into()),
            category: Some("override-cat".into()),
            urgency: Some(NotificationUrgency::Critical),
            timeout_ms: Some(5000),
            icon: Some(NotificationIcon::Named("override-icon".into())),
            replace_id: Some("replace-1".into()),
            group_id: None,
            actions: Vec::new(),
            progress: None,
            badge_count: None,
        });

        let request = build_request(&config, &dispatch, &prepared).unwrap();
        assert_eq!(request.app_name, "OverrideApp");
        assert_eq!(request.category.as_deref(), Some("override-cat"));
        assert_eq!(request.urgency, NotificationUrgency::Critical);
        assert_eq!(request.timeout_ms, Some(5000));
        assert_eq!(
            request.icon,
            Some(NotificationIcon::Named("override-icon".into()))
        );
        assert_eq!(request.subtitle.as_deref(), Some("sub"));
        assert_eq!(request.replace_id.as_deref(), Some("replace-1"));
    }

    #[test]
    fn silent_flag_is_carried_through() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop()).silent();

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert!(request.silent);
    }

    #[test]
    fn first_image_attachment_is_selected() {
        let message = Message::text("body")
            .image("/tmp/first.png")
            .image("/tmp/second.png");
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop());

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert_eq!(
            request.image.as_deref(),
            Some(std::path::Path::new("/tmp/first.png"))
        );
    }

    #[test]
    fn body_is_none_when_message_has_no_body() {
        let message = Message {
            title: Some("Alert".into()),
            body: None,
            attachments: Vec::new(),
            location: None,
            metadata: BTreeMap::new(),
        };
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop());

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert!(request.body.is_none());
        assert_eq!(request.title, "Alert");
    }

    #[tokio::test]
    async fn send_prepared_produces_typed_receipt() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::Linux));
        let provider = DesktopNotificationProvider::with_backend(default_config(), backend.clone());

        let message = Message::text("body").title("hi");
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop());

        let receipt = super::super::Provider::send_prepared(&provider, &dispatch, &prepared)
            .await
            .unwrap();

        assert_eq!(receipt.provider, ProviderKind::Desktop);
        match receipt.message_ref {
            MessageRef::Desktop {
                platform,
                notification_id,
            } => {
                assert_eq!(platform, DesktopPlatform::Linux);
                assert_eq!(notification_id, "fake-id-1");
            }
            _ => panic!("expected MessageRef::Desktop"),
        }
        assert_eq!(receipt.raw_id, "fake-id-1");
        assert_eq!(
            receipt.metadata.get("platform").map(String::as_str),
            Some("Linux")
        );

        let captured = backend.last_request();
        assert_eq!(captured.title, "hi");
    }

    #[tokio::test]
    async fn send_prepared_propagates_backend_metadata() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::MacOS));
        *backend.response.lock().unwrap() = Some(
            DesktopNotificationReceipt::new("uuid-42").with_metadata("delivery", "applescript"),
        );
        let provider = DesktopNotificationProvider::with_backend(default_config(), backend.clone());

        let message = Message::text("body").title("mac-test");
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop());

        let receipt = super::super::Provider::send_prepared(&provider, &dispatch, &prepared)
            .await
            .unwrap();

        assert_eq!(receipt.raw_id, "uuid-42");
        assert_eq!(
            receipt.metadata.get("delivery").map(String::as_str),
            Some("applescript")
        );
        assert_eq!(
            receipt.metadata.get("platform").map(String::as_str),
            Some("macOS")
        );
    }

    #[tokio::test]
    async fn rejects_non_desktop_target() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::Linux));
        let provider = DesktopNotificationProvider::with_backend(default_config(), backend);

        #[cfg(feature = "discord")]
        let dispatch = Dispatch::to(Target::discord_channel("123"));
        #[cfg(not(feature = "discord"))]
        let dispatch = Dispatch::to(Target::desktop());

        let message = Message::text("body").title("wrong target");
        let prepared = PreparedMessage::new(&message);

        let result = super::super::Provider::send_prepared(&provider, &dispatch, &prepared).await;

        #[cfg(feature = "discord")]
        assert!(matches!(result, Err(MessengerError::InvalidMessage(_))));
        #[cfg(not(feature = "discord"))]
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_via_messenger_plans_title_only_message() {
        use crate::provider::Messenger;

        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::Linux));
        let provider = Box::new(DesktopNotificationProvider::with_backend(
            default_config(),
            backend.clone(),
        ));
        let mut messenger = Messenger::new();
        messenger.register(provider);

        let message = Message {
            title: Some("Title Only".into()),
            body: None,
            attachments: Vec::new(),
            location: None,
            metadata: BTreeMap::new(),
        };
        let dispatch = Dispatch::to(Target::desktop());

        let receipt = messenger.send(dispatch, &message).await.unwrap();
        assert_eq!(receipt.provider, ProviderKind::Desktop);
        let captured = backend.last_request();
        assert_eq!(captured.title, "Title Only");
        assert!(captured.body.is_none());
    }

    #[tokio::test]
    async fn replace_produces_typed_receipt() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::Linux));
        let provider = DesktopNotificationProvider::with_backend(default_config(), backend.clone());

        let original_receipt = SendReceipt {
            provider: ProviderKind::Desktop,
            message_ref: MessageRef::Desktop {
                platform: DesktopPlatform::Linux,
                notification_id: "orig-123".into(),
            },
            raw_id: "orig-123".into(),
            metadata: BTreeMap::new(),
        };

        let message = Message::text("updated body").title("Updated");
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop());

        let receipt = provider
            .replace(&original_receipt, &dispatch, &prepared)
            .await
            .unwrap();

        assert_eq!(receipt.provider, ProviderKind::Desktop);
        assert_eq!(receipt.raw_id, "fake-id-replaced");

        let captured = backend.last_request();
        assert_eq!(captured.title, "Updated");
        assert_eq!(captured.body.as_deref(), Some("updated body"));
    }

    #[tokio::test]
    async fn replace_rejects_non_desktop_receipt() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::Linux));
        let provider = DesktopNotificationProvider::with_backend(default_config(), backend);

        let slack_receipt = SendReceipt {
            provider: ProviderKind::Slack,
            message_ref: MessageRef::Slack {
                channel_id: "C123".into(),
                thread_ts: "1234567890".into(),
            },
            raw_id: "1234567890".into(),
            metadata: BTreeMap::new(),
        };

        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);
        let dispatch = Dispatch::to(Target::desktop());

        let result = provider.replace(&slack_receipt, &dispatch, &prepared).await;
        assert!(matches!(result, Err(MessengerError::InvalidMessage(_))));
    }

    #[tokio::test]
    async fn dismiss_succeeds_with_desktop_receipt() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::MacOS));
        let provider = DesktopNotificationProvider::with_backend(default_config(), backend);

        let receipt = SendReceipt {
            provider: ProviderKind::Desktop,
            message_ref: MessageRef::Desktop {
                platform: DesktopPlatform::MacOS,
                notification_id: "notif-42".into(),
            },
            raw_id: "notif-42".into(),
            metadata: BTreeMap::new(),
        };

        provider.dismiss(&receipt).await.unwrap();
    }

    #[tokio::test]
    async fn dismiss_rejects_non_desktop_receipt() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::Linux));
        let provider = DesktopNotificationProvider::with_backend(default_config(), backend);

        let slack_receipt = SendReceipt {
            provider: ProviderKind::Slack,
            message_ref: MessageRef::Slack {
                channel_id: "C123".into(),
                thread_ts: "1234567890".into(),
            },
            raw_id: "1234567890".into(),
            metadata: BTreeMap::new(),
        };

        let result = provider.dismiss(&slack_receipt).await;
        assert!(matches!(result, Err(MessengerError::InvalidMessage(_))));
    }

    #[test]
    fn group_id_override_is_passed_through() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut dispatch = Dispatch::to(Target::desktop());
        dispatch.overrides = ProviderOverrides::Desktop(DesktopOverrides {
            group_id: Some("build-alerts".into()),
            ..Default::default()
        });

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert_eq!(request.group_id.as_deref(), Some("build-alerts"));
    }

    #[test]
    fn replace_id_override_is_passed_through() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut dispatch = Dispatch::to(Target::desktop());
        dispatch.overrides = ProviderOverrides::Desktop(DesktopOverrides {
            replace_id: Some("prev-123".into()),
            ..Default::default()
        });

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert_eq!(request.replace_id.as_deref(), Some("prev-123"));
    }

    #[test]
    fn actions_override_is_passed_through() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut dispatch = Dispatch::to(Target::desktop());
        dispatch.overrides = ProviderOverrides::Desktop(DesktopOverrides {
            actions: vec![
                crate::dispatch::NotificationAction {
                    id: "ok".into(),
                    label: "OK".into(),
                },
                crate::dispatch::NotificationAction {
                    id: "cancel".into(),
                    label: "Cancel".into(),
                },
            ],
            ..Default::default()
        });

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert_eq!(request.actions.len(), 2);
        assert_eq!(request.actions[0].id, "ok");
        assert_eq!(request.actions[0].label, "OK");
        assert_eq!(request.actions[1].id, "cancel");
        assert_eq!(request.actions[1].label, "Cancel");
    }

    #[test]
    fn progress_override_is_passed_through() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut dispatch = Dispatch::to(Target::desktop());
        dispatch.overrides = ProviderOverrides::Desktop(DesktopOverrides {
            progress: Some(crate::dispatch::NotificationProgress {
                current: 42,
                total: 100,
            }),
            ..Default::default()
        });

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert_eq!(
            request.progress,
            Some(crate::dispatch::NotificationProgress {
                current: 42,
                total: 100,
            })
        );
    }

    #[test]
    fn badge_count_override_is_passed_through() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut dispatch = Dispatch::to(Target::desktop());
        dispatch.overrides = ProviderOverrides::Desktop(DesktopOverrides {
            badge_count: Some(7),
            ..Default::default()
        });

        let request = build_request(&default_config(), &dispatch, &prepared).unwrap();
        assert_eq!(request.badge_count, Some(7));
    }

    #[test]
    fn config_actions_used_when_override_empty() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut config = default_config();
        config.actions = vec![crate::dispatch::NotificationAction {
            id: "view".into(),
            label: "View".into(),
        }];

        let dispatch = Dispatch::to(Target::desktop());
        let request = build_request(&config, &dispatch, &prepared).unwrap();
        assert_eq!(request.actions.len(), 1);
        assert_eq!(request.actions[0].id, "view");
    }

    #[test]
    fn config_progress_used_when_override_none() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut config = default_config();
        config.progress = Some(crate::dispatch::NotificationProgress {
            current: 10,
            total: 50,
        });

        let dispatch = Dispatch::to(Target::desktop());
        let request = build_request(&config, &dispatch, &prepared).unwrap();
        assert_eq!(
            request.progress,
            Some(crate::dispatch::NotificationProgress {
                current: 10,
                total: 50,
            })
        );
    }

    #[test]
    fn config_badge_count_used_when_override_none() {
        let message = Message::text("body");
        let prepared = PreparedMessage::new(&message);

        let mut config = default_config();
        config.badge_count = Some(3);

        let dispatch = Dispatch::to(Target::desktop());
        let request = build_request(&config, &dispatch, &prepared).unwrap();
        assert_eq!(request.badge_count, Some(3));
    }
}
