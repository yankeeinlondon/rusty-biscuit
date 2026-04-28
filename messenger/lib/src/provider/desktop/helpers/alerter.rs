//! `alerter` helper backend.
//!
//! Wraps the macOS [`alerter`](https://github.com/vjeantet/alerter) CLI,
//! which lets the user click action buttons and supplies an inline reply
//! field. The helper blocks until the user activates the alert, dismisses
//! it, or the alert times out — alerter's killer feature is round-tripping
//! the user's choice as JSON on stdout.
//!
//! Alerter does not provide a notification id we can later replace, so the
//! `replace()` method is unsupported.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;

use crate::dispatch::{NotificationIcon, NotificationUrgency};

use super::super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};
use super::process::run_expect_success;
use super::{HelperBackend, HelperCapabilities, HelperError, HelperName};

/// Score awarded when alerter has actions or a reply slot to drive.
const SCORE_INTERACTIVE: u8 = 90;
/// Score awarded when alerter is asked to deliver a notice-only dispatch.
///
/// alerter blocks the calling task until the user closes the alert, which
/// makes it a poor fit for fire-and-forget notices. Election ranks it well
/// below `terminal-notifier` and `notify-send`-class helpers, but keeps it
/// available as a fallback when nothing else is on the host.
const SCORE_NOTICE_ONLY: u8 = 30;
/// Notice-only timeout. Leaves a generous ceiling so the dialog can be
/// dismissed manually if the user wants to inspect it.
const NOTICE_TIMEOUT_MS: u64 = 60_000;

/// Helper backend that delivers via the macOS `alerter` CLI.
pub(crate) struct AlerterHelper {
    pub(crate) path: PathBuf,
}

/// Activation payload produced by alerter on stdout (`--json` mode).
///
/// Field names match the CLI exactly. Optional fields are absent when the
/// user dismissed without supplying input.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AlerterActivation {
    /// One of `actionClicked`, `closed`, `timeout`, `replied`,
    /// `contentsClicked`. Older alerter builds use `replied` for inline
    /// replies; we accept both spellings.
    #[serde(rename = "activationType")]
    pub activation_type: String,
    /// Action id of the chosen button (only when
    /// `activationType == "actionClicked"`).
    #[serde(rename = "activationValue", default)]
    pub activation_value: Option<String>,
    /// Reply text typed by the user (only when
    /// `activationType == "replied"`).
    #[serde(default, rename = "activationAt")]
    pub activation_at: Option<String>,
    /// Reply payload (alerter writes this under `activationValue` when the
    /// activation type is `replied`).
    #[serde(default, rename = "replyText")]
    pub reply_text: Option<String>,
}

impl AlerterHelper {
    /// Build a helper from sniff detection data.
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Materialize the argv passed to `alerter`.
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

        let body = request
            .body
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&request.title);
        args.push(OsString::from("-message"));
        args.push(OsString::from(body));

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

        if !request.actions.is_empty() {
            args.push(OsString::from("-actions"));
            args.push(OsString::from(format_actions(&request.actions)));
        }

        if request.actions.iter().any(|action| action.id == "reply") {
            args.push(OsString::from("-reply"));
        }

        if let Some(close_label) = request.actions.iter().find(|action| action.id == "close") {
            args.push(OsString::from("-closeLabel"));
            args.push(OsString::from(&close_label.label));
        }

        if let Some(timeout_ms) = request.timeout_ms {
            // alerter uses seconds.
            args.push(OsString::from("-timeout"));
            args.push(OsString::from(((timeout_ms / 1000).max(1)).to_string()));
        }

        args.push(OsString::from("-json"));

        args
    }

    /// Parse alerter JSON output into a receipt.
    pub(crate) fn parse_output(
        request: &DesktopNotificationRequest,
        stdout: &str,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            // alerter emits empty output on `closeLabel` activation in
            // some builds. Surface a `dismissed` receipt without metadata
            // rather than failing the send.
            return Ok(DesktopNotificationReceipt {
                notification_id: uuid::Uuid::new_v4().to_string(),
                metadata: BTreeMap::from([(
                    "activation_type".to_string(),
                    "dismissed".to_string(),
                )]),
            });
        }

        let activation: AlerterActivation = serde_json::from_str(trimmed).map_err(|error| {
            HelperError::Parse(format!("alerter JSON unparseable: {error}; raw=`{trimmed}`"))
        })?;

        let id = request
            .group_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut metadata = BTreeMap::new();
        let activation_type = normalize_activation_type(&activation.activation_type);
        metadata.insert("activation_type".to_string(), activation_type.to_string());

        match activation_type {
            "action" => {
                if let Some(value) = activation.activation_value.as_deref() {
                    metadata.insert("activation_key".to_string(), value.to_string());
                }
            }
            "reply" => {
                let text = activation
                    .reply_text
                    .as_deref()
                    .or(activation.activation_value.as_deref())
                    .or(activation.activation_at.as_deref())
                    .unwrap_or("");
                metadata.insert("reply_text".to_string(), text.to_string());
            }
            _ => {}
        }

        Ok(DesktopNotificationReceipt {
            notification_id: id,
            metadata,
        })
    }
}

#[async_trait]
impl HelperBackend for AlerterHelper {
    fn name(&self) -> HelperName {
        HelperName::Alerter
    }

    fn capabilities(&self) -> HelperCapabilities {
        HelperCapabilities {
            actions: true,
            reply: true,
            image: true,
            sound: true,
            replace: false,
            group: false,
            blocking: true,
        }
    }

    fn score(&self, request: &DesktopNotificationRequest) -> u8 {
        if request.actions.is_empty() {
            SCORE_NOTICE_ONLY
        } else {
            SCORE_INTERACTIVE
        }
    }

    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let args = self.build_args(request);
        let interactive = !request.actions.is_empty();
        let timeout_ms = if interactive { None } else { Some(NOTICE_TIMEOUT_MS) };
        let output = run_expect_success(&self.path, &args, None, timeout_ms).await?;
        Self::parse_output(request, &output.stdout)
    }
}

/// Map portable urgency to alerter's `-sound` argument.
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

/// Format the action list as `id1|Label1,id2|Label2` per alerter's syntax.
fn format_actions(actions: &[crate::dispatch::NotificationAction]) -> String {
    actions
        .iter()
        .map(|action| format!("{}|{}", action.id, action.label))
        .collect::<Vec<_>>()
        .join(",")
}

/// Normalize alerter's `activationType` strings into the values our
/// receipt-metadata vocabulary uses.
fn normalize_activation_type(raw: &str) -> &'static str {
    match raw {
        "actionClicked" => "action",
        "replied" => "reply",
        "timeout" => "timeout",
        "contentsClicked" => "content_clicked",
        "closed" | "" => "dismissed",
        _ => "dismissed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::NotificationAction;

    fn helper() -> AlerterHelper {
        AlerterHelper::new(PathBuf::from("/usr/local/bin/alerter"))
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
        let rendered: Vec<String> = helper
            .build_args(&notice_request())
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            rendered,
            vec![
                "-title", "Hello", "-message", "World", "-sound", "default", "-json",
            ]
        );
    }

    #[test]
    fn build_args_includes_actions_block() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        request.actions.push(NotificationAction {
            id: "cancel".into(),
            label: "Cancel".into(),
        });
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-actions").unwrap();
        assert_eq!(rendered[pos + 1], "ok|OK,cancel|Cancel");
    }

    #[test]
    fn build_args_includes_reply_when_action_id_is_reply() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "reply".into(),
            label: "Reply".into(),
        });
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(rendered.contains(&"-reply".to_string()));
    }

    #[test]
    fn build_args_uses_close_action_as_close_label() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "close".into(),
            label: "Dismiss".into(),
        });
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-closeLabel").unwrap();
        assert_eq!(rendered[pos + 1], "Dismiss");
    }

    #[test]
    fn build_args_critical_uses_basso_sound() {
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
    fn build_args_passes_timeout_in_seconds() {
        let helper = helper();
        let mut request = notice_request();
        request.timeout_ms = Some(15_000);
        let rendered: Vec<String> = helper
            .build_args(&request)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-timeout").unwrap();
        assert_eq!(rendered[pos + 1], "15");
    }

    #[test]
    fn score_notice_only_when_no_actions() {
        let helper = helper();
        assert_eq!(helper.score(&notice_request()), SCORE_NOTICE_ONLY);
    }

    #[test]
    fn score_interactive_when_actions_present() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        assert_eq!(helper.score(&request), SCORE_INTERACTIVE);
    }

    #[test]
    fn parse_output_action_clicked() {
        let request = notice_request();
        let stdout = r#"{"activationType":"actionClicked","activationValue":"ok"}"#;
        let receipt = AlerterHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("ok"),
        );
    }

    #[test]
    fn parse_output_reply_recovers_text() {
        let request = notice_request();
        let stdout =
            r#"{"activationType":"replied","activationValue":"hello there"}"#;
        let receipt = AlerterHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("reply"),
        );
        assert_eq!(
            receipt.metadata.get("reply_text").map(String::as_str),
            Some("hello there"),
        );
    }

    #[test]
    fn parse_output_timeout_records_type() {
        let request = notice_request();
        let stdout = r#"{"activationType":"timeout"}"#;
        let receipt = AlerterHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("timeout"),
        );
    }

    #[test]
    fn parse_output_closed_records_dismissed() {
        let request = notice_request();
        let stdout = r#"{"activationType":"closed"}"#;
        let receipt = AlerterHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[test]
    fn parse_output_contents_clicked_records_type() {
        let request = notice_request();
        let stdout = r#"{"activationType":"contentsClicked"}"#;
        let receipt = AlerterHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("content_clicked"),
        );
    }

    #[test]
    fn parse_output_empty_returns_dismissed() {
        let request = notice_request();
        let receipt = AlerterHelper::parse_output(&request, "").unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[test]
    fn parse_output_invalid_json_propagates_parse_error() {
        let request = notice_request();
        let result = AlerterHelper::parse_output(&request, "not-json");
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }

    #[tokio::test]
    async fn replace_returns_unsupported() {
        let helper = helper();
        let request = notice_request();
        let result = helper.replace("ignored", &request).await;
        assert!(matches!(result, Err(HelperError::Unsupported("replace"))));
    }
}
