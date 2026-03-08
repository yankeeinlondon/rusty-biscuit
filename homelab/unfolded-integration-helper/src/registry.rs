//! Device registry data model for UC integrations.
//!
//! Defines the types that represent device lifecycle states: discovered
//! (`KnownDevice`), configured (`ConfiguredDevice`), and bound to a Remote
//! (`RemoteAssignment`).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How a device was first discovered by the integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    /// Operator passed `--host` or a seed file.
    CliHint,
    /// Found via network scan.
    NetworkScan,
    /// Configured through Remote setup flow.
    RemoteSetup,
    /// Loaded from persisted state.
    Persisted,
}

/// Resolved device identity and capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    /// Integration-specific extra fields (e.g., `ableRemoteBoot`, `wif_mac`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// A device the server has discovered, validated, or been told about.
///
/// Not necessarily assigned to any Remote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownDevice {
    /// Stable identity (e.g., MAC address, serial, or model+host fingerprint).
    pub device_id: String,
    /// How this device was found.
    pub source: DiscoverySource,
    /// Transport address.
    pub host: String,
    pub port: u16,
    /// Device metadata resolved during validation.
    #[serde(default)]
    pub metadata: DeviceMetadata,
    /// When this device was last successfully validated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validated: Option<DateTime<Utc>>,
}

/// A concrete device instance the server can poll and control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfiguredDevice {
    pub device_id: String,
    pub device_name: String,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub metadata: DeviceMetadata,
    /// Integration-specific config (e.g., WOL params for Eversolo).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub driver_config: HashMap<String, Value>,
}

/// A binding between a Remote and configured device instances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteAssignment {
    pub remote_id: String,
    pub device_ids: Vec<String>,
}

/// The full persisted registry state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegistryData {
    #[serde(default)]
    pub known_devices: Vec<KnownDevice>,
    #[serde(default)]
    pub configured_devices: Vec<ConfiguredDevice>,
    #[serde(default)]
    pub assignments: Vec<RemoteAssignment>,
}
