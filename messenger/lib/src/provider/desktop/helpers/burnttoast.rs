//! `burnttoast` helper backend.
//!
//! Wraps the [BurntToast](https://github.com/Windos/BurntToast) PowerShell
//! module. BurntToast is not a standalone CLI; the helper invokes
//! `pwsh -NoProfile -Command -` and pipes a generated script over stdin.
//! Secondary to [`super::snoretoast::SnoreToastHelper`] when both are
//! available — BurntToast pays a PowerShell startup cost per send and its
//! activation handling depends on script-block bookkeeping that is more
//! brittle than SnoreToast's exit codes.
//!
//! Activation results are captured by writing a tab-separated marker line
//! (`__MESSENGER_ACTIVATION__\t<json>`) from the OnActivated/OnDismissed
//! script blocks. The helper then locates that marker in stdout and parses
//! the JSON payload into receipt metadata.

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::OnceLock;

use async_trait::async_trait;
use serde::Deserialize;

use crate::dispatch::NotificationUrgency;

use super::super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};
use super::process::spawn_helper;
use super::{HelperBackend, HelperCapabilities, HelperError, HelperName};

/// Score awarded when BurntToast is the chosen helper. SnoreToast outranks
/// it whenever both are installed; this helper is the secondary path.
const SCORE_DEFAULT: u8 = 40;
/// Time we will wait for `pwsh` to load BurntToast and submit a notice.
/// Notice-only sends complete quickly once the module is cached.
const NOTICE_TIMEOUT_MS: u64 = 10_000;
/// Marker line emitted from BurntToast event handlers.
const ACTIVATION_MARKER: &str = "__MESSENGER_ACTIVATION__";

/// Helper backend that delivers via the BurntToast PowerShell module.
pub(crate) struct BurntToastHelper {
    pub(crate) pwsh_path: PathBuf,
    pub(crate) app_id: String,
    /// Tracks whether the AppID has been registered with BurntToast in this
    /// process. The registration is cheap once but we still avoid re-running
    /// it per send.
    app_id_registered: OnceLock<()>,
}

/// Activation payload emitted by the BurntToast script's OnActivated /
/// OnDismissed handlers.
#[derive(Debug, Clone, Deserialize)]
struct BurntToastActivation {
    /// One of `action`, `reply`, `dismissed`, `timeout`, `failed`,
    /// `content_clicked`. The script writes these strings directly so we
    /// do not need to translate.
    #[serde(rename = "activationType")]
    activation_type: String,
    /// Action id of the chosen button (only when `activationType == "action"`).
    #[serde(default, rename = "activationKey")]
    activation_key: Option<String>,
    /// Reply text typed by the user (only when `activationType == "reply"`).
    #[serde(default, rename = "replyText")]
    reply_text: Option<String>,
}

impl BurntToastHelper {
    /// Build a helper from sniff detection data and the configured AppID.
    pub(crate) fn new(pwsh_path: PathBuf, app_id: String) -> Self {
        Self {
            pwsh_path,
            app_id,
            app_id_registered: OnceLock::new(),
        }
    }

    /// Materialize the argv used to invoke `pwsh`.
    ///
    /// The script is piped over stdin, so the argv only carries the standard
    /// non-interactive flags. Public-in-crate so unit tests can snapshot the
    /// expected invocation without spawning a process.
    pub(crate) fn build_args(&self) -> Vec<OsString> {
        vec![
            OsString::from("-NoProfile"),
            OsString::from("-NonInteractive"),
            OsString::from("-Command"),
            OsString::from("-"),
        ]
    }

    /// Materialize the PowerShell script piped over stdin.
    ///
    /// The script imports BurntToast, builds a toast content tree from the
    /// request, submits it via `Submit-BTNotification`, and writes the
    /// activation outcome as a `__MESSENGER_ACTIVATION__\t<json>` line.
    /// Public-in-crate so unit tests can assert the script template.
    pub(crate) fn build_script(&self, request: &DesktopNotificationRequest) -> String {
        let mut script = String::new();
        script.push_str("$ErrorActionPreference = 'Stop'\n");
        script.push_str("Import-Module BurntToast\n");

        let title = ps_quote(&request.title);
        let body = request
            .body
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ps_quote)
            .unwrap_or_else(|| "''".to_string());
        script.push_str(&format!("$Texts = @({title}, {body})\n"));

        if !request.actions.is_empty() {
            script.push_str("$Buttons = @(\n");
            for action in &request.actions {
                let label = ps_quote(&action.label);
                let id = ps_quote(&action.id);
                script.push_str(&format!(
                    "    New-BTButton -Content {label} -Arguments {id}\n"
                ));
            }
            script.push_str(")\n");
        }

        if let Some(image) = request.image.as_ref() {
            let path = ps_quote(&image.to_string_lossy());
            script.push_str(&format!(
                "$AppLogo = New-BTImage -Source {path} -AppLogoOverride\n"
            ));
        }

        if let Some(sound) = sound_for_urgency(request.urgency, request.silent) {
            let sound = ps_quote(sound);
            script.push_str(&format!("$Audio = New-BTAudio -Source {sound}\n"));
        } else if request.silent {
            script.push_str("$Audio = New-BTAudio -Silent\n");
        }

        let app_id = ps_quote(&self.app_id);
        script.push_str(&format!("$AppId = {app_id}\n"));

        // Build the toast content tree.
        script.push_str("$Bindings = @(New-BTText -Text $Texts[0])\n");
        script.push_str("if ($Texts[1] -and $Texts[1].Length -gt 0) { $Bindings += New-BTText -Text $Texts[1] }\n");

        let visual_args = match request.image.as_ref() {
            Some(_) => {
                "$Visual = New-BTVisual -BindingGeneric (New-BTBinding -Children $Bindings -AppLogoOverride $AppLogo)\n"
            }
            None => "$Visual = New-BTVisual -BindingGeneric (New-BTBinding -Children $Bindings)\n",
        };
        script.push_str(visual_args);

        if !request.actions.is_empty() {
            script.push_str("$ToastAction = New-BTAction -Buttons $Buttons\n");
            script.push_str("$Content = New-BTContent -Visual $Visual -Actions $ToastAction\n");
        } else {
            script.push_str("$Content = New-BTContent -Visual $Visual\n");
        }

        if let Some(replace_id) = request.replace_id.as_deref() {
            let id = ps_quote(replace_id);
            script.push_str(&format!("$UniqueId = {id}\n"));
        } else {
            script.push_str("$UniqueId = [Guid]::NewGuid().ToString()\n");
        }

        // Submit the notification with activation handlers. The `-Sync`
        // pattern keeps the pwsh process alive long enough for the OS to
        // dispatch the OnActivated callback.
        script.push_str("$ActivatedScript = {\n");
        script.push_str("    param($sender, $eventArgs)\n");
        script.push_str("    $payload = @{ activationType = 'action'; activationKey = $eventArgs.Arguments } | ConvertTo-Json -Compress\n");
        script.push_str(&format!(
            "    Write-Host \"{ACTIVATION_MARKER}`t$payload\"\n"
        ));
        script.push_str("}\n");

        script.push_str("$DismissedScript = {\n");
        script.push_str(
            "    $payload = @{ activationType = 'dismissed' } | ConvertTo-Json -Compress\n",
        );
        script.push_str(&format!(
            "    Write-Host \"{ACTIVATION_MARKER}`t$payload\"\n"
        ));
        script.push_str("}\n");

        script.push_str("$FailedScript = {\n");
        script
            .push_str("    $payload = @{ activationType = 'failed' } | ConvertTo-Json -Compress\n");
        script.push_str(&format!(
            "    Write-Host \"{ACTIVATION_MARKER}`t$payload\"\n"
        ));
        script.push_str("}\n");

        script.push_str(
            "$Submit = @{\n    Content = $Content\n    UniqueIdentifier = $UniqueId\n    AppId = $AppId\n",
        );
        if request.silent || sound_for_urgency(request.urgency, request.silent).is_some() {
            script.push_str("    Audio = $Audio\n");
        }
        script.push_str("    OnActivated = $ActivatedScript\n");
        script.push_str("    OnDismissed = $DismissedScript\n");
        script.push_str("    OnFailed = $FailedScript\n");
        script.push_str("}\n");
        script.push_str("Submit-BTNotification @Submit\n");

        script
    }

    /// Parse stdout for the BurntToast activation marker and return a
    /// receipt. When the marker is absent the receipt records a fallback
    /// `dismissed` activation type — BurntToast occasionally exits without
    /// surfacing the OnDismissed handler when the user dismisses while the
    /// toast is in the action center.
    pub(crate) fn parse_output(
        request: &DesktopNotificationRequest,
        stdout: &str,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        let id = request
            .replace_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut metadata = BTreeMap::new();

        match find_activation_marker(stdout) {
            Some(payload) => {
                let activation: BurntToastActivation =
                    serde_json::from_str(payload).map_err(|error| {
                        HelperError::Parse(format!(
                            "burnttoast activation JSON unparseable: {error}; raw=`{payload}`"
                        ))
                    })?;
                metadata.insert(
                    "activation_type".to_string(),
                    activation.activation_type.clone(),
                );
                if activation.activation_type == "action"
                    && let Some(key) = activation.activation_key.as_deref()
                {
                    metadata.insert("activation_key".to_string(), key.to_string());
                }
                if activation.activation_type == "reply"
                    && let Some(text) = activation.reply_text.as_deref()
                {
                    metadata.insert("reply_text".to_string(), text.to_string());
                }
            }
            None => {
                metadata.insert("activation_type".to_string(), "dismissed".to_string());
            }
        }

        Ok(DesktopNotificationReceipt {
            notification_id: id,
            metadata,
        })
    }

    #[cfg(test)]
    pub(crate) fn mark_app_id_registered(&self) {
        let _ = self.app_id_registered.set(());
    }

    #[cfg(test)]
    pub(crate) fn app_id_registered(&self) -> bool {
        self.app_id_registered.get().is_some()
    }

    /// Build the PowerShell script that registers the AppID idempotently
    /// via `New-BTAppId`. Public-in-crate so unit tests can assert the
    /// registration command shape without spawning a process.
    pub(crate) fn build_register_script(&self) -> String {
        let app_id = ps_quote(&self.app_id);
        format!(
            "$ErrorActionPreference = 'Stop'\nImport-Module BurntToast\nNew-BTAppId -AppId {app_id} -ErrorAction SilentlyContinue\n"
        )
    }

    /// Run `New-BTAppId` once per process to register the configured AppID.
    ///
    /// `New-BTAppId` is idempotent on Windows — it writes to the registry
    /// only when the AppID is missing — but each invocation pays a
    /// PowerShell startup cost. Caching the success in `OnceLock` avoids
    /// re-running the script on every send.
    async fn ensure_app_id_registered(&self) -> Result<(), HelperError> {
        if self.app_id_registered.get().is_some() {
            return Ok(());
        }
        let args = self.build_args();
        let script = self.build_register_script();
        let output = spawn_helper(
            &self.pwsh_path,
            &args,
            Some(&script),
            Some(NOTICE_TIMEOUT_MS),
        )
        .await?;
        if output.status != 0 {
            return Err(HelperError::Exited {
                status: output.status,
                stderr: super::process::first_line(&output.stderr).to_string(),
            });
        }
        let _ = self.app_id_registered.set(());
        Ok(())
    }
}

#[async_trait]
impl HelperBackend for BurntToastHelper {
    fn name(&self) -> HelperName {
        HelperName::BurntToast
    }

    fn capabilities(&self) -> HelperCapabilities {
        HelperCapabilities {
            actions: true,
            reply: false,
            image: true,
            sound: true,
            replace: true,
            group: false,
            blocking: true,
        }
    }

    fn score(&self, _request: &DesktopNotificationRequest) -> u8 {
        SCORE_DEFAULT
    }

    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        self.ensure_app_id_registered().await?;
        let args = self.build_args();
        let script = self.build_script(request);
        let interactive = !request.actions.is_empty();
        let timeout_ms = if interactive {
            None
        } else {
            Some(NOTICE_TIMEOUT_MS)
        };
        let output = spawn_helper(&self.pwsh_path, &args, Some(&script), timeout_ms).await?;
        if output.status != 0 {
            return Err(HelperError::Exited {
                status: output.status,
                stderr: super::process::first_line(&output.stderr).to_string(),
            });
        }
        Self::parse_output(request, &output.stdout)
    }

    async fn replace(
        &self,
        id: &str,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        self.ensure_app_id_registered().await?;
        let mut request = request.clone();
        request.replace_id = Some(id.to_string());
        let args = self.build_args();
        let script = self.build_script(&request);
        let output = spawn_helper(
            &self.pwsh_path,
            &args,
            Some(&script),
            Some(NOTICE_TIMEOUT_MS),
        )
        .await?;
        if output.status != 0 {
            return Err(HelperError::Exited {
                status: output.status,
                stderr: super::process::first_line(&output.stderr).to_string(),
            });
        }
        let mut receipt = Self::parse_output(&request, &output.stdout)?;
        receipt
            .metadata
            .insert("replaced".to_string(), id.to_string());
        Ok(receipt)
    }
}

/// Find the trailing JSON payload after a `__MESSENGER_ACTIVATION__\t`
/// marker anywhere in stdout. Returns the raw JSON slice without the
/// trailing newline, or `None` if no marker is present.
fn find_activation_marker(stdout: &str) -> Option<&str> {
    let prefix = format!("{ACTIVATION_MARKER}\t");
    stdout
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
}

/// Map portable urgency to BurntToast's `New-BTAudio -Source` argument.
///
/// BurntToast accepts the same WinRT system sound URIs as SnoreToast.
fn sound_for_urgency(urgency: NotificationUrgency, silent: bool) -> Option<&'static str> {
    if silent {
        return None;
    }
    match urgency {
        NotificationUrgency::Low => None,
        NotificationUrgency::Normal => Some("ms-winsoundevent:Notification.Default"),
        NotificationUrgency::Critical => Some("ms-winsoundevent:Notification.Looping.Alarm"),
    }
}

/// Quote a string for safe inclusion as a PowerShell single-quoted literal.
///
/// PowerShell single-quoted strings escape an embedded `'` by doubling it.
/// Backticks and `$` are inert inside single quotes, so we only need to
/// double single quotes.
fn ps_quote(value: &str) -> String {
    let escaped = value.replace('\'', "''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::NotificationAction;

    fn helper() -> BurntToastHelper {
        BurntToastHelper::new(
            PathBuf::from("C:/Program Files/PowerShell/7/pwsh.exe"),
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

    #[test]
    fn build_args_returns_expected_pwsh_invocation() {
        let helper = helper();
        let rendered: Vec<String> = helper
            .build_args()
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            rendered,
            vec!["-NoProfile", "-NonInteractive", "-Command", "-"],
        );
    }

    #[test]
    fn build_script_contains_required_pieces_for_notice() {
        let helper = helper();
        let script = helper.build_script(&notice_request());
        assert!(script.contains("Import-Module BurntToast"));
        assert!(script.contains("'Hello'"));
        assert!(script.contains("'World'"));
        assert!(script.contains("'RustyBiscuit.Messenger'"));
        assert!(script.contains("Submit-BTNotification @Submit"));
        assert!(script.contains("OnActivated"));
        assert!(script.contains("OnDismissed"));
        assert!(script.contains("OnFailed"));
        assert!(script.contains("__MESSENGER_ACTIVATION__"));
    }

    #[test]
    fn build_script_emits_buttons_for_actions() {
        let helper = helper();
        let mut request = notice_request();
        request.actions.push(NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        });
        let script = helper.build_script(&request);
        assert!(script.contains("New-BTButton -Content 'OK' -Arguments 'ok'"));
        assert!(script.contains("$ToastAction = New-BTAction -Buttons $Buttons"));
        assert!(script.contains("Actions $ToastAction"));
    }

    #[test]
    fn build_script_emits_app_logo_for_image() {
        let helper = helper();
        let mut request = notice_request();
        request.image = Some(PathBuf::from("C:/tmp/icon.png"));
        let script = helper.build_script(&request);
        assert!(script.contains("New-BTImage -Source 'C:/tmp/icon.png' -AppLogoOverride"));
        assert!(script.contains("AppLogoOverride $AppLogo"));
    }

    #[test]
    fn build_script_silent_emits_silent_audio() {
        let helper = helper();
        let mut request = notice_request();
        request.silent = true;
        let script = helper.build_script(&request);
        assert!(script.contains("New-BTAudio -Silent"));
    }

    #[test]
    fn build_script_critical_emits_alarm_sound() {
        let helper = helper();
        let mut request = notice_request();
        request.urgency = NotificationUrgency::Critical;
        let script = helper.build_script(&request);
        assert!(script.contains("ms-winsoundevent:Notification.Looping.Alarm"));
    }

    #[test]
    fn build_script_low_urgency_omits_audio_block() {
        let helper = helper();
        let mut request = notice_request();
        request.urgency = NotificationUrgency::Low;
        let script = helper.build_script(&request);
        assert!(!script.contains("New-BTAudio"));
    }

    #[test]
    fn build_script_uses_replace_id_when_present() {
        let helper = helper();
        let mut request = notice_request();
        request.replace_id = Some("replace-42".into());
        let script = helper.build_script(&request);
        assert!(script.contains("$UniqueId = 'replace-42'"));
    }

    #[test]
    fn build_script_escapes_single_quotes_in_title() {
        let helper = helper();
        let mut request = notice_request();
        request.title = "It's working".into();
        let script = helper.build_script(&request);
        assert!(script.contains("'It''s working'"));
    }

    #[test]
    fn parse_output_recovers_action() {
        let request = notice_request();
        let stdout = "preamble\n__MESSENGER_ACTIVATION__\t{\"activationType\":\"action\",\"activationKey\":\"ok\"}\n";
        let receipt = BurntToastHelper::parse_output(&request, stdout).unwrap();
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
    fn parse_output_recovers_reply_text() {
        let request = notice_request();
        let stdout =
            "__MESSENGER_ACTIVATION__\t{\"activationType\":\"reply\",\"replyText\":\"hi\"}";
        let receipt = BurntToastHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("reply"),
        );
        assert_eq!(
            receipt.metadata.get("reply_text").map(String::as_str),
            Some("hi"),
        );
    }

    #[test]
    fn parse_output_records_dismissed() {
        let request = notice_request();
        let stdout = "__MESSENGER_ACTIVATION__\t{\"activationType\":\"dismissed\"}";
        let receipt = BurntToastHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[test]
    fn parse_output_falls_back_to_dismissed_when_marker_missing() {
        let request = notice_request();
        let receipt = BurntToastHelper::parse_output(&request, "no marker here\n").unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[test]
    fn parse_output_records_failed_activation() {
        let request = notice_request();
        let stdout = "__MESSENGER_ACTIVATION__\t{\"activationType\":\"failed\"}";
        let receipt = BurntToastHelper::parse_output(&request, stdout).unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("failed"),
        );
    }

    #[test]
    fn parse_output_propagates_invalid_json_marker() {
        let request = notice_request();
        let stdout = "__MESSENGER_ACTIVATION__\tnot-json";
        let result = BurntToastHelper::parse_output(&request, stdout);
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }

    #[test]
    fn parse_output_uses_replace_id_as_notification_id() {
        let mut request = notice_request();
        request.replace_id = Some("99".into());
        let receipt = BurntToastHelper::parse_output(&request, "").unwrap();
        assert_eq!(receipt.notification_id, "99");
    }

    #[test]
    fn parse_output_generates_uuid_when_no_replace_id() {
        let request = notice_request();
        let receipt = BurntToastHelper::parse_output(&request, "").unwrap();
        assert!(uuid::Uuid::parse_str(&receipt.notification_id).is_ok());
    }

    #[test]
    fn score_returns_secondary_default() {
        let helper = helper();
        assert_eq!(helper.score(&notice_request()), SCORE_DEFAULT);
    }

    #[test]
    fn app_id_registration_is_idempotent() {
        let helper = helper();
        assert!(!helper.app_id_registered());
        helper.mark_app_id_registered();
        assert!(helper.app_id_registered());
        helper.mark_app_id_registered();
        assert!(helper.app_id_registered());
    }

    #[test]
    fn build_register_script_invokes_new_bt_app_id() {
        let helper = helper();
        let script = helper.build_register_script();
        assert!(script.contains("Import-Module BurntToast"));
        assert!(
            script.contains(
                "New-BTAppId -AppId 'RustyBiscuit.Messenger' -ErrorAction SilentlyContinue"
            )
        );
    }

    #[test]
    fn ps_quote_escapes_embedded_single_quote() {
        assert_eq!(ps_quote("plain"), "'plain'");
        assert_eq!(ps_quote("It's"), "'It''s'");
        assert_eq!(ps_quote("$inert"), "'$inert'");
    }

    #[test]
    fn find_activation_marker_returns_last_match() {
        let stdout = "__MESSENGER_ACTIVATION__\t{\"activationType\":\"dismissed\"}\n__MESSENGER_ACTIVATION__\t{\"activationType\":\"action\",\"activationKey\":\"ok\"}\n";
        let payload = find_activation_marker(stdout).unwrap();
        assert_eq!(
            payload,
            "{\"activationType\":\"action\",\"activationKey\":\"ok\"}"
        );
    }

    #[test]
    fn find_activation_marker_returns_none_when_absent() {
        assert!(find_activation_marker("nothing here\n").is_none());
    }
}
