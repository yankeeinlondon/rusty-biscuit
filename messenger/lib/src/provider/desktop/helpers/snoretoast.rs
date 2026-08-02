//! `snoretoast` helper backend.
//!
//! Wraps the Windows [`SnoreToast`](https://github.com/KDE/snoretoast) CLI,
//! which delivers WinRT toast notifications from unbundled apps. Default
//! Windows choice with full action button + reply support and replacement
//! via `-id`. Requires an App User Model ID; the backend supplies one at
//! construction time.

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use async_trait::async_trait;

use crate::dispatch::{NotificationAction, NotificationUrgency};

use super::super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};
use super::process::spawn_helper;
use super::{HelperBackend, HelperCapabilities, HelperError, HelperName};

/// Score awarded when snoretoast can serve any dispatch.
const SCORE_DEFAULT: u8 = 90;
/// Maximum image file size accepted by Windows toast notifications.
const MAX_IMAGE_BYTES: u64 = 200 * 1024;
/// Maximum image dimension accepted by Windows toast notifications.
const MAX_IMAGE_DIMENSION: u32 = 1024;
/// Default time we will wait for SnoreToast to return on notice-only sends.
const NOTICE_TIMEOUT_MS: u64 = 5_000;

/// Helper backend that delivers via the Windows `snoretoast` CLI.
pub(crate) struct SnoreToastHelper {
    pub(crate) path: PathBuf,
    pub(crate) app_id: String,
    app_id_registered: OnceLock<()>,
}

impl SnoreToastHelper {
    /// Build a helper from sniff detection data and the configured AppID.
    pub(crate) fn new(path: PathBuf, app_id: String) -> Self {
        Self {
            path,
            app_id,
            app_id_registered: OnceLock::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn mark_app_id_registered(&self) {
        let _ = self.app_id_registered.set(());
    }

    #[cfg(test)]
    pub(crate) fn app_id_registered(&self) -> bool {
        self.app_id_registered.get().is_some()
    }

    /// Materialize the argv SnoreToast expects for AppID registration.
    pub(crate) fn build_register_args(&self) -> Vec<OsString> {
        let shortcut = format!("{}.lnk", self.app_id);
        vec![
            OsString::from("-appID"),
            OsString::from(&self.app_id),
            OsString::from("-install"),
            OsString::from(shortcut),
            self.path.clone().into_os_string(),
            OsString::from(&self.app_id),
        ]
    }

    /// Run SnoreToast's idempotent `-install` registration once per helper.
    async fn ensure_app_id_registered(&self) -> Result<(), HelperError> {
        if self.app_id_registered.get().is_some() {
            return Ok(());
        }
        let args = self.build_register_args();
        let output = spawn_helper(&self.path, &args, None, Some(NOTICE_TIMEOUT_MS)).await?;
        if output.status != 0 {
            return Err(HelperError::Exited {
                status: output.status,
                stderr: super::process::first_line(&output.stderr).to_string(),
            });
        }
        let _ = self.app_id_registered.set(());
        Ok(())
    }

    /// Materialize the argv passed to `snoretoast`.
    ///
    /// The returned `image_dropped` flag indicates whether a configured
    /// image failed pre-validation (oversize, wrong format, missing). The
    /// helper's `send()` uses it to annotate the receipt with
    /// `dropped=image_too_large`.
    pub(crate) fn build_args(&self, request: &DesktopNotificationRequest) -> (Vec<OsString>, bool) {
        let mut args: Vec<OsString> = Vec::new();
        let mut image_dropped = false;

        args.push(OsString::from("-appID"));
        args.push(OsString::from(&self.app_id));

        args.push(OsString::from("-t"));
        args.push(OsString::from(&request.title));

        if let Some(body) = request.body.as_deref() {
            args.push(OsString::from("-m"));
            args.push(OsString::from(body));
        }

        if let Some(image) = request.image.as_ref() {
            if validate_png(image).is_ok() {
                args.push(OsString::from("-p"));
                args.push(image.clone().into_os_string());
            } else {
                image_dropped = true;
            }
        }

        if let Some(sound) = sound_for_urgency(request.urgency, request.silent) {
            args.push(OsString::from("-s"));
            args.push(OsString::from(sound));
        } else if request.silent {
            args.push(OsString::from("-silent"));
        }

        if !request.actions.is_empty() {
            let labels: Vec<&str> = request
                .actions
                .iter()
                .map(|action| action.label.as_str())
                .collect();
            args.push(OsString::from("-b"));
            args.push(OsString::from(labels.join(";")));
        }

        if request.actions.iter().any(|action| action.id == "reply") {
            args.push(OsString::from("-tb"));
        }

        if let Some(replace_id) = request.replace_id.as_deref() {
            args.push(OsString::from("-id"));
            args.push(OsString::from(replace_id));
        }

        (args, image_dropped)
    }

    /// Build a label→id map for the request's actions.
    fn id_by_label(request: &DesktopNotificationRequest) -> HashMap<String, String> {
        request
            .actions
            .iter()
            .map(|action| (action.label.clone(), action.id.clone()))
            .collect()
    }

    /// Parse snoretoast output and exit code into a receipt.
    ///
    /// SnoreToast exit codes (per upstream `Notification::CloseReason`):
    ///
    /// - `0` Activated: notification body or content area clicked
    /// - `1` Hidden: dismissed without user interaction
    /// - `2` Dismissed: explicitly dismissed by the user
    /// - `3` TimedOut
    /// - `4` Failed: SnoreToast reports a hard failure
    /// - `5` Activated with text reply (older builds use stdout capture)
    /// - `-1` (or other): treated as a generic failure
    pub(crate) fn parse_output(
        request: &DesktopNotificationRequest,
        stdout: &str,
        exit_status: i32,
        image_dropped: bool,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        if exit_status == 4 || exit_status < 0 {
            return Err(HelperError::Exited {
                status: exit_status,
                stderr: super::process::first_line(stdout).to_string(),
            });
        }

        let id = request
            .replace_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut metadata = BTreeMap::new();

        if image_dropped {
            metadata.insert("dropped".to_string(), "image_too_large".to_string());
        }

        let label_map = Self::id_by_label(request);
        let trimmed_stdout = stdout.trim();

        if let Some(action_id) = label_map.get(trimmed_stdout) {
            metadata.insert("activation_type".to_string(), "action".to_string());
            metadata.insert("activation_key".to_string(), action_id.clone());
        } else if exit_status == 5 && !trimmed_stdout.is_empty() {
            metadata.insert("activation_type".to_string(), "reply".to_string());
            metadata.insert("reply_text".to_string(), trimmed_stdout.to_string());
        } else {
            let activation_type = match exit_status {
                0 => "content_clicked",
                1 | 2 => "dismissed",
                3 => "timeout",
                _ => "dismissed",
            };
            metadata.insert("activation_type".to_string(), activation_type.to_string());
        }

        Ok(DesktopNotificationReceipt {
            notification_id: id,
            metadata,
        })
    }
}

#[async_trait]
impl HelperBackend for SnoreToastHelper {
    fn name(&self) -> HelperName {
        HelperName::SnoreToast
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

    fn score(&self, request: &DesktopNotificationRequest) -> u8 {
        if has_unmappable_labels(&request.actions) {
            return 0;
        }
        SCORE_DEFAULT
    }

    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        self.ensure_app_id_registered().await?;
        let (args, image_dropped) = self.build_args(request);
        let interactive = !request.actions.is_empty();
        let timeout_ms = if interactive {
            None
        } else {
            Some(NOTICE_TIMEOUT_MS)
        };
        let output = spawn_helper(&self.path, &args, None, timeout_ms).await?;
        Self::parse_output(request, &output.stdout, output.status, image_dropped)
    }

    async fn replace(
        &self,
        id: &str,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        self.ensure_app_id_registered().await?;
        let mut request = request.clone();
        request.replace_id = Some(id.to_string());
        let (args, image_dropped) = self.build_args(&request);
        let output = spawn_helper(&self.path, &args, None, Some(NOTICE_TIMEOUT_MS)).await?;
        let mut receipt =
            Self::parse_output(&request, &output.stdout, output.status, image_dropped)?;
        receipt
            .metadata
            .insert("replaced".to_string(), id.to_string());
        Ok(receipt)
    }
}

/// Map portable urgency to SnoreToast's `-s <sound>` argument.
///
/// SnoreToast accepts the WinRT system sound strings under
/// `ms-winsoundevent:Notification.*`. We pick the looping alarm for critical
/// alerts and suppress sound on low/silent. Returning `None` lets `build_args`
/// emit `-silent` instead when the dispatch is silent.
fn sound_for_urgency(urgency: NotificationUrgency, silent: bool) -> Option<&'static str> {
    if silent {
        return None;
    }
    match urgency {
        NotificationUrgency::Low => None,
        NotificationUrgency::Normal => Some("Notification.Default"),
        NotificationUrgency::Critical => Some("Notification.Looping.Alarm"),
    }
}

/// Detect action labels we cannot reliably round-trip from SnoreToast stdout.
///
/// SnoreToast joins button labels with `;` and prints the chosen label on
/// stdout. Labels that contain `;` or duplicate another label make it
/// impossible to recover the originating action id; the helper scores 0 in
/// those cases so the platform backend can fall through.
fn has_unmappable_labels(actions: &[NotificationAction]) -> bool {
    let mut seen: HashSet<&str> = HashSet::new();
    for action in actions {
        if action.label.contains(';') {
            return true;
        }
        if !seen.insert(action.label.as_str()) {
            return true;
        }
    }
    false
}

/// Validate a PNG file against Windows toast constraints.
///
/// Returns `Ok(())` when the file starts with the PNG signature, has size
/// no greater than 200 KiB, and reports IHDR width/height ≤ 1024 each.
/// Failure variants are simple `&str` tags so the caller can record them
/// without an enum hop.
fn validate_png(path: &Path) -> Result<(), &'static str> {
    let metadata = std::fs::metadata(path).map_err(|_| "missing")?;
    if metadata.len() > MAX_IMAGE_BYTES {
        return Err("image_too_large");
    }

    let bytes = std::fs::read(path).map_err(|_| "io")?;
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("not_png");
    }

    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err("image_too_large");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::NotificationAction;
    use std::io::Write;

    fn helper() -> SnoreToastHelper {
        SnoreToastHelper::new(
            PathBuf::from("C:/Program Files/SnoreToast/snoretoast.exe"),
            "RustyBiscuit.Messenger".to_string(),
        )
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

    /// Build a minimal valid PNG (signature + IHDR for 1×1) to exercise
    /// `validate_png`. Returns `(signature_bytes, ihdr_width, ihdr_height)`
    /// concatenated; the rest of the file (extra chunks) is irrelevant for
    /// our header-only check.
    fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        // 4-byte length (data length = 13 for IHDR), 4-byte type "IHDR"
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        // IHDR: width (4), height (4), bit_depth (1), color_type (1),
        // compression (1), filter (1), interlace (1)
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        // 4-byte CRC placeholder
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn build_args_notice_only_minimal() {
        let helper = helper();
        let (args, image_dropped) = helper.build_args(&notice_request());
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            rendered,
            vec![
                "-appID",
                "RustyBiscuit.Messenger",
                "-t",
                "Hello",
                "-m",
                "World",
                "-s",
                "Notification.Default",
            ]
        );
        assert!(!image_dropped);
    }

    #[test]
    fn build_args_emits_buttons_when_actions_present() {
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
        let (args, _) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-b").unwrap();
        assert_eq!(rendered[pos + 1], "OK;Cancel");
    }

    #[test]
    fn build_args_emits_textbox_when_reply_action_present() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "reply".into(),
            label: "Reply".into(),
        });
        let (args, _) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(rendered.contains(&"-tb".to_string()));
    }

    #[test]
    fn build_args_emits_replace_id_when_present() {
        let helper = helper();
        let mut request = notice_request();
        request.replace_id = Some("99".into());
        let (args, _) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-id").unwrap();
        assert_eq!(rendered[pos + 1], "99");
    }

    #[test]
    fn build_args_silent_emits_silent_flag() {
        let helper = helper();
        let mut request = notice_request();
        request.silent = true;
        let (args, _) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(rendered.contains(&"-silent".to_string()));
        assert!(!rendered.contains(&"-s".to_string()));
    }

    #[test]
    fn build_args_critical_uses_alarm_sound() {
        let helper = helper();
        let mut request = notice_request();
        request.urgency = NotificationUrgency::Critical;
        let (args, _) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let pos = rendered.iter().position(|s| s == "-s").unwrap();
        assert_eq!(rendered[pos + 1], "Notification.Looping.Alarm");
    }

    #[test]
    fn build_args_drops_oversized_image() {
        let helper = helper();
        let mut request = notice_request();

        let temp = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        // Write a PNG header followed by enough padding to exceed 200 KiB.
        let mut bytes = minimal_png(64, 64);
        bytes.resize(MAX_IMAGE_BYTES as usize + 1, 0);
        let mut file = temp.reopen().unwrap();
        file.write_all(&bytes).unwrap();
        file.flush().unwrap();
        request.image = Some(temp.path().to_path_buf());

        let (args, image_dropped) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(image_dropped);
        assert!(!rendered.contains(&"-p".to_string()));
    }

    #[test]
    fn build_args_drops_oversized_dimensions() {
        let helper = helper();
        let mut request = notice_request();

        let temp = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        let bytes = minimal_png(2048, 2048);
        let mut file = temp.reopen().unwrap();
        file.write_all(&bytes).unwrap();
        file.flush().unwrap();
        request.image = Some(temp.path().to_path_buf());

        let (args, image_dropped) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(image_dropped);
        assert!(!rendered.contains(&"-p".to_string()));
    }

    #[test]
    fn build_args_keeps_valid_image() {
        let helper = helper();
        let mut request = notice_request();

        let temp = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        let bytes = minimal_png(64, 64);
        let mut file = temp.reopen().unwrap();
        file.write_all(&bytes).unwrap();
        file.flush().unwrap();
        request.image = Some(temp.path().to_path_buf());

        let (args, image_dropped) = helper.build_args(&request);
        let rendered: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(!image_dropped);
        assert!(rendered.contains(&"-p".to_string()));
    }

    #[test]
    fn score_default_for_notice_request() {
        let helper = helper();
        assert_eq!(helper.score(&notice_request()), SCORE_DEFAULT);
    }

    #[test]
    fn score_default_with_unique_action_labels() {
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
        assert_eq!(helper.score(&request), SCORE_DEFAULT);
    }

    #[test]
    fn score_zero_with_duplicate_action_labels() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "Same".into(),
        });
        request.actions.push(NotificationAction {
            id: "cancel".into(),
            label: "Same".into(),
        });
        assert_eq!(helper.score(&request), 0);
    }

    #[test]
    fn score_zero_with_separator_in_label() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "Yes;No".into(),
        });
        assert_eq!(helper.score(&request), 0);
    }

    #[test]
    fn parse_output_records_content_clicked_for_exit_zero() {
        let request = notice_request();
        let receipt = SnoreToastHelper::parse_output(&request, "", 0, false).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("content_clicked"),
        );
    }

    #[test]
    fn parse_output_records_dismissed_for_exit_one_and_two() {
        let request = notice_request();
        for code in [1, 2] {
            let receipt = SnoreToastHelper::parse_output(&request, "", code, false).unwrap();
            assert_eq!(
                receipt.metadata.get("activation_type").map(String::as_str),
                Some("dismissed"),
            );
        }
    }

    #[test]
    fn parse_output_records_timeout_for_exit_three() {
        let request = notice_request();
        let receipt = SnoreToastHelper::parse_output(&request, "", 3, false).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("timeout"),
        );
    }

    #[test]
    fn parse_output_errors_on_exit_four() {
        let request = notice_request();
        let result = SnoreToastHelper::parse_output(&request, "boom", 4, false);
        assert!(matches!(result, Err(HelperError::Exited { .. })));
    }

    #[test]
    fn parse_output_errors_on_negative_exit() {
        let request = notice_request();
        let result = SnoreToastHelper::parse_output(&request, "boom", -1, false);
        assert!(matches!(result, Err(HelperError::Exited { .. })));
    }

    #[test]
    fn parse_output_records_reply_for_exit_five() {
        let request = notice_request();
        let receipt = SnoreToastHelper::parse_output(&request, "hello there\n", 5, false).unwrap();
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
    fn parse_output_recovers_action_id_via_label() {
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "confirm".into(),
            label: "Confirm".into(),
        });
        let receipt = SnoreToastHelper::parse_output(&request, "Confirm\n", 0, false).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("confirm"),
        );
    }

    #[test]
    fn parse_output_records_image_dropped_metadata() {
        let request = notice_request();
        let receipt = SnoreToastHelper::parse_output(&request, "", 0, true).unwrap();
        assert_eq!(
            receipt.metadata.get("dropped").map(String::as_str),
            Some("image_too_large"),
        );
    }

    #[test]
    fn parse_output_uses_replace_id_as_notification_id() {
        let mut request = notice_request();
        request.replace_id = Some("42".into());
        let receipt = SnoreToastHelper::parse_output(&request, "", 0, false).unwrap();
        assert_eq!(receipt.notification_id, "42");
    }

    #[test]
    fn parse_output_generates_uuid_when_no_replace_id() {
        let request = notice_request();
        let receipt = SnoreToastHelper::parse_output(&request, "", 0, false).unwrap();
        assert!(uuid::Uuid::parse_str(&receipt.notification_id).is_ok());
    }
}
