//! Configuration file management for homelab server.
//!
//! Configuration is stored in `~/homey.json` and loaded at startup.
//! The REST API can modify the configuration, which is auto-saved to disk.

use petname::Generator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default configuration filename
const CONFIG_FILENAME: &str = "homey.json";

/// Configuration for the homelab server.
///
/// Stores device configurations for Sony receivers and Arcam amplifiers.
/// Configuration is persisted to `~/homey.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HomeyConfig {
    /// Sony receiver configurations keyed by device name
    #[serde(default)]
    pub sony_receivers: HashMap<String, SonyReceiverService>,

    /// Arcam amplifier configurations keyed by device name
    #[serde(default)]
    pub arcam_amps: HashMap<String, ArcamAmpService>,
}

/// Configuration for a Sony ES receiver.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SonyReceiverService {
    /// Hostname or IP address of the receiver
    pub host: String,

    /// Port number (default: 10000)
    #[serde(default = "default_sony_port")]
    pub port: u16,
}

fn default_sony_port() -> u16 {
    10000
}

/// Configuration for an Arcam amplifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArcamAmpService {
    /// Hostname or IP address of the amplifier
    pub host: String,

    /// Port number (default: 50000)
    #[serde(default = "default_arcam_port")]
    pub port: u16,
}

fn default_arcam_port() -> u16 {
    50000
}

impl HomeyConfig {
    /// Creates an empty configuration.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Validates a device name against the allowed pattern.
///
/// Device names must be lowercase alphanumeric with underscores and hyphens only.
///
/// ## Returns
///
/// `true` if the name is valid, `false` otherwise.
pub fn is_valid_device_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Manual check to avoid regex dependency
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_config_serialization() {
        let config = HomeyConfig::new();
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("sony_receivers"));
        assert!(json.contains("arcam_amps"));
    }

    #[test]
    fn test_config_with_devices() {
        let mut config = HomeyConfig::new();
        config.sony_receivers.insert(
            "living-room".to_string(),
            SonyReceiverService {
                host: "192.168.1.100".to_string(),
                port: 10000,
            },
        );
        config.arcam_amps.insert(
            "office".to_string(),
            ArcamAmpService {
                host: "192.168.1.101".to_string(),
                port: 50000,
            },
        );

        let json = serde_json::to_string(&config).unwrap();
        let parsed: HomeyConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config, parsed);
        assert!(parsed.sony_receivers.contains_key("living-room"));
        assert!(parsed.arcam_amps.contains_key("office"));
    }

    #[test]
    fn test_sony_receiver_default_port() {
        let json = r#"{"host": "192.168.1.100"}"#;
        let service: SonyReceiverService = serde_json::from_str(json).unwrap();
        assert_eq!(service.port, 10000);
    }

    #[test]
    fn test_arcam_amp_default_port() {
        let json = r#"{"host": "192.168.1.101"}"#;
        let service: ArcamAmpService = serde_json::from_str(json).unwrap();
        assert_eq!(service.port, 50000);
    }

    #[test]
    fn test_valid_device_names() {
        assert!(is_valid_device_name("living-room"));
        assert!(is_valid_device_name("office_1"));
        assert!(is_valid_device_name("device123"));
        assert!(is_valid_device_name("a"));
        assert!(is_valid_device_name("test-device-name"));
    }

    #[test]
    fn test_invalid_device_names() {
        assert!(!is_valid_device_name("")); // empty
        assert!(!is_valid_device_name("Living-Room")); // uppercase
        assert!(!is_valid_device_name("device.name")); // dots
        assert!(!is_valid_device_name("device name")); // space
        assert!(!is_valid_device_name("device/name")); // slash
        assert!(!is_valid_device_name("café")); // non-ascii
    }
}
