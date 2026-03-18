//! Unfolded Circle Core REST model types.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Localized text map keyed by language code (for example `en`, `en_US`).
pub type LanguageText = BTreeMap<String, String>;

/// Login request payload for `/pub/login`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct LoginRequest {
    /// User account name.
    pub username: String,
    /// User password.
    pub password: String,
}

/// Generic API response envelope used by many endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ApiResponseMessage {
    /// Response status text.
    pub status: Option<String>,
    /// Human-readable message.
    pub message: Option<String>,
}

/// System information returned by `/system`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct SystemInfo {
    /// Friendly name of the device model.
    pub model_name: String,
    /// Full model number (for example `ucr2`, `ucr3-##`).
    pub model_number: String,
    /// Hardware serial number.
    pub serial_number: String,
    /// Hardware revision identifier.
    pub hw_revision: String,
}

/// Single uploaded/listed resource item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ResourceItem {
    /// Resource type.
    ///
    /// Serialized as `type` in API payloads.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Resource identifier.
    pub id: String,
    /// Size in bytes.
    pub size: Option<u32>,
}

/// Resource list response.
pub type ResourceItems = Vec<ResourceItem>;

/// Integration driver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum IntegrationDriverType {
    /// Firmware built-in integration driver.
    Local,
    /// User-installed integration driver package.
    Custom,
    /// External network-hosted integration driver.
    External,
}

/// Integration driver runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DriverState {
    /// Driver exists but requires configuration before activation.
    NotConfigured,
    /// Driver is idle.
    Idle,
    /// Driver is currently connecting.
    Connecting,
    /// Driver is connected and active.
    Active,
    /// Driver is attempting to reconnect.
    Reconnecting,
    /// Driver is in an error state.
    Error,
}

/// Installed custom integration summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IntegrationDriverInfo {
    /// Driver identifier.
    pub driver_id: String,
    /// Localized driver display name.
    pub name: LanguageText,
    /// Driver origin/type.
    pub driver_type: IntegrationDriverType,
    /// Driver version.
    pub version: String,
    /// Whether the driver is enabled.
    pub enabled: bool,
    /// Optional developer name.
    pub developer_name: Option<String>,
    /// Optional external driver URL.
    pub driver_url: Option<String>,
    /// Optional icon identifier.
    pub icon: Option<String>,
    /// Optional runtime state.
    pub driver_state: Option<DriverState>,
}

/// Result of uploading IR code-set CSV content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct CodeSetUploadResult {
    /// Total number of processed CSV rows.
    pub processed: u32,
    /// Number of newly added rows.
    pub added: u32,
    /// Number of updated IR code rows.
    pub updated: u32,
}

/// Backup restore report item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackupRestoreItem {
    /// Database records.
    Db,
    /// Integration driver records.
    IntegrationDriver,
    /// Integration records.
    Integration,
    /// Activity records.
    Activity,
    /// Macro records.
    Macro,
    /// Remote records.
    Remote,
    /// Profile records.
    Profile,
    /// Dock records.
    Dock,
    /// Resource records.
    Resource,
}

/// One backup-restore report row.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct BackupRestoreReportItem {
    /// Restored object category.
    pub item: BackupRestoreItem,
    /// Total available items in the backup.
    pub available: i32,
    /// Successfully restored items.
    pub ok: i32,
}

/// Type of custom component installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustomComponent {
    /// Custom UI replacement.
    Ui,
    /// Custom web configurator replacement.
    WebConfigurator,
}

/// Installed custom component information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct CustomInstall {
    /// Component key (`ui` or `web_configurator`).
    pub component: CustomComponent,
    /// Whether the component is installed.
    pub installed: bool,
    /// Whether the component is currently active.
    pub active: bool,
    /// Installation timestamp (when installed).
    pub installation_date: Option<String>,
}

/// Backup restore report list.
pub type BackupRestoreReportItems = Vec<BackupRestoreReportItem>;

#[cfg(test)]
mod tests {
    use super::*;

    // --- Enum serialization tests ---

    #[test]
    fn integration_driver_type_serializes_to_uppercase() {
        assert_eq!(
            serde_json::to_string(&IntegrationDriverType::Local).unwrap(),
            r#""LOCAL""#
        );
        assert_eq!(
            serde_json::to_string(&IntegrationDriverType::Custom).unwrap(),
            r#""CUSTOM""#
        );
        assert_eq!(
            serde_json::to_string(&IntegrationDriverType::External).unwrap(),
            r#""EXTERNAL""#
        );
    }

    #[test]
    fn driver_state_serializes_to_screaming_snake_case() {
        assert_eq!(
            serde_json::to_string(&DriverState::NotConfigured).unwrap(),
            r#""NOT_CONFIGURED""#
        );
        assert_eq!(
            serde_json::to_string(&DriverState::Active).unwrap(),
            r#""ACTIVE""#
        );
        assert_eq!(
            serde_json::to_string(&DriverState::Idle).unwrap(),
            r#""IDLE""#
        );
        assert_eq!(
            serde_json::to_string(&DriverState::Connecting).unwrap(),
            r#""CONNECTING""#
        );
        assert_eq!(
            serde_json::to_string(&DriverState::Reconnecting).unwrap(),
            r#""RECONNECTING""#
        );
        assert_eq!(
            serde_json::to_string(&DriverState::Error).unwrap(),
            r#""ERROR""#
        );
    }

    #[test]
    fn backup_restore_item_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Db).unwrap(),
            r#""db""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::IntegrationDriver).unwrap(),
            r#""integration_driver""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Integration).unwrap(),
            r#""integration""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Activity).unwrap(),
            r#""activity""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Macro).unwrap(),
            r#""macro""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Remote).unwrap(),
            r#""remote""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Profile).unwrap(),
            r#""profile""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Dock).unwrap(),
            r#""dock""#
        );
        assert_eq!(
            serde_json::to_string(&BackupRestoreItem::Resource).unwrap(),
            r#""resource""#
        );
    }

    #[test]
    fn custom_component_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&CustomComponent::Ui).unwrap(),
            r#""ui""#
        );
        assert_eq!(
            serde_json::to_string(&CustomComponent::WebConfigurator).unwrap(),
            r#""web_configurator""#
        );
    }

    // --- Roundtrip tests ---

    #[test]
    fn system_info_roundtrip() {
        let info = SystemInfo {
            model_name: "Remote Two".into(),
            model_number: "ucr2".into(),
            serial_number: "SN-12345".into(),
            hw_revision: "rev3".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: SystemInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, parsed);
    }

    #[test]
    fn code_set_upload_result_roundtrip() {
        let result = CodeSetUploadResult {
            processed: 100,
            added: 80,
            updated: 20,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: CodeSetUploadResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, parsed);
    }

    #[test]
    fn resource_item_roundtrip_with_serde_rename() {
        let item = ResourceItem {
            type_name: "icon".into(),
            id: "res-001".into(),
            size: Some(4096),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains(r#""type":"#));
        assert!(!json.contains(r#""type_name""#));
        let parsed: ResourceItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, parsed);
    }

    #[test]
    fn login_request_roundtrip() {
        let req = LoginRequest {
            username: "admin".into(),
            password: "secret".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: LoginRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, parsed);
    }

    #[test]
    fn api_response_message_roundtrip() {
        let msg = ApiResponseMessage {
            status: Some("ok".into()),
            message: Some("Success".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ApiResponseMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn integration_driver_info_roundtrip() {
        let mut name = LanguageText::new();
        name.insert("en".into(), "Test Driver".into());
        let info = IntegrationDriverInfo {
            driver_id: "drv-001".into(),
            name,
            driver_type: IntegrationDriverType::Custom,
            version: "1.0.0".into(),
            enabled: true,
            developer_name: Some("Tester".into()),
            driver_url: None,
            icon: None,
            driver_state: Some(DriverState::Active),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: IntegrationDriverInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, parsed);
    }
}
