//! `dunstify` helper backend.
//!
//! Wraps the dunst project's `dunstify(1)` CLI. Only elected when the active
//! D-Bus notification daemon is dunst — `--wait` and `-A` round-trips do not
//! work end-to-end against other servers.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use async_trait::async_trait;

use crate::dispatch::{NotificationIcon, NotificationUrgency};

use super::super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};
use super::process::{run_expect_success, spawn_helper};
use super::{HelperBackend, HelperCapabilities, HelperError, HelperName};

/// Score awarded when dunstify is the active daemon and dispatch is interactive.
const SCORE_INTERACTIVE: u8 = 90;
/// Score awarded when dunstify is the active daemon and dispatch is notice-only.
const SCORE_NOTICE_ONLY: u8 = 70;
/// Default time we will wait for dunstify to return for notice-only sends.
const NOTICE_TIMEOUT_MS: u64 = 3_000;

/// Helper backend that delivers via the dunst-specific `dunstify` CLI.
pub(crate) struct DunstifyHelper {
    pub(crate) path: PathBuf,
    /// `true` when sniff reports the active D-Bus notification daemon as
    /// dunst. `--wait` / `-A` round-trips need dunst as the bus owner.
    pub(crate) daemon_is_dunst: bool,
}

impl DunstifyHelper {
    /// Build a helper from sniff detection data.
    pub(crate) fn new(path: PathBuf, daemon_is_dunst: bool) -> Self {
        Self {
            path,
            daemon_is_dunst,
        }
    }

    /// Materialize the argv passed to `dunstify`.
    ///
    /// Public-in-crate so unit tests can snapshot the exact argv without
    /// spawning a process.
    pub(crate) fn build_args(&self, request: &DesktopNotificationRequest) -> Vec<OsString> {
        let mut args: Vec<OsString> = Vec::new();

        args.push(OsString::from("--appname"));
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
            args.push(OsString::from("--timeout"));
            args.push(OsString::from(timeout_ms.to_string()));
        }

        if request.silent {
            args.push(OsString::from("--hint"));
            args.push(OsString::from("string:suppress-sound:true"));
        }

        if let Some(group) = request.group_id.as_deref() {
            args.push(OsString::from("--hint"));
            args.push(OsString::from(format!("string:stack-tag:{group}")));
        }

        for action in &request.actions {
            args.push(OsString::from("--action"));
            args.push(OsString::from(format!("{},{}", action.id, action.label)));
        }

        if let Some(replace_id) = request.replace_id.as_deref() {
            args.push(OsString::from("--replace"));
            args.push(OsString::from(replace_id));
        }

        args.push(OsString::from("--printid"));

        let interactive = !request.actions.is_empty();
        if interactive {
            args.push(OsString::from("--wait"));
        }

        args.push(OsString::from(&request.title));
        if let Some(body) = request.body.as_deref() {
            args.push(OsString::from(body));
        }

        args
    }

    /// Parse dunstify stdout into a receipt.
    ///
    /// Format (under `--printid` and optionally `--wait`):
    ///
    /// - line 1: numeric notification id
    /// - line 2 (only when `--wait` was passed and the user activated): the
    ///   chosen action key
    pub(crate) fn parse_output(
        request: &DesktopNotificationRequest,
        stdout: &str,
        exit_status: i32,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let mut lines = stdout.lines();
        let id_line = lines
            .next()
            .ok_or_else(|| HelperError::Parse("dunstify produced no output".into()))?
            .trim()
            .to_string();

        if id_line.is_empty() {
            return Err(HelperError::Parse(
                "dunstify --printid returned empty id".into(),
            ));
        }

        let mut metadata = BTreeMap::new();
        let action_key = lines.next().map(str::trim).filter(|line| !line.is_empty());

        let interactive = !request.actions.is_empty();
        if interactive {
            metadata.insert(
                "close_reason".to_string(),
                close_reason_for_status(exit_status).to_string(),
            );
            if let Some(key) = action_key {
                metadata.insert("activation_type".to_string(), "action".to_string());
                metadata.insert("activation_key".to_string(), key.to_string());
            } else {
                metadata.insert(
                    "activation_type".to_string(),
                    activation_type_for_status(exit_status).to_string(),
                );
            }
        }

        Ok(DesktopNotificationReceipt {
            notification_id: id_line,
            metadata,
        })
    }
}

#[async_trait]
impl HelperBackend for DunstifyHelper {
    fn name(&self) -> HelperName {
        HelperName::Dunstify
    }

    fn capabilities(&self) -> HelperCapabilities {
        HelperCapabilities {
            actions: true,
            reply: false,
            image: true,
            sound: true,
            replace: true,
            group: true,
            blocking: true,
        }
    }

    fn score(&self, request: &DesktopNotificationRequest) -> u8 {
        if !self.daemon_is_dunst {
            return 0;
        }
        if !request.actions.is_empty() {
            SCORE_INTERACTIVE
        } else {
            SCORE_NOTICE_ONLY
        }
    }

    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let args = self.build_args(request);
        let interactive = !request.actions.is_empty();
        let timeout_ms = if interactive {
            None
        } else {
            Some(NOTICE_TIMEOUT_MS)
        };

        let output = if interactive {
            spawn_helper(&self.path, &args, None, timeout_ms).await?
        } else {
            run_expect_success(&self.path, &args, None, timeout_ms).await?
        };
        Self::parse_output(request, &output.stdout, output.status)
    }

    async fn replace(
        &self,
        id: &str,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let mut request = request.clone();
        request.replace_id = Some(id.to_string());
        let args = self.build_args(&request);
        let output = run_expect_success(&self.path, &args, None, Some(NOTICE_TIMEOUT_MS)).await?;
        let mut receipt = Self::parse_output(&request, &output.stdout, output.status)?;
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

fn activation_type_for_status(status: i32) -> &'static str {
    match status {
        2 => "timeout",
        3 => "dismissed",
        _ => "dismissed",
    }
}

fn close_reason_for_status(status: i32) -> &'static str {
    match status {
        1 => "dismissed",
        2 => "timeout",
        3 => "closed_by_call",
        _ => "undefined",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::NotificationAction;

    fn helper(daemon_is_dunst: bool) -> DunstifyHelper {
        DunstifyHelper::new(PathBuf::from("/usr/bin/dunstify"), daemon_is_dunst)
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
        let helper = helper(true);
        let request = notice_request();
        let args = helper.build_args(&request);
        let rendered: Vec<String> =
            args.iter().map(|s| s.to_string_lossy().into_owned()).collect();
        assert_eq!(
            rendered,
            vec![
                "--appname",
                "Messenger",
                "--urgency",
                "normal",
                "--printid",
                "Hello",
                "World",
            ]
        );
    }

    #[test]
    fn build_args_includes_actions_and_wait_when_interactive() {
        let helper = helper(true);
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "Confirm".into(),
        });
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(rendered.contains(&"--action".to_string()));
        assert!(rendered.contains(&"ok,Confirm".to_string()));
        assert!(rendered.contains(&"--wait".to_string()));
    }

    #[test]
    fn build_args_passes_replace_id_and_group() {
        let helper = helper(true);
        let mut request = notice_request();
        request.replace_id = Some("42".into());
        request.group_id = Some("build".into());
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(rendered.contains(&"--replace".to_string()));
        assert!(rendered.contains(&"42".to_string()));
        assert!(rendered.contains(&"string:stack-tag:build".to_string()));
    }

    #[test]
    fn build_args_passes_silent_hint() {
        let helper = helper(true);
        let mut request = notice_request();
        request.silent = true;
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(rendered.contains(&"string:suppress-sound:true".to_string()));
    }

    #[test]
    fn build_args_uses_critical_urgency() {
        let helper = helper(true);
        let mut request = notice_request();
        request.urgency = NotificationUrgency::Critical;
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let urgency_pos = rendered.iter().position(|s| s == "--urgency").unwrap();
        assert_eq!(rendered[urgency_pos + 1], "critical");
    }

    #[test]
    fn score_zero_when_daemon_is_not_dunst() {
        let helper = helper(false);
        assert_eq!(helper.score(&notice_request()), 0);
    }

    #[test]
    fn score_zero_for_interactive_when_daemon_is_not_dunst() {
        // Daemon-mismatch edge case: even an interactive dispatch (which
        // would normally win the highest dunstify score) must score 0 when
        // the bus owner is not dunst, so election skips this helper and
        // falls through to notify-send / native.
        let helper = helper(false);
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        assert_eq!(helper.score(&request), 0);
    }

    #[test]
    fn score_70_for_notice_when_dunst() {
        let helper = helper(true);
        assert_eq!(helper.score(&notice_request()), SCORE_NOTICE_ONLY);
    }

    #[test]
    fn score_90_for_interactive_when_dunst() {
        let helper = helper(true);
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        assert_eq!(helper.score(&request), SCORE_INTERACTIVE);
    }

    #[test]
    fn parse_output_returns_id_for_notice_only() {
        let request = notice_request();
        let receipt = DunstifyHelper::parse_output(&request, "12345\n", 0).unwrap();
        assert_eq!(receipt.notification_id, "12345");
        assert!(receipt.metadata.is_empty());
    }

    #[test]
    fn parse_output_records_action_for_interactive() {
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let receipt = DunstifyHelper::parse_output(&request, "9\nok\n", 0).unwrap();
        assert_eq!(receipt.notification_id, "9");
        assert_eq!(receipt.metadata.get("activation_type").unwrap(), "action");
        assert_eq!(receipt.metadata.get("activation_key").unwrap(), "ok");
    }

    #[test]
    fn parse_output_records_dismissal_when_no_action_key() {
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let receipt = DunstifyHelper::parse_output(&request, "9\n", 1).unwrap();
        assert_eq!(receipt.notification_id, "9");
        assert_eq!(
            receipt.metadata.get("close_reason").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[test]
    fn parse_output_errors_on_empty_id() {
        let request = notice_request();
        let result = DunstifyHelper::parse_output(&request, "\n", 0);
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }
}
