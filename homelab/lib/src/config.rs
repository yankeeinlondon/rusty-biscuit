//! Configuration file management for homelab devices.
//!
//! Configuration is stored in `~/homey.json` and shared between the CLI and server.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default configuration filename
const CONFIG_FILENAME: &str = "homey.json";

/// Configuration for the homelab system.
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

    /// Eversolo DMP-A8 music streamer configurations keyed by device name
    #[serde(default)]
    pub eversolo_devices: HashMap<String, EversoloService>,

    /// Samsung Smart TV configurations keyed by device name
    #[serde(default)]
    pub samsung_tvs: HashMap<String, SamsungTvService>,
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

/// Configuration for an Eversolo DMP-A8 music streamer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EversoloService {
    /// Hostname or IP address of the Eversolo
    pub host: String,

    /// Port number (default: 9529)
    #[serde(default = "default_eversolo_port")]
    pub port: u16,

    /// MAC address for Wake-on-LAN (auto-detected from device info)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
}

fn default_eversolo_port() -> u16 {
    9529
}

/// Configuration for a Samsung Smart TV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SamsungTvService {
    /// Hostname or IP address of the TV
    pub host: String,

    /// REST API port number (default: 8001)
    #[serde(default = "default_samsung_rest_port")]
    pub rest_port: u16,

    /// WebSocket API port number (default: 8002)
    #[serde(default = "default_samsung_ws_port")]
    pub ws_port: u16,

    /// MAC address for Wake-on-LAN (auto-detected from device info)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
}

fn default_samsung_rest_port() -> u16 {
    8001
}

fn default_samsung_ws_port() -> u16 {
    8002
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// I/O error reading or writing config file
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// Invalid device name
    #[error("Invalid device name: {0}")]
    InvalidDeviceName(String),

    /// Invalid host (empty)
    #[error("Invalid host: {0}")]
    InvalidHost(String),
}

impl HomeyConfig {
    /// Creates an empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the default configuration file path (`~/homey.json`).
    pub fn default_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(CONFIG_FILENAME))
    }

    /// Loads configuration from the default path (`~/homey.json`).
    ///
    /// If the file doesn't exist, creates an empty config file and returns default config.
    /// If the file is empty or invalid, returns default config.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - Home directory cannot be determined
    /// - File exists but cannot be read
    /// - File cannot be created
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::default_path().ok_or_else(|| {
            ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Home directory not found",
            ))
        })?;
        Self::load_from(&path)
    }

    /// Loads configuration from the specified path.
    ///
    /// If the file doesn't exist, creates an empty config file and returns default config.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            let config = Self::new();
            config.save_to(path)?;
            return Ok(config);
        }

        let contents = std::fs::read_to_string(path)?;
        if contents.trim().is_empty() {
            return Ok(Self::new());
        }

        let config: Self = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Saves configuration to the default path (`~/homey.json`).
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - Home directory cannot be determined
    /// - File cannot be written
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::default_path().ok_or_else(|| {
            ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Home directory not found",
            ))
        })?;
        self.save_to(&path)
    }

    /// Saves configuration to the specified path.
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

/// Parses a host string with optional port (host:port format).
pub fn parse_host_port(input: &str, default_port: u16) -> (String, u16) {
    // Handle IPv6 with brackets: [::1]:port
    if input.starts_with('[') {
        if let Some(bracket_idx) = input.find("]:") {
            let host = input[1..bracket_idx].to_string();
            let port = input[bracket_idx + 2..].parse().unwrap_or(default_port);
            return (host, port);
        }
        // Just [::1] without port
        let host = input.trim_start_matches('[').trim_end_matches(']');
        return (host.to_string(), default_port);
    }

    // Count colons to detect IPv6 vs IPv4:port
    let colon_count = input.matches(':').count();
    if colon_count > 1 {
        // IPv6 address without port
        return (input.to_string(), default_port);
    }

    if let Some(idx) = input.rfind(':') {
        let host = &input[..idx];
        let port = input[idx + 1..].parse().unwrap_or(default_port);
        (host.to_string(), port)
    } else {
        (input.to_string(), default_port)
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
    use tempfile::TempDir;

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
    fn test_eversolo_default_port() {
        let json = r#"{"host": "192.168.1.50"}"#;
        let service: EversoloService = serde_json::from_str(json).unwrap();
        assert_eq!(service.port, 9529);
    }

    #[test]
    fn test_config_backward_compat() {
        let json = r#"{"sony_receivers": {}, "arcam_amps": {}}"#;
        let config: HomeyConfig = serde_json::from_str(json).unwrap();
        assert!(config.eversolo_devices.is_empty());
    }

    #[test]
    fn test_config_with_eversolo() {
        let mut config = HomeyConfig::new();
        config.eversolo_devices.insert(
            "living-room".to_string(),
            EversoloService {
                host: "192.168.1.50".to_string(),
                port: 9529,
                mac_address: None,
            },
        );

        let json = serde_json::to_string(&config).unwrap();
        let parsed: HomeyConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config, parsed);
        assert!(parsed.eversolo_devices.contains_key("living-room"));
        assert_eq!(parsed.eversolo_devices["living-room"].port, 9529);
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
        assert!(!is_valid_device_name("caf\u{e9}")); // non-ascii
    }

    // --- File I/O Tests ---

    #[test]
    fn test_load_creates_file_if_missing() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("homey.json");

        assert!(!config_path.exists());

        let config = HomeyConfig::load_from(&config_path).unwrap();
        assert!(config_path.exists());
        assert!(config.sony_receivers.is_empty());
        assert!(config.arcam_amps.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("homey.json");

        let mut config = HomeyConfig::new();
        config.sony_receivers.insert(
            "test-receiver".to_string(),
            SonyReceiverService {
                host: "192.168.1.50".to_string(),
                port: 10000,
            },
        );

        config.save_to(&config_path).unwrap();
        let loaded = HomeyConfig::load_from(&config_path).unwrap();

        assert_eq!(config, loaded);
    }

    #[test]
    fn test_load_empty_file_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("homey.json");

        std::fs::write(&config_path, "").unwrap();

        let config = HomeyConfig::load_from(&config_path).unwrap();
        assert!(config.sony_receivers.is_empty());
        assert!(config.arcam_amps.is_empty());
    }

    #[test]
    fn test_load_whitespace_only_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("homey.json");

        std::fs::write(&config_path, "   \n\t  ").unwrap();

        let config = HomeyConfig::load_from(&config_path).unwrap();
        assert!(config.sony_receivers.is_empty());
    }

    // --- Parse Host Port Tests ---

    #[test]
    fn test_parse_host_port_simple() {
        let (host, port) = parse_host_port("192.168.1.100", 10000);
        assert_eq!(host, "192.168.1.100");
        assert_eq!(port, 10000);
    }

    #[test]
    fn test_parse_host_port_with_port() {
        let (host, port) = parse_host_port("192.168.1.100:8080", 10000);
        assert_eq!(host, "192.168.1.100");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_host_port_dns() {
        let (host, port) = parse_host_port("receiver.local:9000", 10000);
        assert_eq!(host, "receiver.local");
        assert_eq!(port, 9000);
    }

    #[test]
    fn test_parse_host_port_ipv6_no_port() {
        let (host, port) = parse_host_port("::1", 10000);
        assert_eq!(host, "::1");
        assert_eq!(port, 10000);
    }

    #[test]
    fn test_parse_host_port_ipv6_with_brackets() {
        let (host, port) = parse_host_port("[::1]:8080", 10000);
        assert_eq!(host, "::1");
        assert_eq!(port, 8080);
    }
}
