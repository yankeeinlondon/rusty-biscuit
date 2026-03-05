//! Samsung Smart TV REST API model types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level response from the Samsung Smart View `/api/v2/` info endpoint.
///
/// Fields vary by model and firmware revision. The typed core covers fields
/// observed on S95C-class TVs; unknown fields are captured in `extra` to
/// preserve round-trip fidelity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SamsungDeviceInfoResponse {
    /// Device UUID assigned by the TV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Human-readable device name (often the TV's network hostname).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Nested device details block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<SamsungDeviceInfo>,

    /// Firmware-variable fields not covered by typed members.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Nested device information block within the Smart View info response.
///
/// All fields are optional because firmware revisions may omit or rename
/// properties without notice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SamsungDeviceInfo {
    /// Hardware model identifier (e.g., `"QN65S95CAFXZA"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Marketing model name (e.g., `"S95C"`).
    #[serde(rename = "modelName", skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,

    /// Tizen OS version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,

    /// Network connection type (e.g., `"wireless"`, `"wired"`).
    #[serde(rename = "networkType", skip_serializing_if = "Option::is_none")]
    pub network_type: Option<String>,

    /// Display resolution string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    /// Whether the TV supports token-based remote auth.
    #[serde(rename = "TokenAuthSupport", skip_serializing_if = "Option::is_none")]
    pub token_auth_support: Option<String>,

    /// Whether the TV supports Frame/Art Mode (`"true"` or `"false"`).
    #[serde(rename = "FrameTVSupport", skip_serializing_if = "Option::is_none")]
    pub frame_tv_support: Option<String>,

    /// TV's own IP address on the local network.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,

    /// Wi-Fi MAC address (useful for Wake-on-LAN).
    #[serde(rename = "wifiMac", skip_serializing_if = "Option::is_none")]
    pub wifi_mac: Option<String>,

    /// Firmware version string.
    #[serde(rename = "firmwareVersion", skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,

    /// Device manufacturer name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,

    /// Country/region code.
    #[serde(rename = "countryCode", skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,

    /// Unique device identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duid: Option<String>,

    /// Firmware-variable fields not covered by typed members.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Response from the app status endpoint (`GET /api/v2/applications/{app_id}`).
///
/// Returns running state, version, and other metadata for a specific app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SamsungAppStatusResponse {
    /// Whether the app is currently visible on screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,

    /// Whether the app is currently running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,

    /// App version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// App display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// App package ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Additional app metadata not covered by typed fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_info_response_deserializes_known_fields() {
        let json = r#"{
            "id": "uuid-1234",
            "name": "Living Room TV",
            "device": {
                "model": "QN65S95CAFXZA",
                "modelName": "S95C",
                "os": "Tizen 7.0",
                "networkType": "wireless",
                "resolution": "3840x2160"
            }
        }"#;

        let resp: SamsungDeviceInfoResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id.as_deref(), Some("uuid-1234"));
        assert_eq!(resp.name.as_deref(), Some("Living Room TV"));
        let device = resp.device.as_ref().unwrap();
        assert_eq!(device.model.as_deref(), Some("QN65S95CAFXZA"));
        assert_eq!(device.model_name.as_deref(), Some("S95C"));
        assert_eq!(device.os.as_deref(), Some("Tizen 7.0"));
        assert_eq!(device.network_type.as_deref(), Some("wireless"));
        assert_eq!(device.resolution.as_deref(), Some("3840x2160"));
    }

    #[test]
    fn device_info_response_preserves_unknown_fields() {
        let json = r#"{
            "id": "uuid-1234",
            "version": "2.0.25",
            "device": {
                "model": "QN65S95CAFXZA",
                "wifiMac": "AA:BB:CC:DD:EE:FF",
                "type": "Samsung SmartTV"
            },
            "uri": "http://192.168.1.1:8001/api/v2/"
        }"#;

        let resp: SamsungDeviceInfoResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.extra.get("version").and_then(|v| v.as_str()), Some("2.0.25"));
        assert_eq!(resp.extra.get("uri").and_then(|v| v.as_str()), Some("http://192.168.1.1:8001/api/v2/"));

        let device = resp.device.as_ref().unwrap();
        // wifiMac is now a typed field
        assert_eq!(device.wifi_mac.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
        assert_eq!(
            device.extra.get("type").and_then(|v| v.as_str()),
            Some("Samsung SmartTV")
        );
    }

    #[test]
    fn device_info_response_handles_minimal_payload() {
        let json = r#"{}"#;
        let resp: SamsungDeviceInfoResponse = serde_json::from_str(json).unwrap();
        assert!(resp.id.is_none());
        assert!(resp.name.is_none());
        assert!(resp.device.is_none());
        assert!(resp.extra.is_empty());
    }

    #[test]
    fn device_info_roundtrips_through_serde() {
        let original = SamsungDeviceInfoResponse {
            id: Some("uuid-1234".to_string()),
            name: Some("TV".to_string()),
            device: Some(SamsungDeviceInfo {
                model: Some("QN65S95CAFXZA".to_string()),
                model_name: None,
                os: None,
                network_type: None,
                resolution: None,
                token_auth_support: None,
                frame_tv_support: None,
                ip: None,
                wifi_mac: None,
                firmware_version: None,
                manufacturer: None,
                country_code: None,
                duid: None,
                extra: BTreeMap::new(),
            }),
            extra: BTreeMap::new(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let restored: SamsungDeviceInfoResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn device_info_promotes_frame_tv_support() {
        let json = r#"{
            "device": {
                "FrameTVSupport": "true",
                "ip": "192.168.1.42",
                "firmwareVersion": "T-KTMDEUC-1300.3",
                "manufacturer": "Samsung",
                "countryCode": "US",
                "duid": "uuid:12345678-1234-1234-1234-123456789012"
            }
        }"#;

        let resp: SamsungDeviceInfoResponse = serde_json::from_str(json).unwrap();
        let device = resp.device.as_ref().unwrap();
        assert_eq!(device.frame_tv_support.as_deref(), Some("true"));
        assert_eq!(device.ip.as_deref(), Some("192.168.1.42"));
        assert_eq!(device.firmware_version.as_deref(), Some("T-KTMDEUC-1300.3"));
        assert_eq!(device.manufacturer.as_deref(), Some("Samsung"));
        assert_eq!(device.country_code.as_deref(), Some("US"));
        assert_eq!(device.duid.as_deref(), Some("uuid:12345678-1234-1234-1234-123456789012"));
    }

    #[test]
    fn app_status_response_deserializes() {
        let json = r#"{
            "visible": true,
            "running": true,
            "version": "3.0.0",
            "name": "YouTube",
            "id": "111299001912",
            "extra_field": "value"
        }"#;

        let resp: SamsungAppStatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.visible, Some(true));
        assert_eq!(resp.running, Some(true));
        assert_eq!(resp.version.as_deref(), Some("3.0.0"));
        assert_eq!(resp.name.as_deref(), Some("YouTube"));
        assert_eq!(resp.id.as_deref(), Some("111299001912"));
        assert_eq!(resp.extra.get("extra_field").and_then(|v| v.as_str()), Some("value"));
    }
}
