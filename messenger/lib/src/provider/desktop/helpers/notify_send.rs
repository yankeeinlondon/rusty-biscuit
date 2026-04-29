//! `notify-send` helper backend.
//!
//! Wraps the libnotify-provided `notify-send(1)` CLI. Universal Linux
//! default: ships with libnotify and is present on most desktops. Actions
//! require libnotify ≥ 0.7.8; older versions surface only as notice-only.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use async_trait::async_trait;

use crate::dispatch::{NotificationIcon, NotificationUrgency};

use super::super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};
use super::process::run_expect_success;
use super::{HelperBackend, HelperCapabilities, HelperError, HelperName};

/// Score awarded when notify-send works for the dispatch.
const SCORE_DEFAULT: u8 = 60;
/// Score awarded when actions are requested but the libnotify version is
/// known to be too old: notify-send still delivers, but the actions get
/// dropped.
const SCORE_DROPPED_ACTIONS: u8 = 40;
/// Default time we will wait for notify-send to return.
const TIMEOUT_MS: u64 = 5_000;

/// First libnotify version that supports `-A id=Label` action callbacks.
const ACTIONS_MIN_MAJOR: u32 = 0;
const ACTIONS_MIN_MINOR: u32 = 7;
const ACTIONS_MIN_PATCH: u32 = 8;

/// Helper backend that delivers via libnotify's `notify-send`.
pub(crate) struct NotifySendHelper {
    pub(crate) path: PathBuf,
    /// Parsed libnotify version, if it could be probed. `None` means
    /// `--version` failed; treat as old (no actions support).
    pub(crate) version: Option<(u32, u32, u32)>,
}

impl NotifySendHelper {
    /// Build a helper from sniff detection data.
    pub(crate) fn new(path: PathBuf, version: Option<(u32, u32, u32)>) -> Self {
        Self { path, version }
    }

    /// Whether this notify-send is new enough to support `-A id=Label`.
    fn supports_actions(&self) -> bool {
        match self.version {
            Some((major, minor, patch)) => {
                (major, minor, patch) >= (ACTIONS_MIN_MAJOR, ACTIONS_MIN_MINOR, ACTIONS_MIN_PATCH)
            }
            None => false,
        }
    }

    /// Materialize the argv passed to `notify-send`.
    pub(crate) fn build_args(&self, request: &DesktopNotificationRequest) -> Vec<OsString> {
        let mut args: Vec<OsString> = Vec::new();

        args.push(OsString::from("--app-name"));
        args.push(OsString::from(&request.app_name));

        if let Some(icon) = request.icon.as_ref() {
            args.push(OsString::from("--icon"));
            args.push(match icon {
                NotificationIcon::Named(name) => OsString::from(name),
                NotificationIcon::Path(path) => path.clone().into_os_string(),
            });
        } else if let Some(image) = request.image.as_ref() {
            args.push(OsString::from("--icon"));
            args.push(image.clone().into_os_string());
        }

        args.push(OsString::from("--urgency"));
        args.push(OsString::from(urgency_str(request.urgency)));

        if let Some(timeout_ms) = request.timeout_ms {
            args.push(OsString::from("--expire-time"));
            args.push(OsString::from(timeout_ms.to_string()));
        }

        if let Some(category) = request.category.as_deref() {
            args.push(OsString::from("--category"));
            args.push(OsString::from(category));
        }

        if request.silent {
            args.push(OsString::from("--hint"));
            args.push(OsString::from("string:suppress-sound:true"));
        }

        if self.supports_actions() {
            for action in &request.actions {
                args.push(OsString::from("-A"));
                args.push(OsString::from(format!("{}={}", action.id, action.label)));
            }
        }

        if let Some(replace_id) = request.replace_id.as_deref() {
            args.push(OsString::from("--replace-id"));
            args.push(OsString::from(replace_id));
        }

        args.push(OsString::from("-p"));

        args.push(OsString::from(&request.title));
        if let Some(body) = request.body.as_deref() {
            args.push(OsString::from(body));
        }

        args
    }

    /// Parse notify-send stdout into a receipt.
    ///
    /// `-p` prints the assigned id on a single line; older notify-send
    /// versions also tolerate empty output (we error in that case so
    /// callers know the helper produced nothing useful).
    pub(crate) fn parse_output(
        request: &DesktopNotificationRequest,
        stdout: &str,
        actions_dropped: bool,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let id = stdout
            .lines()
            .next()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .ok_or_else(|| HelperError::Parse("notify-send -p returned empty id".into()))?
            .to_string();

        let mut metadata = BTreeMap::new();
        if actions_dropped && !request.actions.is_empty() {
            metadata.insert(
                "dropped".to_string(),
                "actions_libnotify_old".to_string(),
            );
        }

        Ok(DesktopNotificationReceipt {
            notification_id: id,
            metadata,
        })
    }
}

#[async_trait]
impl HelperBackend for NotifySendHelper {
    fn name(&self) -> HelperName {
        HelperName::NotifySend
    }

    fn capabilities(&self) -> HelperCapabilities {
        HelperCapabilities {
            actions: self.supports_actions(),
            reply: false,
            image: true,
            sound: true,
            replace: true,
            group: false,
            blocking: false,
        }
    }

    fn score(&self, request: &DesktopNotificationRequest) -> u8 {
        if request.actions.is_empty() {
            SCORE_DEFAULT
        } else if self.supports_actions() {
            SCORE_DEFAULT
        } else {
            SCORE_DROPPED_ACTIONS
        }
    }

    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let args = self.build_args(request);
        let output = run_expect_success(&self.path, &args, None, Some(TIMEOUT_MS)).await?;
        let actions_dropped = !request.actions.is_empty() && !self.supports_actions();
        Self::parse_output(request, &output.stdout, actions_dropped)
    }

    async fn replace(
        &self,
        id: &str,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let mut request = request.clone();
        request.replace_id = Some(id.to_string());
        let args = self.build_args(&request);
        let output = run_expect_success(&self.path, &args, None, Some(TIMEOUT_MS)).await?;
        let actions_dropped = !request.actions.is_empty() && !self.supports_actions();
        let mut receipt = Self::parse_output(&request, &output.stdout, actions_dropped)?;
        receipt
            .metadata
            .insert("replaced".to_string(), id.to_string());
        Ok(receipt)
    }
}

fn urgency_str(urgency: NotificationUrgency) -> &'static str {
    match urgency {
        NotificationUrgency::Low => "low",
        NotificationUrgency::Normal => "normal",
        NotificationUrgency::Critical => "critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::NotificationAction;

    fn helper_modern() -> NotifySendHelper {
        NotifySendHelper::new(PathBuf::from("/usr/bin/notify-send"), Some((0, 8, 0)))
    }

    fn helper_old() -> NotifySendHelper {
        NotifySendHelper::new(PathBuf::from("/usr/bin/notify-send"), Some((0, 7, 0)))
    }

    fn helper_unknown_version() -> NotifySendHelper {
        NotifySendHelper::new(PathBuf::from("/usr/bin/notify-send"), None)
    }

    fn notice_request() -> DesktopNotificationRequest {
        DesktopNotificationRequest {
            title: "Hi".into(),
            body: Some("Body".into()),
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
        let helper = helper_modern();
        let args = helper.build_args(&notice_request());
        let rendered: Vec<String> =
            args.iter().map(|s| s.to_string_lossy().into_owned()).collect();
        assert_eq!(
            rendered,
            vec![
                "--app-name",
                "Messenger",
                "--urgency",
                "normal",
                "-p",
                "Hi",
                "Body",
            ]
        );
    }

    #[test]
    fn build_args_emits_action_flags_when_modern() {
        let helper = helper_modern();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(rendered.contains(&"-A".to_string()));
        assert!(rendered.contains(&"ok=OK".to_string()));
    }

    #[test]
    fn build_args_drops_actions_when_old_version() {
        let helper = helper_old();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(!rendered.contains(&"-A".to_string()));
    }

    #[test]
    fn build_args_passes_replace_id() {
        let helper = helper_modern();
        let mut request = notice_request();
        request.replace_id = Some("99".into());
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "--replace-id").unwrap();
        assert_eq!(rendered[pos + 1], "99");
    }

    #[test]
    fn score_default_for_notice_only() {
        let helper = helper_modern();
        assert_eq!(helper.score(&notice_request()), SCORE_DEFAULT);
    }

    #[test]
    fn score_default_for_actions_on_modern_libnotify() {
        let helper = helper_modern();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        assert_eq!(helper.score(&request), SCORE_DEFAULT);
    }

    #[test]
    fn score_drops_to_40_for_actions_on_old_libnotify() {
        let helper = helper_old();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        assert_eq!(helper.score(&request), SCORE_DROPPED_ACTIONS);
    }

    #[test]
    fn score_treats_unknown_version_as_old_for_actions() {
        let helper = helper_unknown_version();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        assert_eq!(helper.score(&request), SCORE_DROPPED_ACTIONS);
    }

    #[test]
    fn parse_output_returns_id() {
        let request = notice_request();
        let receipt = NotifySendHelper::parse_output(&request, "42\n", false).unwrap();
        assert_eq!(receipt.notification_id, "42");
        assert!(receipt.metadata.is_empty());
    }

    #[test]
    fn parse_output_marks_dropped_actions() {
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let receipt = NotifySendHelper::parse_output(&request, "42\n", true).unwrap();
        assert_eq!(
            receipt.metadata.get("dropped").map(String::as_str),
            Some("actions_libnotify_old"),
        );
    }

    #[test]
    fn parse_output_errors_on_empty_id() {
        let request = notice_request();
        let result = NotifySendHelper::parse_output(&request, "\n", false);
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }
}
