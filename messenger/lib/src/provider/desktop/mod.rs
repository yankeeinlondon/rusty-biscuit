//! Desktop notifications provider.
//!
//! Provides a single [`Provider`](crate::provider::Provider) that delivers a
//! [`Message`](crate::message::Message) to the host operating system's
//! notification center. Runtime backend selection (Linux, macOS, Windows)
//! lands in Phase 4; this module wires up the request-construction and
//! provider-send surface so that tests and the CLI can exercise the
//! public provider API with a swappable in-memory backend.

mod backend;
pub mod request;

use std::path::PathBuf;
use std::sync::Arc;

use crate::attachment::{AttachmentKind, AttachmentSource};
use crate::capabilities::CapabilitySet;
use crate::dispatch::{
    Dispatch, NotificationIcon, NotificationUrgency, ProviderOverrides,
};
use crate::error::MessengerError;
use crate::prepared::PreparedMessage;
use crate::receipt::{DesktopPlatform, MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

pub(crate) use backend::DesktopBackend;
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
}

/// macOS-specific desktop configuration.
#[derive(Debug, Clone, Default)]
pub struct MacOsDesktopConfig {
    /// Bundle identifier used when invoking the native framework.
    pub bundle_id: Option<String>,
    /// How the macOS backend should pick between AppleScript and native
    /// `UserNotifications.framework` delivery.
    pub strategy: MacOsNotificationStrategy,
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
    /// ## Notes
    ///
    /// Phase 3 does not yet ship the platform backends. Until Phase 4
    /// lands, this constructor returns a provider whose sends fail with
    /// [`MessengerError::MissingConfiguration`] so callers can register
    /// the provider without panicking. Use
    /// [`DesktopNotificationProvider::with_backend`] for tests and custom
    /// backends.
    pub fn new(config: DesktopConfig) -> Self {
        Self::with_backend(config, Arc::new(UnimplementedBackend))
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

    let urgency = overrides
        .and_then(|o| o.urgency)
        .unwrap_or(config.urgency);

    let timeout_ms = overrides.and_then(|o| o.timeout_ms).or(config.timeout_ms);

    let icon = overrides
        .and_then(|o| o.icon.clone())
        .or_else(|| config.icon.clone());

    let subtitle = overrides.and_then(|o| o.subtitle.clone());
    let replace_id = overrides.and_then(|o| o.replace_id.clone());

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

/// Placeholder backend used until Phase 4 wires platform implementations.
///
/// Returning `MissingConfiguration` rather than panicking keeps
/// [`DesktopNotificationProvider::new`] viable as a library entry point
/// while providing a clear signal to callers that platform support has
/// not yet been selected.
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
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows"
        )))]
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
        assert_eq!(request.image.as_deref(), Some(std::path::Path::new("/tmp/first.png")));
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
        let provider =
            DesktopNotificationProvider::with_backend(default_config(), backend.clone());

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
        assert_eq!(receipt.metadata.get("platform").map(String::as_str), Some("Linux"));

        let captured = backend.last_request();
        assert_eq!(captured.title, "hi");
    }

    #[tokio::test]
    async fn send_prepared_propagates_backend_metadata() {
        let backend = Arc::new(CapturingBackend::with_platform(DesktopPlatform::MacOS));
        *backend.response.lock().unwrap() = Some(
            DesktopNotificationReceipt::new("uuid-42")
                .with_metadata("delivery", "applescript"),
        );
        let provider =
            DesktopNotificationProvider::with_backend(default_config(), backend.clone());

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
}
