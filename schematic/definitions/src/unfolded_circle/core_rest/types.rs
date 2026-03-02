//! Unfolded Circle Core REST model types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Localized text map keyed by language code (for example `en`, `en_US`).
pub type LanguageText = BTreeMap<String, String>;

/// Login request payload for `/pub/login`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoginRequest {
    /// User account name.
    pub username: String,
    /// User password.
    pub password: String,
}

/// Generic API response envelope used by many endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiResponseMessage {
    /// Response status text.
    pub status: Option<String>,
    /// Human-readable message.
    pub message: Option<String>,
}

/// System information returned by `/system`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Friendly name of the device model.
    pub model_name: Option<String>,
    /// Full model number (for example `ucr2`, `ucr3-##`).
    pub model_number: Option<String>,
    /// Hardware serial number.
    pub serial_number: Option<String>,
    /// Hardware revision identifier.
    pub hw_revision: Option<String>,
}

/// Single uploaded/listed resource item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceItem {
    /// Resource type.
    ///
    /// Serialized as `type` in API payloads.
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    /// Resource identifier.
    pub id: Option<String>,
    /// Size in bytes.
    pub size: Option<u32>,
}

/// Resource list response.
pub type ResourceItems = Vec<ResourceItem>;

/// Integration driver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CodeSetUploadResult {
    /// Total number of processed CSV rows.
    pub processed: Option<u32>,
    /// Number of newly added rows.
    pub added: Option<u32>,
    /// Number of updated IR code rows.
    pub updated: Option<u32>,
}

/// Backup restore report item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackupRestoreReportItem {
    /// Restored object category.
    pub item: BackupRestoreItem,
    /// Total available items in the backup.
    pub available: i32,
    /// Successfully restored items.
    pub ok: i32,
}

/// Type of custom component installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomComponent {
    /// Custom UI replacement.
    Ui,
    /// Custom web configurator replacement.
    WebConfigurator,
}

/// Installed custom component information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
