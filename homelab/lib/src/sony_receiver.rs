use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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

    /// Device is powered off (native API requires power).
    #[error("Device is powered off")]
    DeviceOff,
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

/// Zone status from the native web API.
#[derive(Debug, Clone, Serialize)]
pub struct ZoneStatus {
    pub zone: u8,
    pub power: String,
    pub volume: String,
    pub input: String,
}

/// Main zone status from the native web API.
#[derive(Debug, Clone, Serialize)]
pub struct MainZoneStatus {
    pub power: String,
    pub volume: String,
    pub mute: String,
    pub input: String,
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

/// Input configuration from the native web API (port 80).
///
/// This provides the user-defined input names, HDMI port assignments,
/// visibility settings, and sound field presets that the JSON-RPC API
/// doesn't expose. Includes all 18 per-input features.
#[derive(Debug, Clone, Serialize)]
pub struct NativeInputConfig {
    pub category: String,
    pub name: String,
    pub hdmi_assign: String,
    pub icon: String,
    pub visible: bool,
    pub sound_field: String,
    pub digital_assign: String,
    pub input_mode: String,
    pub subwoofer_level: String,
    pub subwoofer_lpf: String,
    pub in_ceiling_mode: bool,
    pub trigger_1: bool,
    pub trigger_2: bool,
    pub trigger_3: bool,
    pub preset_gain: String,
    pub av_sync: String,
}

/// System settings from the native web API (port 80).
///
/// These are system-wide settings not available via JSON-RPC.
/// Returns `null` when a setting is unavailable (e.g., "ERR").
#[derive(Debug, Clone, Serialize)]
pub struct NativeSystemSettings {
    pub volume_display: Option<String>,
    pub dimmer: Option<String>,
    pub device_name: Option<String>,
    pub network: NetworkStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkStatus {
    pub wired: Option<String>,
    pub wireless: Option<String>,
    pub internet: Option<String>,
}

/// Audio settings from the native web API (port 80).
///
/// These are audio settings not available via JSON-RPC.
/// Returns `null` when a setting is unavailable (e.g., "ERR" in Pure Direct mode).
#[derive(Debug, Clone, Serialize)]
pub struct NativeAudioSettings {
    pub pure_direct: Option<String>,
    pub sound_field: Option<String>,
    pub front_balance: Option<String>,
    pub center_level: Option<String>,
    pub subwoofer_level: Option<String>,
    pub dolby_level: Option<String>,
    pub surround_level: Option<String>,
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
/// Extract error code and message from a Sony API error value.
///
/// Sony errors are typically `[code, "message"]` arrays.
/// Returns `(Some(code), message)` if the code is an integer, or `(None, message)`.
fn extract_sony_error(err: &Value) -> (Option<i64>, String) {
    if let Some(arr) = err.as_array() {
        let code = arr.iter().find_map(|v| v.as_i64());
        let msg = arr
            .iter()
            .find_map(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{err}"));
        (code, msg)
    } else {
        (None, format!("{err}"))
    }
}

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
        // Sony receivers only accept one concurrent TCP connection.
        // Disable connection pooling so each request opens and closes
        // its own connection, preventing one client from blocking others.
        let client = Client::builder()
            .pool_max_idle_per_host(0)
            .build()
            .expect("failed to build HTTP client");

        Self {
            host,
            port,
            client,
        }
    }

    /// Send a JSON-RPC command to the receiver and return the response.
    ///
    /// No retry logic — each command is sent exactly once. For state-changing
    /// commands like `setPowerStatus`, retrying can cause the receiver to
    /// toggle state (e.g. off → on → off) leading to unpredictable results.
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
            let (_code, msg) = extract_sony_error(err);
            return Err(SonyError::Api(msg));
        }

        // Normalize: getMethodTypes returns "results" (plural), all others use "result".
        if let Some(results) = json_resp.get("results").cloned()
            && json_resp.get("result").is_none()
        {
            json_resp["result"] = results;
        }

        Ok(json_resp)
    }

    // --- NATIVE WEB API (port 80) ---

    /// Check if the receiver is powered on using the native web API.
    ///
    /// This queries the native API directly without any pre-checks.
    /// Returns true if powered on (value is "on"), false if in standby or unreachable.
    async fn check_power_native(&self) -> Result<bool, SonyError> {
        let url = format!("http://{}:80/fcgi-bin/request.fcgi", self.host);
        let payload = json!({
            "type": "http_get",
            "packet": [["main.power"]]
        });

        match self.client.post(&url).json(&payload).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Ok(false);
                }
                let json_resp: Value = match resp.json().await {
                    Ok(v) => v,
                    Err(_) => return Ok(false),
                };

                if let Some(packets) = json_resp["packet"].as_array() {
                    for packet in packets {
                        if let Some(entries) = packet.as_array() {
                            for entry in entries {
                                if let (Some("main.power"), Some(value)) =
                                    (entry["feature"].as_str(), entry["value"].as_str())
                                {
                                    return Ok(value == "on");
                                }
                            }
                        }
                    }
                }
                Ok(false)
            }
            Err(_) => Ok(false),
        }
    }

    /// Query features from the receiver's native web API (`/fcgi-bin/request.fcgi`).
    ///
    /// This is a separate API from the JSON-RPC API on the configured port.
    /// It runs on port 80 and provides access to settings that the JSON-RPC
    /// API doesn't expose (e.g. correct power status, input configuration).
    ///
    /// Note: The native API typically requires the receiver to be powered on.
    /// This method checks power state first and returns an error if the device is off.
    async fn native_get(&self, features: &[&str]) -> Result<Vec<(String, String)>, SonyError> {
        // Use native API to check power state
        let is_on = self.check_power_native().await?;

        if !is_on {
            return Err(SonyError::DeviceOff);
        }

        let url = format!("http://{}:80/fcgi-bin/request.fcgi", self.host);
        let feature_list: Vec<&str> = features.to_vec();
        let payload = json!({
            "type": "http_get",
            "packet": [feature_list]
        });

        let response = self.client.post(&url).json(&payload).send().await?;
        let json_resp: Value = response.json().await?;

        let mut results = Vec::new();
        if let Some(packets) = json_resp["packet"].as_array() {
            for packet in packets {
                if let Some(entries) = packet.as_array() {
                    for entry in entries {
                        if let (Some(feature), Some(value)) =
                            (entry["feature"].as_str(), entry["value"].as_str())
                        {
                            results.push((feature.to_string(), value.to_string()));
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    /// Set a feature on the receiver's native web API (`/fcgi-bin/request.fcgi`).
    ///
    /// This allows setting values for features that the JSON-RPC API doesn't support
    /// or that require the Native API for reliable updates.
    pub async fn native_set(&self, feature: &str, value: &str) -> Result<(), SonyError> {
        let url = format!("http://{}:80/fcgi-bin/request.fcgi", self.host);
        let payload = json!({
            "type": "http_set",
            "packet": [{
                "feature": feature,
                "value": value
            }]
        });

        self.client.post(&url).json(&payload).send().await?;
        Ok(())
    }

    // --- SYSTEM ENDPOINT ---

    /// Get the power status from the native web API.
    ///
    /// Returns `"active"` when the receiver is on, or `"standby"` when
    /// in standby. Unlike the JSON-RPC `getPowerStatus`, this correctly
    /// reports the state even with Quick Start / Network Standby enabled.
    pub async fn get_power_status(&self) -> Result<String, SonyError> {
        // Use native API directly to check power - this already handles
        // the case where device is off (returns false)
        let is_on = self.check_power_native().await?;
        if is_on {
            Ok("active".to_string())
        } else {
            Ok("standby".to_string())
        }
    }

    /// Queries input configuration from the native web API.
    ///
    /// Returns user-defined names, HDMI assignments, visibility, icons,
    /// sound field presets, and all 18 per-input features for all 8 input categories.
    pub async fn get_native_inputs(&self) -> Result<Vec<NativeInputConfig>, SonyError> {
        const CATEGORIES: &[&str] = &["GAME", "STB", "BD", "SAT", "VIDEO", "AUX", "TV", "CD"];
        const FEATURES: &[&str] = &[
            "inputname", "hdmiassign", "show", "icon", "soundfield",
            "digitalassign", "inputmode", "swlevel", "swlpf",
            "inceilingmode", "usetrigger1", "usetrigger2", "usetrigger3",
            "presetgain", "avsync"
        ];

        let feature_list: Vec<String> = CATEGORIES
            .iter()
            .flat_map(|cat| FEATURES.iter().map(move |feat| format!("{cat}.{feat}")))
            .collect();

        let feature_refs: Vec<&str> = feature_list.iter().map(|s| s.as_str()).collect();
        let results = self.native_get(&feature_refs).await?;

        // Build a lookup map: "GAME.inputname" -> "GAME"
        let mut lookup: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (feature, value) in &results {
            lookup.insert(feature.clone(), value.clone());
        }

        let mut inputs = Vec::new();
        for &cat in CATEGORIES {
            let name = lookup
                .get(&format!("{cat}.inputname"))
                .cloned()
                .unwrap_or_default();
            let hdmi_assign = lookup
                .get(&format!("{cat}.hdmiassign"))
                .cloned()
                .unwrap_or_default();
            let icon = lookup
                .get(&format!("{cat}.icon"))
                .cloned()
                .unwrap_or_default();
            let visible = lookup
                .get(&format!("{cat}.show"))
                .map(|v| v == "true")
                .unwrap_or(false);
            let sound_field = lookup
                .get(&format!("{cat}.soundfield"))
                .cloned()
                .unwrap_or_default();
            let digital_assign = lookup
                .get(&format!("{cat}.digitalassign"))
                .cloned()
                .unwrap_or_default();
            let input_mode = lookup
                .get(&format!("{cat}.inputmode"))
                .cloned()
                .unwrap_or_default();
            let subwoofer_level = lookup
                .get(&format!("{cat}.swlevel"))
                .cloned()
                .unwrap_or_default();
            let subwoofer_lpf = lookup
                .get(&format!("{cat}.swlpf"))
                .cloned()
                .unwrap_or_default();
            let in_ceiling_mode = lookup
                .get(&format!("{cat}.inceilingmode"))
                .map(|v| v == "true")
                .unwrap_or(false);
            let trigger_1 = lookup
                .get(&format!("{cat}.usetrigger1"))
                .map(|v| v == "true")
                .unwrap_or(false);
            let trigger_2 = lookup
                .get(&format!("{cat}.usetrigger2"))
                .map(|v| v == "true")
                .unwrap_or(false);
            let trigger_3 = lookup
                .get(&format!("{cat}.usetrigger3"))
                .map(|v| v == "true")
                .unwrap_or(false);
            let preset_gain = lookup
                .get(&format!("{cat}.presetgain"))
                .cloned()
                .unwrap_or_default();
            let av_sync = lookup
                .get(&format!("{cat}.avsync"))
                .cloned()
                .unwrap_or_default();

            inputs.push(NativeInputConfig {
                category: cat.to_string(),
                name,
                hdmi_assign,
                icon,
                visible,
                sound_field,
                digital_assign,
                input_mode,
                subwoofer_level,
                subwoofer_lpf,
                in_ceiling_mode,
                trigger_1,
                trigger_2,
                trigger_3,
                preset_gain,
                av_sync,
            });
        }

        Ok(inputs)
    }

    /// Queries system settings from the native web API.
    ///
    /// Returns system settings including volume display units, dimmer state,
    /// device name, and network connectivity status.
    pub async fn get_system_settings(&self) -> Result<NativeSystemSettings, SonyError> {
        const FEATURES: &[&str] = &[
            "system.volumedisplay",
            "system.dimmer",
            "system.devicename",
            "system.internetstatus",
            "system.wiredlan",
            "system.wirelesslan",
        ];

        let results = self.native_get(FEATURES).await?;

        let mut lookup: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (feature, value) in &results {
            lookup.insert(feature.clone(), value.clone());
        }

        let to_option = |s: &String| -> Option<String> {
            if s == "ERR" || s.is_empty() { None } else { Some(s.clone()) }
        };

        let volume_display = lookup
            .get("system.volumedisplay")
            .map(to_option)
            .unwrap_or_else(|| Some("dB".to_string()));
        let dimmer = lookup
            .get("system.dimmer")
            .map(to_option)
            .unwrap_or_else(|| Some("off".to_string()));
        let device_name = lookup.get("system.devicename").map(to_option).unwrap_or_default();
        let wired = lookup.get("system.wiredlan").map(to_option).unwrap_or_default();
        let wireless = lookup.get("system.wirelesslan").map(to_option).unwrap_or_default();
        let internet = lookup.get("system.internetstatus").map(to_option).unwrap_or_default();

        Ok(NativeSystemSettings {
            volume_display,
            dimmer,
            device_name,
            network: NetworkStatus {
                wired,
                wireless,
                internet,
            },
        })
    }

    /// Queries zone 2 status from the native web API.
    pub async fn get_zone2_status(&self) -> Result<ZoneStatus, SonyError> {
        let features = &["zone2.power", "zone2.volume", "zone2.input"];
        let results = self.native_get(features).await?;

        let mut lookup: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (feature, value) in &results {
            lookup.insert(feature.clone(), value.clone());
        }

        Ok(ZoneStatus {
            zone: 2,
            power: lookup.get("zone2.power").cloned().unwrap_or_default(),
            volume: lookup.get("zone2.volume").cloned().unwrap_or_default(),
            input: lookup.get("zone2.input").cloned().unwrap_or_default(),
        })
    }

    /// Queries zone 3 status from the native web API.
    pub async fn get_zone3_status(&self) -> Result<ZoneStatus, SonyError> {
        let features = &["zone3.power", "zone3.volume", "zone3.input"];
        let results = self.native_get(features).await?;

        let mut lookup: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (feature, value) in &results {
            lookup.insert(feature.clone(), value.clone());
        }

        Ok(ZoneStatus {
            zone: 3,
            power: lookup.get("zone3.power").cloned().unwrap_or_default(),
            volume: lookup.get("zone3.volume").cloned().unwrap_or_default(),
            input: lookup.get("zone3.input").cloned().unwrap_or_default(),
        })
    }

    /// Queries main zone status from the native web API.
    ///
    /// Returns power, volume, mute, and input for the main zone.
    pub async fn get_main_zone_status(&self) -> Result<MainZoneStatus, SonyError> {
        let features = &["main.power", "main.volume", "main.mute", "main.input"];
        let results = self.native_get(features).await?;

        let mut lookup: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (feature, value) in &results {
            lookup.insert(feature.clone(), value.clone());
        }

        Ok(MainZoneStatus {
            power: lookup.get("main.power").cloned().unwrap_or_default(),
            volume: lookup.get("main.volume").cloned().unwrap_or_default(),
            mute: lookup.get("main.mute").cloned().unwrap_or_default(),
            input: lookup.get("main.input").cloned().unwrap_or_default(),
        })
    }

    /// Queries audio settings from the native web API.
    ///
    /// Returns pure direct mode, sound field, and various speaker level settings.
    pub async fn get_audio_settings(&self) -> Result<NativeAudioSettings, SonyError> {
        const FEATURES: &[&str] = &[
            "audio.puredirect",
            "audio.soundfield",
            "audio.frontbalance",
            "audio.centerlevel",
            "audio.subwooferlevel",
            "audio.dolbylevel",
            "audio.surroundlevel",
        ];

        let results = self.native_get(FEATURES).await?;

        let mut lookup: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (feature, value) in &results {
            lookup.insert(feature.clone(), value.clone());
        }

        let to_option = |s: &String| -> Option<String> {
            if s == "ERR" { None } else { Some(s.clone()) }
        };

        Ok(NativeAudioSettings {
            pure_direct: lookup.get("audio.puredirect").map(to_option).unwrap_or_default(),
            sound_field: lookup.get("audio.soundfield").map(to_option).unwrap_or_default(),
            front_balance: lookup.get("audio.frontbalance").map(to_option).unwrap_or_default(),
            center_level: lookup.get("audio.centerlevel").map(to_option).unwrap_or_default(),
            subwoofer_level: lookup.get("audio.subwooferlevel").map(to_option).unwrap_or_default(),
            dolby_level: lookup.get("audio.dolbylevel").map(to_option).unwrap_or_default(),
            surround_level: lookup.get("audio.surroundlevel").map(to_option).unwrap_or_default(),
        })
    }

/// Set receiver power state.
    ///
    /// Uses `"active"` to power on and `"standby"` to power off. The value
    /// `"off"` is rejected (error 40001) when Quick Start/Network Standby is
    /// enabled — which is the default on most Sony ES receivers.
    pub async fn set_power(&self, active: bool) -> Result<(), SonyError> {
        let status = if active { "active" } else { "standby" };
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
                json!([{ "output": "" }]),
                "1.1",
            )
            .await?;

        let info: Vec<VolumeInfo> = serde_json::from_value(unwrap_sony_result(&response).clone())?;

        if let Some(first) = info.first() {
            Ok(first.mute == "on")
        } else {
            Err(SonyError::NoContent)
        }
    }

    /// Retrieves the current volume information from the receiver.
    ///
    /// Returns volume level, mute status, and min/max volume bounds.
    pub async fn get_volume(&self) -> Result<VolumeInfo, SonyError> {
        let response = self
            .send_command(
                SonyReceiverEndpoints::Audio,
                AudioAction::GetVolumeInformation,
                json!([{ "output": "" }]),
                "1.1",
            )
            .await?;

        let info: Vec<VolumeInfo> = serde_json::from_value(unwrap_sony_result(&response).clone())?;

        info.into_iter().next().ok_or(SonyError::NoContent)
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
        let entry = if result[0].is_object() {
            &result[0]
        } else {
            result
        };
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
        let entry = if result[0].is_object() {
            &result[0]
        } else {
            result
        };
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
        let entry = if result[0].is_object() {
            &result[0]
        } else {
            result
        };
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
        let entry = if result[0].is_object() {
            &result[0]
        } else {
            result
        };
        let status = entry["status"].as_str().unwrap_or("unknown").to_string();
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
    pub async fn get_available_playback_function(&self) -> Result<PlaybackFunction, SonyError> {
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
        let entry = if result[0].is_object() {
            &result[0]
        } else {
            result
        };
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
        let entry = if result[0].is_object() {
            &result[0]
        } else {
            result
        };
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
    pub async fn get_bluetooth_settings(&self, target: &str) -> Result<Value, SonyError> {
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
    pub async fn set_bluetooth_settings(&self, target: &str, value: &str) -> Result<(), SonyError> {
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
