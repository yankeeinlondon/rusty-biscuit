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

    /// Migrates environment variables to config if config is empty.
    ///
    /// Checks for `SONY_RECEIVER` and `ARCAM_AMP` environment variables.
    /// If found and config is empty for that device type, adds the device
    /// with a random petname (e.g., "brave-cardinal").
    ///
    /// ## Returns
    ///
    /// Returns `true` if any migration occurred and config was modified.
    pub fn migrate_from_env(&mut self) -> bool {
        let mut modified = false;

        // Migrate SONY_RECEIVER if present and no receivers configured
        if self.sony_receivers.is_empty()
            && let Ok(host) = std::env::var("SONY_RECEIVER")
            && !host.is_empty()
        {
            let name = generate_petname();
            let (host, port) = parse_host_port(&host, 10000);
            self.sony_receivers
                .insert(name, SonyReceiverService { host, port });
            modified = true;
        }

        // Migrate ARCAM_AMP if present and no amps configured
        if self.arcam_amps.is_empty()
            && let Ok(host) = std::env::var("ARCAM_AMP")
            && !host.is_empty()
        {
            let name = generate_petname();
            let (host, port) = parse_host_port(&host, 50000);
            self.arcam_amps
                .insert(name, ArcamAmpService { host, port });
            modified = true;
        }

        modified
    }
}

/// Generates a random two-word petname like "brave-cardinal".
fn generate_petname() -> String {
    petname::Petnames::default()
        .generate_one(2, "-")
        .unwrap_or_else(|| "device".to_string())
}

/// Parses a host string with optional port (host:port format).
fn parse_host_port(input: &str, default_port: u16) -> (String, u16) {
    // Handle IPv6 with brackets: [::1]:port
    if input.starts_with('[') {
        if let Some(bracket_idx) = input.find("]:") {
            let host = input[1..bracket_idx].to_string();
            let port = input[bracket_idx + 2..]
                .parse()
                .unwrap_or(default_port);
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

    // --- Petname Generation Tests ---

    #[test]
    fn test_generate_petname_format() {
        let name = generate_petname();
        // Should be two words separated by hyphen
        assert!(name.contains('-'), "petname should contain hyphen: {}", name);
        assert!(is_valid_device_name(&name), "petname should be valid: {}", name);
    }

    #[test]
    fn test_generate_petname_uniqueness() {
        // Generate multiple names and ensure they're different (probabilistic)
        let names: Vec<_> = (0..5).map(|_| generate_petname()).collect();
        let unique_count = names.iter().collect::<std::collections::HashSet<_>>().len();
        // With 5 random names, we should have at least 3 unique (very high probability)
        assert!(unique_count >= 3, "expected at least 3 unique names, got {}", unique_count);
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

    // --- ENV Migration Tests ---

    #[test]
    #[serial_test::serial]
    fn test_migrate_from_env_sony() {
        // Clear any existing env vars
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("SONY_RECEIVER", "192.168.1.100:10000");
        }

        let mut config = HomeyConfig::new();
        let modified = config.migrate_from_env();

        assert!(modified);
        assert_eq!(config.sony_receivers.len(), 1);
        assert!(config.arcam_amps.is_empty());

        // Check the migrated device
        let (name, service) = config.sony_receivers.iter().next().unwrap();
        assert!(is_valid_device_name(name));
        assert_eq!(service.host, "192.168.1.100");
        assert_eq!(service.port, 10000);

        unsafe { std::env::remove_var("SONY_RECEIVER"); }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_from_env_arcam() {
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("ARCAM_AMP", "192.168.1.101");
        }

        let mut config = HomeyConfig::new();
        let modified = config.migrate_from_env();

        assert!(modified);
        assert!(config.sony_receivers.is_empty());
        assert_eq!(config.arcam_amps.len(), 1);

        let (name, service) = config.arcam_amps.iter().next().unwrap();
        assert!(is_valid_device_name(name));
        assert_eq!(service.host, "192.168.1.101");
        assert_eq!(service.port, 50000);

        unsafe { std::env::remove_var("ARCAM_AMP"); }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_skips_if_already_configured() {
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("SONY_RECEIVER", "192.168.1.100");
        }

        let mut config = HomeyConfig::new();
        config.sony_receivers.insert(
            "existing".to_string(),
            SonyReceiverService {
                host: "10.0.0.1".to_string(),
                port: 10000,
            },
        );

        let modified = config.migrate_from_env();

        assert!(!modified);
        assert_eq!(config.sony_receivers.len(), 1);
        assert!(config.sony_receivers.contains_key("existing"));

        unsafe { std::env::remove_var("SONY_RECEIVER"); }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_skips_empty_env_var() {
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("SONY_RECEIVER", "");
        }

        let mut config = HomeyConfig::new();
        let modified = config.migrate_from_env();

        assert!(!modified);
        assert!(config.sony_receivers.is_empty());

        unsafe { std::env::remove_var("SONY_RECEIVER"); }
    }
}
