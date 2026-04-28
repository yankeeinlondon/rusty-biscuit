//! `terminal-notifier` helper backend.
//!
//! Wraps the macOS [`terminal-notifier`](https://github.com/julienXX/terminal-notifier)
//! CLI. Notice-only delivery: terminal-notifier does not support interactive
//! action buttons or inline replies, so the helper scores zero whenever the
//! dispatch carries actions. Replacement is supported via `-remove`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use async_trait::async_trait;

use crate::dispatch::{NotificationIcon, NotificationUrgency};

use super::super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};
use super::process::run_expect_success;
use super::{HelperBackend, HelperCapabilities, HelperError, HelperName};

/// Score awarded when terminal-notifier serves a notice-only dispatch.
const SCORE_NOTICE_ONLY: u8 = 80;
/// Time we will wait for terminal-notifier to return on notice-only sends.
const NOTICE_TIMEOUT_MS: u64 = 5_000;

/// Helper backend that delivers via the macOS `terminal-notifier` CLI.
pub(crate) struct TerminalNotifierHelper {
    pub(crate) path: PathBuf,
}

impl TerminalNotifierHelper {
    /// Build a helper from sniff detection data.
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Materialize the argv passed to `terminal-notifier`.
    ///
    /// Public-in-crate so unit tests can snapshot the exact argv without
    /// spawning a process.
    pub(crate) fn build_args(&self, request: &DesktopNotificationRequest) -> Vec<OsString> {
        let mut args: Vec<OsString> = Vec::new();

        args.push(OsString::from("-title"));
        args.push(OsString::from(&request.title));

        if let Some(subtitle) = request.subtitle.as_deref() {
            args.push(OsString::from("-subtitle"));
            args.push(OsString::from(subtitle));
        }

        if let Some(body) = request.body.as_deref() {
            args.push(OsString::from("-message"));
            args.push(OsString::from(body));
        } else {
            // terminal-notifier requires `-message`; fall back to title.
            args.push(OsString::from("-message"));
            args.push(OsString::from(&request.title));
        }

        if let Some(icon) = request.icon.as_ref()
            && let NotificationIcon::Path(path) = icon
        {
            args.push(OsString::from("-contentImage"));
            args.push(path.clone().into_os_string());
        } else if let Some(image) = request.image.as_ref() {
            args.push(OsString::from("-contentImage"));
            args.push(image.clone().into_os_string());
        }

        if let Some(sound) = sound_for_urgency(request.urgency, request.silent) {
            args.push(OsString::from("-sound"));
            args.push(OsString::from(sound));
        }

        if let Some(group) = request.group_id.as_deref() {
            args.push(OsString::from("-group"));
            args.push(OsString::from(group));
        }

        if let Some(replace_id) = request.replace_id.as_deref() {
            args.push(OsString::from("-remove"));
            args.push(OsString::from(replace_id));
        }

        if request.urgency == NotificationUrgency::Critical {
            args.push(OsString::from("-ignoreDnD"));
        }

        args
    }

    /// Parse terminal-notifier output into a receipt.
    ///
    /// terminal-notifier writes nothing useful to stdout; we generate a
    /// UUID-style receipt id from the group (when set) or a fresh UUID.
    pub(crate) fn parse_output(
        request: &DesktopNotificationRequest,
        _stdout: &str,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let id = request
            .group_id
            .clone()
            .or_else(|| request.replace_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut metadata = BTreeMap::new();
        if request.silent && matches!(request.urgency, NotificationUrgency::Low) {
            metadata.insert("dropped".to_string(), "sound_low_urgency".to_string());
        }

        Ok(DesktopNotificationReceipt {
            notification_id: id,
            metadata,
        })
    }
}

#[async_trait]
impl HelperBackend for TerminalNotifierHelper {
    fn name(&self) -> HelperName {
        HelperName::TerminalNotifier
    }

    fn capabilities(&self) -> HelperCapabilities {
        HelperCapabilities {
            actions: false,
            reply: false,
            image: true,
            sound: true,
            replace: true,
            group: true,
            blocking: false,
        }
    }

    fn score(&self, request: &DesktopNotificationRequest) -> u8 {
        if !request.actions.is_empty() {
            return 0;
        }
        SCORE_NOTICE_ONLY
    }

    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let args = self.build_args(request);
        let output = run_expect_success(&self.path, &args, None, Some(NOTICE_TIMEOUT_MS)).await?;
        Self::parse_output(request, &output.stdout)
    }

    async fn replace(
        &self,
        id: &str,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let mut request = request.clone();
        // terminal-notifier replaces by `-group`; reuse the supplied id as
        // the group so subsequent posts to the same group overwrite this one.
        request.group_id = Some(id.to_string());
        let args = self.build_args(&request);
        let output = run_expect_success(&self.path, &args, None, Some(NOTICE_TIMEOUT_MS)).await?;
        let mut receipt = Self::parse_output(&request, &output.stdout)?;
        receipt
            .metadata
            .insert("replaced".to_string(), id.to_string());
        Ok(receipt)
    }
}

/// Map portable urgency to terminal-notifier's `-sound` argument.
///
/// terminal-notifier exposes the macOS system sound names. We pick `Basso`
/// for critical alerts (the loudest stock sound), default for normal, and
/// suppress sound entirely on low urgency or when the dispatch is silent.
fn sound_for_urgency(urgency: NotificationUrgency, silent: bool) -> Option<&'static str> {
    if silent {
        return None;
    }
    match urgency {
        NotificationUrgency::Low => None,
        NotificationUrgency::Normal => Some("default"),
        NotificationUrgency::Critical => Some("Basso"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::NotificationAction;

    fn helper() -> TerminalNotifierHelper {
        TerminalNotifierHelper::new(PathBuf::from("/usr/local/bin/terminal-notifier"))
    }

    fn notice_request() -> DesktopNotificationRequest {
        DesktopNotificationRequest {
            title: "Hello".into(),
            body: Some("World".into()),
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
    fn build_args_notice_only_minimal() {
        let helper = helper();
        let args = helper.build_args(&notice_request());
        let rendered: Vec<String> =
            args.iter().map(|s| s.to_string_lossy().into_owned()).collect();
        assert_eq!(
            rendered,
            vec![
                "-title", "Hello", "-message", "World", "-sound", "default",
            ]
        );
    }

    #[test]
    fn build_args_includes_subtitle_when_present() {
        let helper = helper();
        let mut request = notice_request();
        request.subtitle = Some("Sub".into());
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-subtitle").unwrap();
        assert_eq!(rendered[pos + 1], "Sub");
    }

    #[test]
    fn build_args_falls_back_to_title_when_body_missing() {
        let helper = helper();
        let mut request = notice_request();
        request.body = None;
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-message").unwrap();
        assert_eq!(rendered[pos + 1], "Hello");
    }

    #[test]
    fn build_args_passes_image_via_content_image() {
        let helper = helper();
        let mut request = notice_request();
        request.image = Some(PathBuf::from("/tmp/icon.png"));
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-contentImage").unwrap();
        assert_eq!(rendered[pos + 1], "/tmp/icon.png");
    }

    #[test]
    fn build_args_critical_uses_basso_and_ignores_dnd() {
        let helper = helper();
        let mut request = notice_request();
        request.urgency = NotificationUrgency::Critical;
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-sound").unwrap();
        assert_eq!(rendered[pos + 1], "Basso");
        assert!(rendered.contains(&"-ignoreDnD".to_string()));
    }

    #[test]
    fn build_args_low_urgency_omits_sound() {
        let helper = helper();
        let mut request = notice_request();
        request.urgency = NotificationUrgency::Low;
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(!rendered.contains(&"-sound".to_string()));
    }

    #[test]
    fn build_args_silent_omits_sound() {
        let helper = helper();
        let mut request = notice_request();
        request.silent = true;
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(!rendered.contains(&"-sound".to_string()));
    }

    #[test]
    fn build_args_passes_group_and_replace_id() {
        let helper = helper();
        let mut request = notice_request();
        request.group_id = Some("build".into());
        request.replace_id = Some("42".into());
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let group_pos = rendered.iter().position(|s| s == "-group").unwrap();
        assert_eq!(rendered[group_pos + 1], "build");
        let remove_pos = rendered.iter().position(|s| s == "-remove").unwrap();
        assert_eq!(rendered[remove_pos + 1], "42");
    }

    #[test]
    fn score_zero_when_actions_present() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        assert_eq!(helper.score(&request), 0);
    }

    #[test]
    fn score_notice_only_for_basic_dispatch() {
        let helper = helper();
        assert_eq!(helper.score(&notice_request()), SCORE_NOTICE_ONLY);
    }

    #[test]
    fn parse_output_uses_group_as_id_when_set() {
        let mut request = notice_request();
        request.group_id = Some("build".into());
        let receipt = TerminalNotifierHelper::parse_output(&request, "").unwrap();
        assert_eq!(receipt.notification_id, "build");
    }

    #[test]
    fn parse_output_generates_uuid_when_no_group() {
        let request = notice_request();
        let receipt = TerminalNotifierHelper::parse_output(&request, "").unwrap();
        assert!(!receipt.notification_id.is_empty());
        assert!(uuid::Uuid::parse_str(&receipt.notification_id).is_ok());
    }
}
