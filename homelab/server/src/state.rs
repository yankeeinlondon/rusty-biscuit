//! Application state management for homelab server.
//!
//! Provides both single-device (legacy ENV-based) and multi-device (config-based)
//! state management.

use crate::config::{
    ArcamAmpService, EversoloService, HomeyConfig, SamsungTvService, SonyReceiverService,
};
use homelab::{network::Host, sony_receiver::SonyReceiver};
use std::{
    collections::HashMap, sync::Arc, sync::atomic::AtomicBool, time::Duration, time::Instant,
};
use tokio::sync::RwLock;

/// Default port for Sony receivers
pub const SONY_DEFAULT_PORT: u16 = 10000;

/// Default port for Arcam amplifiers
pub const ARCAM_DEFAULT_PORT: u16 = 50000;

/// Default request timeout in milliseconds
const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Application state shared across handlers.
///
/// Supports both single-device (legacy) and multi-device (config-based) modes.
/// The multi-device state is protected by `tokio::sync::RwLock` for concurrent access.
#[derive(Clone)]
pub struct AppState {
    /// Legacy: Sony receiver client (pre-configured, reusable)
    pub sony: Option<Arc<SonyReceiver>>,
    /// Legacy: Arcam host (we create connections per-request)
    pub arcam_host: Option<String>,
    /// Request timeout for device operations
    pub request_timeout: Duration,

    /// Multi-device: Sony receivers keyed by name
    pub sony_receivers: Arc<RwLock<HashMap<String, Arc<SonyReceiver>>>>,
    /// Multi-device: Arcam hosts keyed by name
    pub arcam_hosts: Arc<RwLock<HashMap<String, ArcamAmpService>>>,
    /// Multi-device: Eversolo devices keyed by name (connections created per-request)
    pub eversolo_devices: Arc<RwLock<HashMap<String, EversoloService>>>,
    /// Multi-device: Samsung TVs keyed by name (connections created per-request)
    pub samsung_tvs: Arc<RwLock<HashMap<String, SamsungTvService>>>,
    /// Configuration state (for CRUD operations)
    pub config: Arc<RwLock<HomeyConfig>>,
    /// Path to config file (for saving changes)
    pub config_path: Option<std::path::PathBuf>,
    /// Cached Arcam model name (queried once on first heartbeat)
    pub arcam_model: Arc<RwLock<Option<String>>>,
    /// Whether the last heartbeat succeeded (for UI heartbeat indicator)
    pub arcam_heartbeat_alive: Arc<AtomicBool>,
    /// Cached Arcam status (for rate-limited polling)
    pub arcam_cached_status: Arc<RwLock<Option<super::DeviceStatusJson>>>,
    /// Last time Arcam was queried (for rate limiting)
    pub arcam_last_query: Arc<RwLock<Option<Instant>>>,
    /// Current Arcam polling interval in seconds (based on Auto Shutdown setting)
    pub arcam_poll_interval_secs: Arc<RwLock<u64>>,
    /// Last known Sony power status (for change-only INFO logging)
    pub sony_last_power_status: Arc<RwLock<Option<String>>>,
    /// Last known Arcam power state (for change-only INFO logging)
    pub arcam_last_power_state: Arc<RwLock<Option<bool>>>,
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid host format: {0}")]
    InvalidHost(String),
}

impl AppState {
    /// Creates application state from environment variables (legacy mode).
    ///
    /// ## Environment Variables
    ///
    /// - `SONY_RECEIVER` - Sony receiver hostname or IP (optional port: `host:port`, default port 10000)
    /// - `ARCAM_AMP` - Arcam amplifier hostname or IP (port always 50000)
    /// - `REQUEST_TIMEOUT_MS` - Request timeout in milliseconds (default 5000)
    ///
    /// ## Errors
    ///
    /// Returns an error if host parsing fails for configured devices.
    pub fn from_env() -> Result<Self, ConfigError> {
        let sony = match std::env::var("SONY_RECEIVER") {
            Ok(host_str) => Some(Arc::new(parse_sony_host(&host_str)?)),
            Err(_) => None,
        };

        let arcam_host = std::env::var("ARCAM_AMP").ok();

        let request_timeout = std::env::var("REQUEST_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        Ok(Self {
            sony,
            arcam_host,
            request_timeout,
            sony_receivers: Arc::new(RwLock::new(HashMap::new())),
            arcam_hosts: Arc::new(RwLock::new(HashMap::new())),
            eversolo_devices: Arc::new(RwLock::new(HashMap::new())),
            samsung_tvs: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(HomeyConfig::new())),
            config_path: None,
            arcam_model: Arc::new(RwLock::new(None)),
            arcam_heartbeat_alive: Arc::new(AtomicBool::new(false)),
            arcam_cached_status: Arc::new(RwLock::new(None)),
            arcam_last_query: Arc::new(RwLock::new(None)),
            arcam_poll_interval_secs: Arc::new(RwLock::new(60)), // Default 1 minute
            sony_last_power_status: Arc::new(RwLock::new(None)),
            arcam_last_power_state: Arc::new(RwLock::new(None)),
        })
    }

    /// Creates application state from a `HomeyConfig`.
    ///
    /// Initializes multi-device state from the configuration.
    /// The config is stored for CRUD operations and auto-save.
    pub fn from_config(config: HomeyConfig, config_path: Option<std::path::PathBuf>) -> Self {
        let mut sony_receivers = HashMap::new();
        let mut arcam_hosts = HashMap::new();
        let mut eversolo_devices = HashMap::new();
        let mut samsung_tvs = HashMap::new();

        // Build Sony receiver clients from config
        for (name, service) in &config.sony_receivers {
            if let Ok(receiver) = create_sony_receiver(service) {
                sony_receivers.insert(name.clone(), Arc::new(receiver));
            }
        }

        // Store Arcam configs (connections created per-request)
        for (name, service) in &config.arcam_amps {
            arcam_hosts.insert(name.clone(), service.clone());
        }

        // Store Eversolo configs (connections created per-request)
        for (name, service) in &config.eversolo_devices {
            eversolo_devices.insert(name.clone(), service.clone());
        }

        // Store Samsung TV configs (connections created per-request)
        for (name, service) in &config.samsung_tvs {
            samsung_tvs.insert(name.clone(), service.clone());
        }

        // Legacy fields: use first device if available
        let sony = sony_receivers.values().next().cloned();
        let arcam_host = arcam_hosts.values().next().map(|s| s.host.clone());

        let request_timeout = std::env::var("REQUEST_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        Self {
            sony,
            arcam_host,
            request_timeout,
            sony_receivers: Arc::new(RwLock::new(sony_receivers)),
            arcam_hosts: Arc::new(RwLock::new(arcam_hosts)),
            eversolo_devices: Arc::new(RwLock::new(eversolo_devices)),
            samsung_tvs: Arc::new(RwLock::new(samsung_tvs)),
            config: Arc::new(RwLock::new(config)),
            config_path,
            arcam_model: Arc::new(RwLock::new(None)),
            arcam_heartbeat_alive: Arc::new(AtomicBool::new(false)),
            arcam_cached_status: Arc::new(RwLock::new(None)),
            arcam_last_query: Arc::new(RwLock::new(None)),
            arcam_poll_interval_secs: Arc::new(RwLock::new(60)), // Default 1 minute
            sony_last_power_status: Arc::new(RwLock::new(None)),
            arcam_last_power_state: Arc::new(RwLock::new(None)),
        }
    }

    /// Gets a Sony receiver by name.
    ///
    /// ## Returns
    ///
    /// `Some(Arc<SonyReceiver>)` if found, `None` if not configured.
    pub async fn get_sony(&self, name: &str) -> Option<Arc<SonyReceiver>> {
        let receivers = self.sony_receivers.read().await;
        receivers.get(name).cloned()
    }

    /// Gets an Arcam host by name.
    ///
    /// ## Returns
    ///
    /// `Some((host, port))` if found, `None` if not configured.
    pub async fn get_arcam_host(&self, name: &str) -> Option<(String, u16)> {
        let hosts = self.arcam_hosts.read().await;
        hosts.get(name).map(|s| (s.host.clone(), s.port))
    }

    /// Invalidates the cached Arcam status so the next poll fetches fresh data.
    ///
    /// Call this after any command that mutates device state (e.g., set auto-shutdown).
    pub async fn invalidate_arcam_cache(&self) {
        *self.arcam_last_query.write().await = None;
    }

    /// Updates the Arcam polling interval based on Auto Shutdown setting.
    ///
    /// Formula: poll_interval = auto_shutdown_seconds + 60
    /// - Auto Shutdown = Off (0): poll every 60 seconds
    /// - Auto Shutdown = 20 min: poll every 1260 seconds (21 min)
    pub async fn update_arcam_poll_interval(&self, auto_shutdown_value: u8) {
        let interval_secs = match auto_shutdown_value {
            0x00 => 60,               // Off -> 1 minute
            0x01 => 20 * 60 + 60,     // 20 min + 1 min = 21 min
            0x02 => 30 * 60 + 60,     // 30 min + 1 min = 31 min
            0x03 => 60 * 60 + 60,     // 1 hour + 1 min
            0x04 => 2 * 60 * 60 + 60, // 2 hours + 1 min
            _ => 60,                  // Default 1 minute
        };
        *self.arcam_poll_interval_secs.write().await = interval_secs;
    }

    /// Adds or updates a Sony receiver and saves config.
    ///
    /// Uses clone-mutate-save pattern to minimize lock duration.
    pub async fn add_sony(
        &self,
        name: String,
        service: SonyReceiverService,
    ) -> Result<(), crate::config::ConfigError> {
        let receiver = create_sony_receiver(&service)
            .map_err(|e| crate::config::ConfigError::InvalidHost(e.to_string()))?;

        // Clone-mutate-save pattern
        let mut config = self.config.read().await.clone();
        config.sony_receivers.insert(name.clone(), service);
        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        // Update runtime state
        {
            let mut receivers = self.sony_receivers.write().await;
            receivers.insert(name, Arc::new(receiver));
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(())
    }

    /// Removes a Sony receiver and saves config.
    pub async fn remove_sony(&self, name: &str) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();
        let removed = config.sony_receivers.remove(name).is_some();

        if removed {
            if let Some(path) = &self.config_path {
                config.save_to(path)?;
            }
            {
                let mut receivers = self.sony_receivers.write().await;
                receivers.remove(name);
            }
            {
                let mut cfg = self.config.write().await;
                *cfg = config;
            }
        }

        Ok(removed)
    }

    /// Adds or updates an Arcam amplifier and saves config.
    pub async fn add_arcam(
        &self,
        name: String,
        service: ArcamAmpService,
    ) -> Result<(), crate::config::ConfigError> {
        if service.host.is_empty() {
            return Err(crate::config::ConfigError::InvalidHost(
                "empty host".to_string(),
            ));
        }

        // Clone-mutate-save pattern
        let mut config = self.config.read().await.clone();
        config.arcam_amps.insert(name.clone(), service.clone());
        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        // Update runtime state
        {
            let mut hosts = self.arcam_hosts.write().await;
            hosts.insert(name, service);
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(())
    }

    /// Removes an Arcam amplifier and saves config.
    pub async fn remove_arcam(&self, name: &str) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();
        let removed = config.arcam_amps.remove(name).is_some();

        if removed {
            if let Some(path) = &self.config_path {
                config.save_to(path)?;
            }
            {
                let mut hosts = self.arcam_hosts.write().await;
                hosts.remove(name);
            }
            {
                let mut cfg = self.config.write().await;
                *cfg = config;
            }
        }

        Ok(removed)
    }

    /// Renames a Sony receiver and saves config.
    pub async fn rename_sony(
        &self,
        old_name: &str,
        new_name: String,
    ) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();

        // Check if old device exists
        let service = match config.sony_receivers.remove(old_name) {
            Some(s) => s,
            None => return Ok(false),
        };

        // Insert with new name
        config
            .sony_receivers
            .insert(new_name.clone(), service.clone());

        // Save config
        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        // Update runtime state
        {
            let mut receivers = self.sony_receivers.write().await;
            receivers.remove(old_name);
            let receiver = create_sony_receiver(&service)
                .map_err(|e| crate::config::ConfigError::InvalidHost(e.to_string()))?;
            receivers.insert(new_name, Arc::new(receiver));
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(true)
    }

    /// Gets an Eversolo device by name.
    ///
    /// ## Returns
    ///
    /// `Some((host, port))` if found, `None` if not configured.
    pub async fn get_eversolo(&self, name: &str) -> Option<(String, u16)> {
        let devices = self.eversolo_devices.read().await;
        devices.get(name).map(|s| (s.host.clone(), s.port))
    }

    /// Adds or updates an Eversolo device and saves config.
    pub async fn add_eversolo(
        &self,
        name: String,
        service: EversoloService,
    ) -> Result<(), crate::config::ConfigError> {
        if service.host.is_empty() {
            return Err(crate::config::ConfigError::InvalidHost(
                "empty host".to_string(),
            ));
        }

        let mut config = self.config.read().await.clone();
        config
            .eversolo_devices
            .insert(name.clone(), service.clone());
        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        {
            let mut devices = self.eversolo_devices.write().await;
            devices.insert(name, service);
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(())
    }

    /// Removes an Eversolo device and saves config.
    pub async fn remove_eversolo(
        &self,
        name: &str,
    ) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();
        let removed = config.eversolo_devices.remove(name).is_some();

        if removed {
            if let Some(path) = &self.config_path {
                config.save_to(path)?;
            }
            {
                let mut devices = self.eversolo_devices.write().await;
                devices.remove(name);
            }
            {
                let mut cfg = self.config.write().await;
                *cfg = config;
            }
        }

        Ok(removed)
    }

    /// Renames an Eversolo device and saves config.
    pub async fn rename_eversolo(
        &self,
        old_name: &str,
        new_name: String,
    ) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();

        let service = match config.eversolo_devices.remove(old_name) {
            Some(s) => s,
            None => return Ok(false),
        };

        config
            .eversolo_devices
            .insert(new_name.clone(), service.clone());

        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        {
            let mut devices = self.eversolo_devices.write().await;
            devices.remove(old_name);
            devices.insert(new_name, service);
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(true)
    }

    /// Gets a Samsung TV by name.
    ///
    /// ## Returns
    ///
    /// `Some((host, rest_port, ws_port))` if found, `None` if not configured.
    pub async fn get_samsung_tv(&self, name: &str) -> Option<(String, u16, u16)> {
        let tvs = self.samsung_tvs.read().await;
        tvs.get(name)
            .map(|s| (s.host.clone(), s.rest_port, s.ws_port))
    }

    /// Adds or updates a Samsung TV and saves config.
    pub async fn add_samsung_tv(
        &self,
        name: String,
        service: SamsungTvService,
    ) -> Result<(), crate::config::ConfigError> {
        if service.host.is_empty() {
            return Err(crate::config::ConfigError::InvalidHost(
                "empty host".to_string(),
            ));
        }

        let mut config = self.config.read().await.clone();
        config
            .samsung_tvs
            .insert(name.clone(), service.clone());
        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        {
            let mut tvs = self.samsung_tvs.write().await;
            tvs.insert(name, service);
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(())
    }

    /// Removes a Samsung TV and saves config.
    pub async fn remove_samsung_tv(
        &self,
        name: &str,
    ) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();
        let removed = config.samsung_tvs.remove(name).is_some();

        if removed {
            if let Some(path) = &self.config_path {
                config.save_to(path)?;
            }
            {
                let mut tvs = self.samsung_tvs.write().await;
                tvs.remove(name);
            }
            {
                let mut cfg = self.config.write().await;
                *cfg = config;
            }
        }

        Ok(removed)
    }

    /// Renames a Samsung TV and saves config.
    pub async fn rename_samsung_tv(
        &self,
        old_name: &str,
        new_name: String,
    ) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();

        let service = match config.samsung_tvs.remove(old_name) {
            Some(s) => s,
            None => return Ok(false),
        };

        config
            .samsung_tvs
            .insert(new_name.clone(), service.clone());

        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        {
            let mut tvs = self.samsung_tvs.write().await;
            tvs.remove(old_name);
            tvs.insert(new_name, service);
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(true)
    }

    /// Renames an Arcam amplifier and saves config.
    pub async fn rename_arcam(
        &self,
        old_name: &str,
        new_name: String,
    ) -> Result<bool, crate::config::ConfigError> {
        let mut config = self.config.read().await.clone();

        // Check if old device exists
        let service = match config.arcam_amps.remove(old_name) {
            Some(s) => s,
            None => return Ok(false),
        };

        // Insert with new name
        config.arcam_amps.insert(new_name.clone(), service.clone());

        // Save config
        if let Some(path) = &self.config_path {
            config.save_to(path)?;
        }

        // Update runtime state
        {
            let mut hosts = self.arcam_hosts.write().await;
            hosts.remove(old_name);
            hosts.insert(new_name, service);
        }
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        Ok(true)
    }
}

/// Creates a SonyReceiver from a service config.
fn create_sony_receiver(service: &SonyReceiverService) -> Result<SonyReceiver, ConfigError> {
    let host = parse_host(&service.host)?;
    Ok(SonyReceiver::new(host, service.port))
}

/// Parse a Sony receiver host string (host or host:port)
fn parse_sony_host(host_str: &str) -> Result<SonyReceiver, ConfigError> {
    let (host, port) = if let Some(idx) = host_str.rfind(':') {
        // Check if this is IPv6 (contains multiple colons)
        if host_str.matches(':').count() > 1 && !host_str.contains('[') {
            // Plain IPv6 without port
            (host_str, SONY_DEFAULT_PORT)
        } else if host_str.starts_with('[') {
            // IPv6 with brackets: [::1]:port
            if let Some(bracket_idx) = host_str.find("]:") {
                let ip_part = &host_str[1..bracket_idx];
                let port_str = &host_str[bracket_idx + 2..];
                let port = port_str
                    .parse()
                    .map_err(|_| ConfigError::InvalidHost(format!("invalid port: {port_str}")))?;
                (ip_part, port)
            } else {
                // Just [::1] without port
                let ip_part = host_str.trim_start_matches('[').trim_end_matches(']');
                (ip_part, SONY_DEFAULT_PORT)
            }
        } else {
            // host:port format
            let port_str = &host_str[idx + 1..];
            let port = port_str
                .parse()
                .map_err(|_| ConfigError::InvalidHost(format!("invalid port: {port_str}")))?;
            (&host_str[..idx], port)
        }
    } else {
        (host_str, SONY_DEFAULT_PORT)
    };

    let host = parse_host(host)?;
    Ok(SonyReceiver::new(host, port))
}

/// Parse a host string into a Host enum using the library function
fn parse_host(host_str: &str) -> Result<Host, ConfigError> {
    homelab::network::parse_host(host_str).map_err(|e| ConfigError::InvalidHost(e.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sony_host_simple() {
        let receiver = parse_sony_host("192.168.1.100").unwrap();
        assert_eq!(receiver.port(), SONY_DEFAULT_PORT);
    }

    #[test]
    fn test_parse_sony_host_with_port() {
        let receiver = parse_sony_host("192.168.1.100:8080").unwrap();
        assert_eq!(receiver.port(), 8080);
    }

    #[test]
    fn test_parse_sony_host_dns() {
        let receiver = parse_sony_host("receiver.local").unwrap();
        assert_eq!(receiver.port(), SONY_DEFAULT_PORT);
    }

    #[test]
    fn test_parse_sony_host_dns_with_port() {
        let receiver = parse_sony_host("receiver.local:9000").unwrap();
        assert_eq!(receiver.port(), 9000);
    }

    #[test]
    fn test_parse_host_ipv4() {
        let host = parse_host("192.168.1.1").unwrap();
        assert!(matches!(host, Host::V4(_)));
    }

    #[test]
    fn test_parse_host_ipv6() {
        let host = parse_host("::1").unwrap();
        assert!(matches!(host, Host::V6(_)));
    }

    #[test]
    fn test_parse_host_dns() {
        let host = parse_host("mydevice.local").unwrap();
        assert!(matches!(host, Host::Dns(_)));
    }

    #[test]
    fn test_parse_host_empty_fails() {
        let result = parse_host("");
        assert!(result.is_err());
    }

    #[test]
    #[serial_test::serial]
    fn test_app_state_defaults() {
        // Clear env vars for this test
        // SAFETY: Tests run serially
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::remove_var("REQUEST_TIMEOUT_MS");
        }

        let state = AppState::from_env().unwrap();
        assert!(state.sony.is_none());
        assert!(state.arcam_host.is_none());
        assert_eq!(state.request_timeout, Duration::from_millis(5000));
    }

    #[tokio::test]
    async fn test_from_config_creates_receivers() {
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

        let state = AppState::from_config(config, None);

        // Check multi-device state
        let sony = state.get_sony("living-room").await;
        assert!(sony.is_some());

        let arcam = state.get_arcam_host("office").await;
        assert!(arcam.is_some());
        let (host, port) = arcam.unwrap();
        assert_eq!(host, "192.168.1.101");
        assert_eq!(port, 50000);
    }

    #[tokio::test]
    async fn test_get_nonexistent_device() {
        let state = AppState::from_config(HomeyConfig::new(), None);

        assert!(state.get_sony("nonexistent").await.is_none());
        assert!(state.get_arcam_host("nonexistent").await.is_none());
        assert!(state.get_eversolo("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_from_config_creates_eversolo_devices() {
        let mut config = HomeyConfig::new();
        config.eversolo_devices.insert(
            "living-room".to_string(),
            EversoloService {
                host: "192.168.1.50".to_string(),
                port: 9529,
            },
        );

        let state = AppState::from_config(config, None);
        let device = state.get_eversolo("living-room").await;
        assert!(device.is_some());
        let (host, port) = device.unwrap();
        assert_eq!(host, "192.168.1.50");
        assert_eq!(port, 9529);
    }
}
