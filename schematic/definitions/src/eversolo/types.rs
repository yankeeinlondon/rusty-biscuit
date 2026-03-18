//! Eversolo DMP-A8 API response types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// =============================================================================
// Shared
// =============================================================================

/// Generic status response returned by most endpoints.
///
/// A `status` of 200 indicates success.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct StatusResponse {
    pub status: i32,
}

// =============================================================================
// Device
// =============================================================================

/// Response from the `getModel` endpoint with device identity information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetModelResponse {
    pub status: i32,
    pub model: String,
    pub ip: String,
    /// Wired MAC address.
    ///
    /// The API uses snake_case `net_mac` rather than camelCase for this field.
    #[serde(rename = "net_mac")]
    pub net_mac: String,
    /// Wi-Fi MAC address.
    ///
    /// The API uses `wif_mac` (missing the 'i' — a known Zidoo-lineage typo).
    #[serde(rename = "wif_mac", alias = "wifi_mac")]
    pub wifi_mac: String,
    pub firmware: String,
    /// Android base version reported by firmware.
    ///
    /// This field may be absent on older firmware builds. The API uses
    /// all-lowercase `androidversion` rather than camelCase.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "androidversion")]
    pub android_version: Option<String>,
    /// Whether the device supports remote boot/wake behavior.
    ///
    /// This field may be absent on firmware versions that do not expose it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub able_remote_boot: Option<bool>,
    /// Whether EQ settings are available on this hardware/firmware combination.
    ///
    /// This field may be absent on models or firmware without EQ support metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_eq_setting: Option<bool>,
}

// =============================================================================
// Music / State
// =============================================================================

/// Current playback state from `getState`.
///
/// ## Notes
///
/// The payload includes owned `String` fields under `playing_music`. Polling this
/// endpoint at high frequency can create steady allocation churn in UI loops.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetStateResponse {
    /// Status code (200 = success). Absent from some firmware versions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,
    /// Playback state enum value.
    ///
    /// Common values observed in Zidoo-derived firmware:
    /// - `0` = stopped
    /// - `1` = playing
    /// - `2` = paused
    ///
    /// Additional values may appear on newer firmware revisions.
    pub state: i32,
    /// Current position in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    /// Total duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playing_music: Option<PlayingMusic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_data: Option<VolumeData>,
}

/// Metadata for the currently playing track.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlayingMusic {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_number: Option<i32>,
}

/// Volume and mute state.
///
/// ## Notes
///
/// The API spells the current volume field as `currenttVolume` (double-t typo).
/// This is intentional and matches the actual device response.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeData {
    /// Current volume level (note: API spells this `currenttVolume` with double-t).
    #[serde(rename = "currenttVolume")]
    pub current_volume: i32,
    pub max_volume: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_volume: Option<i32>,
    #[serde(default)]
    pub is_mute: bool,
    /// Displayed dB string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_db: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_volume_enable: Option<bool>,
}

// =============================================================================
// I/O
// =============================================================================

/// Input and output device listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputOutputListResponse {
    pub status: i32,
    pub input_data: Vec<InputItem>,
    pub output_data: Vec<OutputItem>,
    /// Unstructured output info that varies by model/output type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_info: Option<serde_json::Value>,
    /// Index into `input_data` of the currently selected input source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_index: Option<i32>,
    /// Index into `output_data` of the currently selected output port.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_index: Option<i32>,
}

/// An available audio input.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputItem {
    pub name: String,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorted_index: Option<i32>,
}

/// An available audio output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OutputItem {
    pub name: String,
    pub tag: String,
    #[serde(default)]
    pub enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorted_index: Option<i32>,
}

// =============================================================================
// Power
// =============================================================================

/// Available power options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PowerOptionsResponse {
    pub status: i32,
    pub data: Vec<PowerOption>,
}

/// A power action option (e.g., poweroff, reboot, screen, timeshutdown).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PowerOption {
    /// Localized display name.
    pub name: String,
    /// Action tag (e.g., "poweroff", "reboot").
    pub tag: String,
}

// =============================================================================
// System / Display
// =============================================================================

/// Brightness level response.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrightnessResponse {
    /// Status code (200 = success). May be absent on some firmware versions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,
    /// Current brightness level, typically in the range `0..=max`.
    ///
    /// The API returns this as `currentValue`.
    pub current_value: i32,
    /// Maximum brightness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i32>,
}

/// List of available display modes (VU or spectrum).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisplayModeListResponse {
    pub status: i32,
    pub data: Vec<DisplayMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_index: Option<i32>,
}

/// A display mode entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode {
    /// Display name (the API uses `title`, not `name`).
    pub title: String,
    /// Mode tag identifier used for selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Mode index accepted by `setVUMode` / `setSpPlayModeList`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_response_deserialization() {
        let json = r#"{"status": 200}"#;
        let resp: StatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn get_model_response_deserialization() {
        let json = r#"{
            "status": 200,
            "model": "DMP-A8",
            "ip": "192.168.1.50",
            "net_mac": "AA:BB:CC:DD:EE:FF",
            "wif_mac": "11:22:33:44:55:66",
            "firmware": "2.3.20"
        }"#;
        let resp: GetModelResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.model, "DMP-A8");
        assert_eq!(resp.ip, "192.168.1.50");
        assert_eq!(resp.net_mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(resp.wifi_mac, "11:22:33:44:55:66");
        assert!(resp.android_version.is_none());
    }

    #[test]
    fn volume_data_double_t_typo() {
        let json = r#"{
            "currenttVolume": 42,
            "maxVolume": 100,
            "isMute": false
        }"#;
        let data: VolumeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.current_volume, 42);
        assert_eq!(data.max_volume, 100);
        assert!(!data.is_mute);

        // Verify serialization preserves the typo
        let serialized = serde_json::to_string(&data).unwrap();
        assert!(serialized.contains("currenttVolume"));
    }

    #[test]
    fn get_state_response_deserialization() {
        let json = r#"{
            "status": 200,
            "state": 1,
            "position": 30000,
            "duration": 240000,
            "playingMusic": {
                "title": "Test Song",
                "artist": "Test Artist"
            },
            "volumeData": {
                "currenttVolume": 50,
                "maxVolume": 100,
                "isMute": false
            }
        }"#;
        let resp: GetStateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, Some(200));
        assert_eq!(resp.state, 1);
        assert_eq!(resp.position, Some(30000));
        let music = resp.playing_music.unwrap();
        assert_eq!(music.title, Some("Test Song".to_string()));
        let vol = resp.volume_data.unwrap();
        assert_eq!(vol.current_volume, 50);
    }

    #[test]
    fn get_state_response_without_status_field() {
        let json = r#"{
            "state": 0,
            "volumeData": {
                "currenttVolume": 160,
                "maxVolume": 200,
                "isMute": false
            }
        }"#;
        let resp: GetStateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, None);
        assert_eq!(resp.state, 0);
        let vol = resp.volume_data.unwrap();
        assert_eq!(vol.current_volume, 160);
    }

    #[test]
    fn input_output_list_response_deserialization() {
        let json = r#"{
            "status": 200,
            "inputData": [
                {"name": "USB", "tag": "usb", "sortedIndex": 0}
            ],
            "outputData": [
                {"name": "XLRRCA", "tag": "xlrrca", "enable": true, "sortedIndex": 0}
            ]
        }"#;
        let resp: InputOutputListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.input_data.len(), 1);
        assert_eq!(resp.output_data.len(), 1);
        assert!(resp.output_data[0].enable);
        assert_eq!(resp.input_index, None);
        assert_eq!(resp.output_index, None);
    }

    #[test]
    fn input_output_list_with_selection_indices() {
        let json = r#"{
            "status": 200,
            "inputData": [
                {"name": "Internal player", "tag": "local", "sortedIndex": 0},
                {"name": "USB", "tag": "usb", "sortedIndex": 1}
            ],
            "inputIndex": 0,
            "outputData": [
                {"name": "XLR/RCA", "tag": "xlrrca", "enable": true, "sortedIndex": 0},
                {"name": "HDMI", "tag": "hdmi", "enable": true, "sortedIndex": 1}
            ],
            "outputIndex": 1
        }"#;
        let resp: InputOutputListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.input_index, Some(0));
        assert_eq!(resp.output_index, Some(1));
        assert_eq!(resp.input_data[0].tag, "local");
        assert_eq!(resp.output_data[1].tag, "hdmi");
    }

    #[test]
    fn power_options_response_deserialization() {
        let json = r#"{
            "status": 200,
            "data": [
                {"name": "Power Off", "tag": "poweroff"},
                {"name": "Reboot", "tag": "reboot"}
            ]
        }"#;
        let resp: PowerOptionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 2);
        assert_eq!(resp.data[0].tag, "poweroff");
    }

    #[test]
    fn brightness_response_deserialization() {
        let json = r#"{"status": 200, "currentValue": 5, "max": 10}"#;
        let resp: BrightnessResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.current_value, 5);
        assert_eq!(resp.status, Some(200));
        assert_eq!(resp.max, Some(10));
    }

    #[test]
    fn brightness_response_without_status() {
        let json = r#"{"currentValue": 80}"#;
        let resp: BrightnessResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.current_value, 80);
        assert_eq!(resp.status, None);
        assert_eq!(resp.max, None);
    }

    #[test]
    fn display_mode_list_response_deserialization() {
        let json = r#"{
            "status": 200,
            "data": [
                {"title": "Classic VU", "tag": "vu_classic", "index": 0},
                {"title": "Modern VU", "tag": "vu_modern", "index": 1}
            ],
            "currentIndex": 0
        }"#;
        let resp: DisplayModeListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 2);
        assert_eq!(resp.data[0].title, "Classic VU");
        assert_eq!(resp.data[0].tag, Some("vu_classic".to_string()));
        assert_eq!(resp.data[0].index, Some(0));
        assert_eq!(resp.current_index, Some(0));
    }

    #[test]
    fn display_mode_minimal_fields() {
        let json = r#"{"title": "Spectrum 1"}"#;
        let mode: DisplayMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode.title, "Spectrum 1");
        assert_eq!(mode.tag, None);
        assert_eq!(mode.index, None);
    }

    #[test]
    fn get_model_response_full_zidoo_payload() {
        // Real response includes many extra fields; serde should ignore them.
        let json = r#"{
            "status": 200,
            "model": "DMP-A8",
            "disModel": "DMP-A8",
            "deviceName": "DMP-A8",
            "ip": "192.168.20.45",
            "net_mac": "AA:BB:CC:DD:EE:FF",
            "dType": 0,
            "duuid": "AA:BB:CC:DD:EE:FF",
            "firmware": "v2.3.20",
            "ram": " 8.0G",
            "flash": " 64.0G",
            "androidversion": "11",
            "wif_mac": "11:22:33:44:55:66",
            "language": "en",
            "ableRemoteBoot": true,
            "ableRemoteShutdown": true,
            "ableRemoteReboot": true,
            "ableRemoteSleep": false,
            "ableMusicService": true,
            "hasEqSetting": true,
            "appcode": 7878
        }"#;
        let resp: GetModelResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.model, "DMP-A8");
        assert_eq!(resp.net_mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(resp.wifi_mac, "11:22:33:44:55:66");
        assert_eq!(resp.firmware, "v2.3.20");
        assert_eq!(resp.android_version, Some("11".to_string()));
        assert_eq!(resp.able_remote_boot, Some(true));
        assert_eq!(resp.has_eq_setting, Some(true));
    }

    #[test]
    fn get_model_wifi_mac_alias() {
        // Some firmware may use "wifi_mac" instead of the typo "wif_mac"
        let json = r#"{
            "status": 200,
            "model": "DMP-A8",
            "ip": "192.168.1.50",
            "net_mac": "AA:BB:CC:DD:EE:FF",
            "wifi_mac": "11:22:33:44:55:66",
            "firmware": "2.3.20"
        }"#;
        let resp: GetModelResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.wifi_mac, "11:22:33:44:55:66");
    }

    #[test]
    fn output_item_enable_defaults_to_false() {
        let json = r#"{"name": "Test", "tag": "test"}"#;
        let item: OutputItem = serde_json::from_str(json).unwrap();
        assert!(!item.enable);
    }

    #[test]
    fn volume_data_is_mute_defaults_to_false() {
        let json = r#"{"currenttVolume": 50, "maxVolume": 100}"#;
        let data: VolumeData = serde_json::from_str(json).unwrap();
        assert!(!data.is_mute);
    }
}
