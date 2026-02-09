use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::network::Host;

// ============================================================================
//                                 ERROR TYPE
// ============================================================================

/// Errors that can occur when communicating with a Sony receiver.
#[derive(Debug, Error)]
pub enum SonyError {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing/serialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Sony API returned an error response.
    #[error("Sony API error: {0}")]
    Api(String),

    /// Receiver returned no content when content was expected.
    #[error("No content returned from receiver")]
    NoContent,

    /// Response format was unexpected or invalid.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

// ============================================================================
//                                 CORE TYPES
// ============================================================================

// --- 1. Endpoint Definitions ---

/// Represents the available API endpoints on a Sony receiver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SonyReceiverEndpoints {
    Audio,
    AvContent,
    System,
    AccessControl,
    AppControl,
    Guide,
    Encryption,
    Browser,
}

impl SonyReceiverEndpoints {
    /// Helper to get the raw path string (e.g., "/sony/system")
    pub fn as_path(&self) -> &'static str {
        match self {
            SonyReceiverEndpoints::Audio => "/sony/audio",
            SonyReceiverEndpoints::AvContent => "/sony/avContent",
            SonyReceiverEndpoints::System => "/sony/system",
            SonyReceiverEndpoints::AccessControl => "/sony/accessControl",
            SonyReceiverEndpoints::AppControl => "/sony/appControl",
            SonyReceiverEndpoints::Guide => "/sony/guide",
            SonyReceiverEndpoints::Encryption => "/sony/encryption",
            SonyReceiverEndpoints::Browser => "/sony/browser",
        }
    }

    /// Returns a list of all endpoints for iteration/discovery
    pub fn all() -> &'static [SonyReceiverEndpoints] {
        &[
            SonyReceiverEndpoints::Audio,
            SonyReceiverEndpoints::AvContent,
            SonyReceiverEndpoints::System,
            SonyReceiverEndpoints::AccessControl,
            SonyReceiverEndpoints::AppControl,
            SonyReceiverEndpoints::Guide,
            SonyReceiverEndpoints::Encryption,
            SonyReceiverEndpoints::Browser,
        ]
    }
}

impl Serialize for SonyReceiverEndpoints {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_path())
    }
}

// ============================================================================
//                               ACTION ENUMS
// ============================================================================

/// Valid commands for `/sony/system`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemAction {
    GetMethodTypes,
    GetVersions,
    GetInterfaceInformation,
    GetSystemInformation,
    GetSystemSupportedFeature,
    GetSupportedApplicationStatusList,
    GetPowerStatus,
    SetPowerStatus,
    GetPowerSettings,
    SetPowerSettings,
    SetSleepTimerSettings,
    GetDeviceMiscSettings,
    SetDeviceMiscSettings,
    GetSettingsTree,
    SetClientInfo,
    #[serde(rename = "getSWUpdateInfo")]
    GetSwUpdateInfo,
    #[serde(rename = "actSWUpdate")]
    ActSwUpdate,
    GetAlexaDeviceInfo,
    GetAlexaRegistrationStatus,
    SetAlexaRegistrationInfo,
    UnregistAlexaDevice,
    GetVoiceCommandGuide,
    ConnectBluetoothDevice,
    GetStorageList,
    GetWuTangInfo,
    SetWuTangInfo,
    GetEciaDeviceInfo,
}

/// Valid commands for `/sony/audio`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioAction {
    GetMethodTypes,
    GetVersions,
    GetVolumeInformation,
    SetAudioVolume,
    SetAudioMute,
    GetSoundSettings,
    SetSoundSettings,
    GetCustomEqualizerSettings,
    SetCustomEqualizerSettings,
    GetSpeakerSettings,
    SetSpeakerSettings,
}

/// Valid commands for `/sony/avContent`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AvContentAction {
    GetMethodTypes,
    GetVersions,
    GetCurrentExternalTerminalsStatus,
    GetSourceList,
    GetSchemeList,
    GetContentCount,
    GetContentList,
    GetPlayingContentInfo,
    GetPlaybackModeSettings,
    GetAvailablePlaybackFunction,
    GetSupportedPlaybackFunction,
    SetPlayContent,
    StopPlayingContent,
    PausePlayingContent,
    SetPlayNextContent,
    SetPlayPreviousContent,
    SetPlaybackModeSettings,
    StartContentBrowsing,
    PresetBroadcastStation,
    SeekBroadcastStation,
    ScanPlayingContent,
    SetActiveTerminal,
    SetBluetoothSettings,
    GetBluetoothSettings,
}

/// Valid commands for `/sony/appControl`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AppControlAction {
    GetMethodTypes,
    GetVersions,
    GetApplicationList,
}

/// Valid commands for `/sony/guide`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GuideAction {
    GetMethodTypes,
    GetVersions,
    GetSupportedApiInfo,
}

// --- structs for System Information ---
#[derive(Debug, Clone, Serialize)]
pub struct SystemInformation {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    pub mac_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wireless_mac_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bd_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<String>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
}

impl SystemInformation {
    fn from_value(value: &Value) -> Self {
        Self {
            model: value_as_string(&value["model"]).unwrap_or_default(),
            // API returns "serialNumber" not "serial"
            serial: value_as_string(&value["serialNumber"])
                .or_else(|| value_as_string(&value["serial"])),
            mac_addr: value_as_string(&value["macAddr"]).unwrap_or_default(),
            wireless_mac_addr: value_as_string(&value["wirelessMacAddr"]),
            bd_addr: value_as_string(&value["bdAddr"]),
            name: value_as_string(&value["name"]),
            generation: value_as_string(&value["generation"]).filter(|s| !s.is_empty()),
            version: value_as_string(&value["version"]).unwrap_or_default(),
            region: value_as_string(&value["region"]),
            product: value_as_string(&value["product"]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwUpdateInfo {
    #[serde(rename = "isUpdatable")]
    pub is_updatable: String, // "true" or "false" string
    #[serde(rename = "swInfo")]
    pub sw_info: Vec<SwInfoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwInfoItem {
    pub version: String,
    #[serde(rename = "releaseDate")]
    pub release_date: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EciaInfo {
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

// --- structs for Generic Settings (WuTang, Speaker, Playback Mode) ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericSettingResult {
    pub target: String,
    #[serde(rename = "currentValue")]
    pub current_value: String,
    pub title: String,
    #[serde(rename = "isAvailable")]
    pub is_available: bool,
    // candidates are optional as some read-only settings don't list them
    pub candidate: Option<Vec<SettingCandidate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingCandidate {
    pub value: String,
    pub title: String,
}

// --- structs for AV Content ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeInfo {
    pub scheme: String, // e.g., "extInput", "storage"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub source: String, // e.g., "extInput:hdmi"
}

/// A browsable content item returned by `getContentList`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub uri: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub index: Option<u32>,
    #[serde(default, rename = "displayNum")]
    pub display_num: Option<String>,
    #[serde(default, rename = "isPlayable")]
    pub is_playable: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackFunction {
    pub output: Option<String>,
    pub functions: Vec<String>, // e.g. ["Play", "Stop", "Pause"]
}

/// Per-URI playback function support returned by `getSupportedPlaybackFunction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedPlaybackFunction {
    pub uri: String,
    pub functions: Vec<SupportedFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedFunction {
    pub function: String,
}

// ============================================================================
//                               DATA STRUCTURES
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerStatusResult {
    pub status: PowerState,
    pub standby_detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerState {
    Active,
    Off,
    Standby,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<String>,
    pub returns: Vec<String>,
    pub version: String,
}

impl<'de> Deserialize<'de> for MethodSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let tuple: (String, Vec<String>, Vec<String>, String) =
            Deserialize::deserialize(deserializer)?;

        Ok(MethodSignature {
            name: tuple.0,
            params: tuple.1,
            returns: tuple.2,
            version: tuple.3,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSource {
    pub title: String,
    pub uri: String,
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<String>,
    pub connection: Option<String>,
    pub label: Option<String>,
    pub active: Option<String>,
}

/// Response from `getPlayingContentInfo`. The Sony API returns a
/// double-nested array with one entry per zone, where fields may be
/// strings or objects depending on the content source.
///
/// ## Notes
///
/// The `state` field reflects the receiver's own transport state, not the
/// connected device's. External inputs (`extInput:*`) always report
/// `STOPPED` because the receiver is passthrough-only — it has no
/// transport control over the source device. Sources the receiver controls
/// directly (e.g. `radio:fm`, `storage:usb1`, `dlna:music`) will report
/// `PLAYING`, `PAUSED`, etc.
#[derive(Debug, Clone, Serialize)]
pub struct PlayingContentInfo {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_kind: Option<String>,
}

impl PlayingContentInfo {
    fn from_value(value: &Value) -> Self {
        Self {
            uri: value_as_string(&value["uri"]).unwrap_or_default(),
            source: value_as_string(&value["source"]),
            title: value_as_string(&value["title"]),
            state: value["stateInfo"]["state"].as_str().map(String::from),
            content_kind: value_as_string(&value["contentKind"]),
        }
    }
}

/// Extract a string from a JSON value that may be a string, object, or null.
fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

/// Unwrap the Sony JSON-RPC `result` field.
///
/// Sony responses use three nesting patterns:
/// - `result: [[{...}, ...]]` — double-nested (e.g. `getPlayingContentInfo` with zones)
/// - `result: [{...}]` — single-nested (e.g. `getSystemInformation`)
/// - `result: [["name", [...], ...], ...]` — flat array of tuples (e.g. `getMethodTypes`)
///
/// This helper unwraps one level when `result[0]` is an array whose first
/// element is an object (double-nested data). Otherwise returns `result` as-is.
fn unwrap_sony_result(response: &Value) -> &Value {
    let result = &response["result"];
    let first = &result[0];
    // Double-nested: result[0] is an array and its first element is an object
    if first.is_array() && first[0].is_object() {
        first
    } else {
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub volume: u32,
    pub mute: String,
    #[serde(rename = "minVolume")]
    pub min_volume: u32,
    #[serde(rename = "maxVolume")]
    pub max_volume: u32,
}

/// Result of probing a single Sony API endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
    pub path: String,
    pub active: bool,
    pub detail: String,
}

#[derive(Serialize)]
struct SonyRequest<M: Serialize> {
    method: M,
    id: u32,
    params: Value,
    version: String,
}

// ============================================================================
//                              CLIENT IMPLEMENTATION
// ============================================================================

/// The **SonyReceiver** struct is intended to provide TCP/IP automation
/// with modern Sony receivers which support the REST / JSON-RPC API style.
pub struct SonyReceiver {
    host: Host,
    port: u16,
    client: Client,
}

impl SonyReceiver {
    pub fn new(host: Host, port: u16) -> Self {
        Self {
            host,
            port,
            client: Client::new(),
        }
    }

    /// Generic async sender using typed Endpoints and generic Methods
    async fn send_command<M: Serialize>(
        &self,
        endpoint: SonyReceiverEndpoints,
        method: M,
        params: Value,
        version: &str,
    ) -> Result<Value, SonyError> {
        let url = format!("http://{}:{}{}", self.host, self.port, endpoint.as_path());

        let payload = SonyRequest {
            method,
            id: 1,
            params,
            version: version.to_string(),
        };

        let response = self.client.post(&url).json(&payload).send().await?;
        let mut json_resp: Value = response.json().await?;

        if let Some(err) = json_resp.get("error") {
            // Sony errors are typically [code, "message"]
            let msg = if let Some(arr) = err.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .next()
                    .unwrap_or_else(|| format!("{err}"))
            } else {
                format!("{err}")
            };
            return Err(SonyError::Api(msg));
        }

        // Normalize: getMethodTypes returns "results" (plural), all others use "result".
        if let Some(results) = json_resp.get("results").cloned() {
            if json_resp.get("result").is_none() {
                json_resp["result"] = results;
            }
        }

        Ok(json_resp)
    }

    // --- SYSTEM ENDPOINT ---
    pub async fn get_power_status(&self) -> Result<String, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::System,
                SystemAction::GetPowerStatus,
                json!([]),
                "1.1",
            )
            .await?;

        let result = unwrap_sony_result(&response);
        let entry = if result[0].is_object() { &result[0] } else { result };
        let status = entry["status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(status)
    }

    pub async fn set_power(&self, active: bool) -> Result<(), SonyError> {
        let status = if active { "active" } else { "off" };
        let params = json!([{ "status": status }]);

        self.send_command(
            SonyReceiverEndpoints::System,
            SystemAction::SetPowerStatus,
            params,
            "1.1",
        )
        .await?;
        Ok(())
    }

    // --- AUDIO ENDPOINT ---
    pub async fn set_volume(&self, level: u32) -> Result<(), SonyError> {
        let params = json!([{
            "target": "speaker",
            "volume": level.to_string()
        }]);

        self.send_command(
            SonyReceiverEndpoints::Audio,
            AudioAction::SetAudioVolume,
            params,
            "1.1",
        )
        .await?;
        Ok(())
    }

    pub async fn get_mute_status(&self) -> Result<bool, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::Audio,
                AudioAction::GetVolumeInformation,
                json!([]),
                "1.1",
            )
            .await?;

        let info: Vec<VolumeInfo> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;

        if let Some(first) = info.first() {
            Ok(first.mute == "on")
        } else {
            Err(SonyError::NoContent)
        }
    }

    pub async fn set_mute(&self, mute: bool) -> Result<(), SonyError> {
        let status = if mute { "on" } else { "off" };
        let params = json!([{ "status": status }]);

        self.send_command(
            SonyReceiverEndpoints::Audio,
            AudioAction::SetAudioMute,
            params,
            "1.1",
        )
        .await?;
        Ok(())
    }

    // --- AV CONTENT ENDPOINT ---
    pub async fn list_inputs(&self) -> Result<Vec<InputSource>, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetCurrentExternalTerminalsStatus,
                json!([]),
                "1.0",
            )
            .await?;

        let inputs: Vec<InputSource> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;
        Ok(inputs)
    }

    pub async fn get_current_input(&self) -> Result<PlayingContentInfo, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetPlayingContentInfo,
                json!([{ "output": "" }]),
                "1.2",
            )
            .await?;

        let zones = unwrap_sony_result(&response);
        if zones.as_array().map(|v| v.is_empty()).unwrap_or(true) {
            return Err(SonyError::NoContent);
        }

        Ok(PlayingContentInfo::from_value(&zones[0]))
    }

    pub async fn set_input(&self, uri: &str) -> Result<(), SonyError> {
        let params = json!([{ "uri": uri }]);
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::SetPlayContent,
            params,
            "1.2",
        )
        .await?;
        Ok(())
    }

    // --- INTROSPECTION ---
    pub async fn get_supported_methods(
        &self,
        endpoint: SonyReceiverEndpoints,
    ) -> Result<Vec<MethodSignature>, SonyError> {
        // "getMethodTypes" is common to all endpoints, so we use a string here or a specific Enum variant.
        // Since we don't know which Action Enum maps to the generic `endpoint` argument,
        // we can cheat and use a raw string for this specific meta-call.
        // OR we can map it if we really want strict typing, but "getMethodTypes" is universal.

        let response = self
            .send_command(endpoint, "getMethodTypes", json!([""]), "1.0")
            .await?;

        let results = unwrap_sony_result(&response);
        let results_array = results
            .as_array()
            .ok_or_else(|| SonyError::InvalidResponse("expected array".to_string()))?;

        let mut signatures = Vec::new();
        for item in results_array {
            let sig: MethodSignature = serde_json::from_value(item.clone())?;
            signatures.push(sig);
        }

        Ok(signatures)
    }
    /// Returns detailed system info (Model, Serial, Mac Address, Firmware Version)
    pub async fn get_system_information(&self) -> Result<SystemInformation, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::System,
                SystemAction::GetSystemInformation,
                json!([]),
                "1.4", // Requires 1.4+
            )
            .await?;

        let result = unwrap_sony_result(&response);
        let entry = if result[0].is_object() { &result[0] } else { result };
        Ok(SystemInformation::from_value(entry))
    }

    /// Checks if a firmware update is available.
    pub async fn get_sw_update_info(&self) -> Result<SwUpdateInfo, SonyError> {
        // "network" is the standard param for checking online updates
        let params = json!([{ "network": "network" }]);

        let response = self
            .send_command(
                SonyReceiverEndpoints::System,
                SystemAction::GetSwUpdateInfo,
                params,
                "1.0",
            )
            .await?;

        let result = unwrap_sony_result(&response);
        let entry = if result[0].is_object() { &result[0] } else { result };
        let info: SwUpdateInfo = serde_json::from_value(entry.clone())?;
        Ok(info)
    }

    /// TRIGGERS a software update.
    /// WARNING: This will likely reboot the receiver.
    pub async fn act_sw_update(&self) -> Result<(), SonyError> {
        self.send_command(
            SonyReceiverEndpoints::System,
            SystemAction::ActSwUpdate,
            json!([]),
            "1.0",
        )
        .await?;
        Ok(())
    }

    /// Gets internal provisioning info ("WuTang").
    /// `target` usually requires specific internal strings.
    pub async fn get_wu_tang_info(
        &self,
        target: &str,
    ) -> Result<Vec<GenericSettingResult>, SonyError> {
        let params = json!([{ "target": target }]);
        let response = self
            .send_command(
                SonyReceiverEndpoints::System,
                SystemAction::GetWuTangInfo,
                params,
                "1.0",
            )
            .await?;

        let results: Vec<GenericSettingResult> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;
        Ok(results)
    }

    pub async fn get_ecia_device_info(&self) -> Result<EciaInfo, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::System,
                SystemAction::GetEciaDeviceInfo,
                json!([]),
                "1.0",
            )
            .await?;

        let result = unwrap_sony_result(&response);
        let entry = if result[0].is_object() { &result[0] } else { result };
        let info: EciaInfo = serde_json::from_value(entry.clone())?;
        Ok(info)
    }

    /// Note: `getAlexaRegistrationInfo` does not exist in standard dumps.
    /// Mapped to `getAlexaRegistrationStatus` which provides the enrollment status.
    pub async fn get_alexa_registration_status(&self) -> Result<String, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::System,
                SystemAction::GetAlexaRegistrationStatus,
                json!([]),
                "1.0",
            )
            .await?;

        let result = unwrap_sony_result(&response);
        let entry = if result[0].is_object() { &result[0] } else { result };
        let status = entry["status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        Ok(status)
    }

    // --- AUDIO ENDPOINT -----------------------------------------------------

    /// Gets configuration for speakers.
    /// Pass a target like `"level"`, `"distance"`, `"size"`, `"pattern"`,
    /// or an empty string to attempt querying all settings.
    pub async fn get_speaker_settings(
        &self,
        target: &str,
    ) -> Result<Vec<GenericSettingResult>, SonyError> {
        let params = json!([{ "target": target }]);

        let response = self
            .send_command(
                SonyReceiverEndpoints::Audio,
                AudioAction::GetSpeakerSettings,
                params,
                "1.0",
            )
            .await?;

        let settings: Vec<GenericSettingResult> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;
        Ok(settings)
    }

    // --- AV CONTENT ENDPOINT ------------------------------------------------

    /// Returns the list of valid URI schemes (e.g. "extInput", "storage")
    pub async fn get_scheme_list(&self) -> Result<Vec<String>, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetSchemeList,
                json!([]),
                "1.0",
            )
            .await?;

        let raw_list: Vec<SchemeInfo> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;

        // Flatten to simple string vector
        Ok(raw_list.into_iter().map(|s| s.scheme).collect())
    }

    /// Checks what playback functions (Play, Pause, Stop) are available
    /// for the current input.
    pub async fn get_available_playback_function(
        &self,
    ) -> Result<PlaybackFunction, SonyError> {
        // "output" is optional, usually defaults to main zone if omitted
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetAvailablePlaybackFunction,
                json!([{ "output": "" }]),
                "1.0",
            )
            .await?;

        let result = unwrap_sony_result(&response);
        let entry = if result[0].is_object() { &result[0] } else { result };
        let func: PlaybackFunction = serde_json::from_value(entry.clone())?;
        Ok(func)
    }

    pub async fn get_playback_mode_settings(
        &self,
        target: &str,
    ) -> Result<Vec<GenericSettingResult>, SonyError> {
        let params = json!([{ "target": target }]);
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetPlaybackModeSettings,
                params,
                "1.0",
            )
            .await?;

        let settings: Vec<GenericSettingResult> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;
        Ok(settings)
    }

    /// Returns available sources for a given URI scheme (e.g. "extInput").
    pub async fn get_source_list(&self, scheme: &str) -> Result<Vec<SourceInfo>, SonyError> {
        let params = json!([{ "scheme": scheme }]);
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetSourceList,
                params,
                "1.0",
            )
            .await?;

        let sources: Vec<SourceInfo> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;
        Ok(sources)
    }

    /// Returns the number of content items for a given source.
    pub async fn get_content_count(&self, source: &str) -> Result<u32, SonyError> {
        let params = json!([{ "source": source }]);
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetContentCount,
                params,
                "1.0",
            )
            .await?;

        let result = unwrap_sony_result(&response);
        let entry = if result[0].is_object() { &result[0] } else { result };
        let count = entry["count"].as_u64().unwrap_or(0) as u32;
        Ok(count)
    }

    /// Returns a page of content items for a given source.
    pub async fn get_content_list(
        &self,
        source: &str,
        start: u32,
        count: u32,
    ) -> Result<Vec<ContentItem>, SonyError> {
        let params = json!([{ "source": source, "stIdx": start, "cnt": count }]);
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetContentList,
                params,
                "1.2",
            )
            .await?;

        let items: Vec<ContentItem> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;
        Ok(items)
    }

    /// Initiates content browsing for a given source URI.
    pub async fn start_content_browsing(&self, source: &str) -> Result<(), SonyError> {
        let params = json!([{ "source": source }]);
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::StartContentBrowsing,
            params,
            "1.0",
        )
        .await?;
        Ok(())
    }

    /// Sets the active output terminal (e.g. zone).
    pub async fn set_active_terminal(&self, uri: &str) -> Result<(), SonyError> {
        let params = json!([{ "uri": uri }]);
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::SetActiveTerminal,
            params,
            "1.0",
        )
        .await?;
        Ok(())
    }

    /// Gets Bluetooth settings from the avContent endpoint.
    pub async fn get_bluetooth_settings(
        &self,
        target: &str,
    ) -> Result<Value, SonyError> {
        let params = json!([{ "target": target }]);
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetBluetoothSettings,
                params,
                "1.0",
            )
            .await?;

        Ok(unwrap_sony_result(&response).clone())
    }

    /// Sets a Bluetooth setting on the avContent endpoint.
    pub async fn set_bluetooth_settings(
        &self,
        target: &str,
        value: &str,
    ) -> Result<(), SonyError> {
        let params = json!([{ "settings": [{ "target": target, "value": value }] }]);
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::SetBluetoothSettings,
            params,
            "1.0",
        )
        .await?;
        Ok(())
    }

    // --- PLAYBACK CONTROLS ---

    pub async fn set_play_next_content(&self) -> Result<(), SonyError> {
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::SetPlayNextContent,
            json!([{}]), // Often requires an empty object inside the array
            "1.0",
        )
        .await?;
        Ok(())
    }

    pub async fn stop_playing_content(&self) -> Result<(), SonyError> {
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::StopPlayingContent,
            json!([{}]),
            "1.1",
        )
        .await?;
        Ok(())
    }

    pub async fn pause_playing_content(&self) -> Result<(), SonyError> {
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::PausePlayingContent,
            json!([{}]),
            "1.1",
        )
        .await?;
        Ok(())
    }

    pub async fn set_play_previous_content(&self) -> Result<(), SonyError> {
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::SetPlayPreviousContent,
            json!([{}]),
            "1.0",
        )
        .await?;
        Ok(())
    }

    /// Returns per-URI playback function support.
    pub async fn get_supported_playback_function(
        &self,
    ) -> Result<Vec<SupportedPlaybackFunction>, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetSupportedPlaybackFunction,
                json!([{ "output": "" }]),
                "1.0",
            )
            .await?;

        let items: Vec<SupportedPlaybackFunction> =
            serde_json::from_value(unwrap_sony_result(&response).clone())?;
        Ok(items)
    }

    /// Presets (saves) a broadcast station by URI.
    pub async fn preset_broadcast_station(&self, uri: &str) -> Result<(), SonyError> {
        let params = json!([{ "uri": uri }]);
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::PresetBroadcastStation,
            params,
            "1.0",
        )
        .await?;
        Ok(())
    }

    /// Seeks to the next or previous broadcast station.
    pub async fn seek_broadcast_station(&self, forward: bool) -> Result<(), SonyError> {
        let direction = if forward { "fwd" } else { "bwd" };
        let params = json!([{ "direction": direction }]);
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::SeekBroadcastStation,
            params,
            "1.0",
        )
        .await?;
        Ok(())
    }

    /// Scans playing content forward or backward.
    pub async fn scan_playing_content(&self, forward: bool) -> Result<(), SonyError> {
        let direction = if forward { "fwd" } else { "bwd" };
        let params = json!([{ "direction": direction, "output": "" }]);
        self.send_command(
            SonyReceiverEndpoints::AvContent,
            AvContentAction::ScanPlayingContent,
            params,
            "1.0",
        )
        .await?;
        Ok(())
    }

    pub async fn get_playing_content_info(&self) -> Result<PlayingContentInfo, SonyError> {
        // We must specify which output we are querying.
        // An empty string "" usually targets the current/main zone.
        let params = json!([{ "output": "" }]);

        let response = self
            .send_command(
                SonyReceiverEndpoints::AvContent,
                AvContentAction::GetPlayingContentInfo,
                params,
                "1.2",
            )
            .await?;

        let zones = unwrap_sony_result(&response);
        if zones.as_array().map(|v| v.is_empty()).unwrap_or(true) {
            return Err(SonyError::NoContent);
        }

        Ok(PlayingContentInfo::from_value(&zones[0]))
    }

    // --- DISCOVERY ---

    /// Returns the host string used by this receiver.
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// Returns the port used by this receiver.
    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn probe_endpoints(&self) -> Result<Vec<ProbeResult>, SonyError> {
        let mut results = Vec::new();

        for endpoint in SonyReceiverEndpoints::all() {
            let path = endpoint.as_path();
            let url = format!("http://{}:{}{}", self.host, self.port, path);

            let payload = json!({
                "method": "getMethodTypes",
                "id": 1,
                "params": [""],
                "version": "1.0"
            });

            let client = Client::builder()
                .timeout(std::time::Duration::from_millis(500))
                .build()?;

            let resp = client.post(&url).json(&payload).send().await;

            let (active, detail) = match resp {
                Ok(response) => {
                    if response.status().is_success() {
                        (true, "Active".to_string())
                    } else {
                        (false, format!("HTTP {}", response.status()))
                    }
                }
                Err(_) => (false, "Unreachable".to_string()),
            };

            results.push(ProbeResult {
                path: path.to_string(),
                active,
                detail,
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_action_serializes_correctly() {
        // Standard camelCase variants
        assert_eq!(
            serde_json::to_string(&SystemAction::GetPowerStatus).unwrap(),
            "\"getPowerStatus\""
        );
        assert_eq!(
            serde_json::to_string(&SystemAction::SetPowerStatus).unwrap(),
            "\"setPowerStatus\""
        );

        // Acronym variants with explicit renames - critical for Sony API compatibility
        assert_eq!(
            serde_json::to_string(&SystemAction::GetSwUpdateInfo).unwrap(),
            "\"getSWUpdateInfo\""
        );
        assert_eq!(
            serde_json::to_string(&SystemAction::ActSwUpdate).unwrap(),
            "\"actSWUpdate\""
        );

        // WuTang and Ecia (not acronyms, camelCase works)
        assert_eq!(
            serde_json::to_string(&SystemAction::GetWuTangInfo).unwrap(),
            "\"getWuTangInfo\""
        );
        assert_eq!(
            serde_json::to_string(&SystemAction::GetEciaDeviceInfo).unwrap(),
            "\"getEciaDeviceInfo\""
        );
        assert_eq!(
            serde_json::to_string(&SystemAction::GetAlexaRegistrationStatus).unwrap(),
            "\"getAlexaRegistrationStatus\""
        );
    }

    #[test]
    fn audio_action_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&AudioAction::GetVolumeInformation).unwrap(),
            "\"getVolumeInformation\""
        );
        assert_eq!(
            serde_json::to_string(&AudioAction::SetAudioMute).unwrap(),
            "\"setAudioMute\""
        );
    }

    #[test]
    fn av_content_action_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&AvContentAction::GetPlayingContentInfo).unwrap(),
            "\"getPlayingContentInfo\""
        );
        assert_eq!(
            serde_json::to_string(&AvContentAction::SetPlayContent).unwrap(),
            "\"setPlayContent\""
        );
    }

    #[test]
    fn endpoints_serialize_to_paths() {
        assert_eq!(
            serde_json::to_string(&SonyReceiverEndpoints::System).unwrap(),
            "\"/sony/system\""
        );
        assert_eq!(
            serde_json::to_string(&SonyReceiverEndpoints::Audio).unwrap(),
            "\"/sony/audio\""
        );
        assert_eq!(
            serde_json::to_string(&SonyReceiverEndpoints::AvContent).unwrap(),
            "\"/sony/avContent\""
        );
    }
}
