//! Unfolded Circle Core REST model types.

use serde::{Deserialize, Serialize};

/// Login request payload for `/pub/login`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    /// User account name.
    pub username: String,
    /// User password.
    pub password: String,
}

/// Generic API response envelope used by many endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponseMessage {
    /// Response status text.
    pub status: Option<String>,
    /// Human-readable message.
    pub message: Option<String>,
}

/// System information returned by `/system`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Device identifier.
    pub id: Option<String>,
    /// Hardware model.
    pub model: Option<String>,
    /// Hardware serial number.
    pub serial: Option<String>,
    /// Firmware or software version.
    pub version: Option<String>,
}

/// Single uploaded/listed resource item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceItem {
    /// Resource identifier.
    pub id: Option<String>,
    /// Resource name.
    pub name: Option<String>,
    /// Resource type name.
    #[serde(rename = "type")]
    pub type_name: Option<String>,
}

/// Resource list wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceItems {
    /// Uploaded or listed resource entries.
    pub items: Vec<ResourceItem>,
}

/// Installed custom integration summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationDriverInfo {
    /// Driver identifier.
    pub driver_id: Option<String>,
    /// Driver name.
    pub name: Option<String>,
    /// Driver version.
    pub version: Option<String>,
}

/// Result of uploading IR code-set CSV content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSetUploadResult {
    /// Number of created IR code rows.
    pub created: Option<u32>,
    /// Number of updated IR code rows.
    pub updated: Option<u32>,
    /// Number of failed rows.
    pub failed: Option<u32>,
}

/// One backup-restore report row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreReportItem {
    /// Status code if present.
    pub code: Option<u16>,
    /// Report detail line.
    pub message: Option<String>,
}

/// Installed custom component information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomInstall {
    /// Component key (`ui`, `web_configurator`, ...).
    pub component: Option<String>,
    /// Whether the component is installed.
    pub installed: Option<bool>,
    /// Whether the component is currently active.
    pub active: Option<bool>,
}

/// Backup restore report list.
pub type BackupRestoreReportItems = Vec<BackupRestoreReportItem>;
