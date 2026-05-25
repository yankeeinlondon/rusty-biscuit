//! Persistent device registry backed by a JSON file.
//!
//! Provides thread-safe CRUD operations for known devices, configured devices,
//! and remote assignments. Writes are atomic (write-to-tmp then rename).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::RwLock;

use crate::registry::{
    ConfiguredDevice, DiscoverySource, KnownDevice, RegistryData, RemoteAssignment,
};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("device {0} already assigned to remote {1}")]
    AlreadyAssigned(String, String),
    #[error("device {0} not found")]
    DeviceNotFound(String),
}

/// Thread-safe persistent registry for device and remote state.
#[derive(Debug, Clone)]
pub struct PersistentRegistry {
    inner: Arc<RwLock<RegistryInner>>,
}

#[derive(Debug)]
struct RegistryInner {
    path: PathBuf,
    data: RegistryData,
}

impl PersistentRegistry {
    /// Load registry from `path`, returning an empty registry if the file is missing.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub async fn load(path: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        let path = path.into();
        let data = if path.exists() {
            let contents = fs::read_to_string(&path)?;
            serde_json::from_str(&contents)?
        } else {
            RegistryData::default()
        };

        Ok(Self {
            inner: Arc::new(RwLock::new(RegistryInner { path, data })),
        })
    }

    /// Resolve the default registry path for a given driver ID.
    ///
    /// Uses `$XDG_DATA_HOME/<driver_id>/registry.json` on desktop or
    /// `/data/<driver_id>/registry.json` inside Docker.
    #[must_use]
    pub fn default_path(driver_id: &str) -> PathBuf {
        // Docker convention: /data exists and is writable
        let docker_data = Path::new("/data");
        if docker_data.exists() && docker_data.is_dir() {
            return docker_data.join(driver_id).join("registry.json");
        }

        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(driver_id)
            .join("registry.json")
    }

    /// Save registry to disk atomically (write to tmp, then rename).
    ///
    /// ## Errors
    ///
    /// Returns an error if the write or rename fails.
    pub async fn save(&self) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        let json = serde_json::to_string_pretty(&inner.data)?;

        if let Some(parent) = inner.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let tmp_path = inner.path.with_extension("json.tmp");
        fs::write(&tmp_path, json.as_bytes())?;
        fs::rename(&tmp_path, &inner.path)?;

        Ok(())
    }

    /// Upsert a known device by `device_id`.
    pub async fn add_known_device(&self, device: KnownDevice) {
        let mut inner = self.inner.write().await;
        if let Some(existing) = inner
            .data
            .known_devices
            .iter_mut()
            .find(|d| d.device_id == device.device_id)
        {
            *existing = device;
        } else {
            inner.data.known_devices.push(device);
        }
    }

    /// Remove a known device by `device_id`.
    pub async fn remove_known_device(&self, device_id: &str) {
        let mut inner = self.inner.write().await;
        inner
            .data
            .known_devices
            .retain(|d| d.device_id != device_id);
    }

    /// Upsert a configured device by `device_id`.
    pub async fn add_configured_device(&self, device: ConfiguredDevice) {
        let mut inner = self.inner.write().await;
        if let Some(existing) = inner
            .data
            .configured_devices
            .iter_mut()
            .find(|d| d.device_id == device.device_id)
        {
            *existing = device;
        } else {
            inner.data.configured_devices.push(device);
        }
    }

    /// Remove a configured device and cascade-remove it from all assignments.
    pub async fn remove_configured_device(&self, device_id: &str) {
        let mut inner = self.inner.write().await;
        inner
            .data
            .configured_devices
            .retain(|d| d.device_id != device_id);

        // Cascade: remove from all assignments
        for assignment in &mut inner.data.assignments {
            assignment.device_ids.retain(|id| id != device_id);
        }
        // Remove empty assignments
        inner.data.assignments.retain(|a| !a.device_ids.is_empty());
    }

    /// Assign a device to a Remote.
    ///
    /// ## Errors
    ///
    /// Returns `AlreadyAssigned` if this device is already assigned to this Remote.
    pub async fn assign_device(
        &self,
        remote_id: &str,
        device_id: &str,
    ) -> Result<(), RegistryError> {
        let mut inner = self.inner.write().await;

        if let Some(assignment) = inner
            .data
            .assignments
            .iter()
            .find(|a| a.remote_id == remote_id)
            && assignment.device_ids.iter().any(|id| id == device_id)
        {
            return Err(RegistryError::AlreadyAssigned(
                device_id.to_string(),
                remote_id.to_string(),
            ));
        }

        if let Some(assignment) = inner
            .data
            .assignments
            .iter_mut()
            .find(|a| a.remote_id == remote_id)
        {
            assignment.device_ids.push(device_id.to_string());
        } else {
            inner.data.assignments.push(RemoteAssignment {
                remote_id: remote_id.to_string(),
                device_ids: vec![device_id.to_string()],
            });
        }

        Ok(())
    }

    /// Unassign a device from a Remote.
    pub async fn unassign_device(&self, remote_id: &str, device_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(assignment) = inner
            .data
            .assignments
            .iter_mut()
            .find(|a| a.remote_id == remote_id)
        {
            assignment.device_ids.retain(|id| id != device_id);
        }
        // Clean up empty assignments
        inner.data.assignments.retain(|a| !a.device_ids.is_empty());
    }

    /// Get all configured devices.
    pub async fn get_configured_devices(&self) -> Vec<ConfiguredDevice> {
        let inner = self.inner.read().await;
        inner.data.configured_devices.clone()
    }

    /// Get configured devices assigned to a specific Remote.
    pub async fn get_assigned_devices(&self, remote_id: &str) -> Vec<ConfiguredDevice> {
        let inner = self.inner.read().await;
        let assigned_ids: Vec<&str> = inner
            .data
            .assignments
            .iter()
            .filter(|a| a.remote_id == remote_id)
            .flat_map(|a| a.device_ids.iter().map(String::as_str))
            .collect();

        inner
            .data
            .configured_devices
            .iter()
            .filter(|d| assigned_ids.contains(&d.device_id.as_str()))
            .cloned()
            .collect()
    }

    /// Check if a device is assigned to a Remote.
    pub async fn is_assigned(&self, remote_id: &str, device_id: &str) -> bool {
        let inner = self.inner.read().await;
        inner
            .data
            .assignments
            .iter()
            .any(|a| a.remote_id == remote_id && a.device_ids.iter().any(|id| id == device_id))
    }

    /// Get all known devices.
    pub async fn get_known_devices(&self) -> Vec<KnownDevice> {
        let inner = self.inner.read().await;
        inner.data.known_devices.clone()
    }

    /// Get all remote assignments.
    pub async fn get_assignments(&self) -> Vec<RemoteAssignment> {
        let inner = self.inner.read().await;
        inner.data.assignments.clone()
    }

    /// Create a seed entry from a CLI `--host` hint.
    ///
    /// If no device with this host exists in the registry, creates both a
    /// known device and a configured device from the hint.
    pub async fn seed_from_cli_hint(
        &self,
        host: &str,
        port: u16,
        device_name: &str,
    ) -> ConfiguredDevice {
        let inner = self.inner.read().await;

        // Check if already known by host
        if let Some(existing) = inner
            .data
            .configured_devices
            .iter()
            .find(|d| d.host == host && d.port == port)
        {
            return existing.clone();
        }
        drop(inner);

        let device_id = format!("{host}:{port}");

        let known = KnownDevice {
            device_id: device_id.clone(),
            source: DiscoverySource::CliHint,
            host: host.to_string(),
            port,
            metadata: Default::default(),
            last_validated: None,
        };

        let configured = ConfiguredDevice {
            device_id: device_id.clone(),
            device_name: device_name.to_string(),
            host: host.to_string(),
            port,
            metadata: Default::default(),
            driver_config: Default::default(),
        };

        self.add_known_device(known).await;
        self.add_configured_device(configured.clone()).await;

        configured
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn registry_in(dir: &Path) -> PersistentRegistry {
        let path = dir.join("registry.json");
        PersistentRegistry::load(path).await.unwrap()
    }

    fn sample_configured(id: &str, name: &str) -> ConfiguredDevice {
        ConfiguredDevice {
            device_id: id.to_string(),
            device_name: name.to_string(),
            host: "192.168.1.100".to_string(),
            port: 9529,
            metadata: Default::default(),
            driver_config: Default::default(),
        }
    }

    fn sample_known(id: &str) -> KnownDevice {
        KnownDevice {
            device_id: id.to_string(),
            source: DiscoverySource::CliHint,
            host: "192.168.1.100".to_string(),
            port: 9529,
            metadata: Default::default(),
            last_validated: None,
        }
    }

    #[tokio::test]
    async fn empty_registry_loads_cleanly() {
        let tmp = TempDir::new().unwrap();
        let reg = registry_in(tmp.path()).await;
        assert!(reg.get_configured_devices().await.is_empty());
        assert!(reg.get_known_devices().await.is_empty());
    }

    #[tokio::test]
    async fn round_trip_save_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("registry.json");

        let reg = PersistentRegistry::load(&path).await.unwrap();
        reg.add_known_device(sample_known("dev1")).await;
        reg.add_configured_device(sample_configured("dev1", "streamer"))
            .await;
        reg.assign_device("remote-1", "dev1").await.unwrap();
        reg.save().await.unwrap();

        let reg2 = PersistentRegistry::load(&path).await.unwrap();
        let devices = reg2.get_configured_devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_name, "streamer");

        let known = reg2.get_known_devices().await;
        assert_eq!(known.len(), 1);

        assert!(reg2.is_assigned("remote-1", "dev1").await);
    }

    #[tokio::test]
    async fn duplicate_assignment_fails() {
        let tmp = TempDir::new().unwrap();
        let reg = registry_in(tmp.path()).await;

        reg.add_configured_device(sample_configured("dev1", "amp"))
            .await;
        reg.assign_device("remote-1", "dev1").await.unwrap();

        let result = reg.assign_device("remote-1", "dev1").await;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), RegistryError::AlreadyAssigned(d, r) if d == "dev1" && r == "remote-1")
        );
    }

    #[tokio::test]
    async fn remove_configured_cascades_to_assignments() {
        let tmp = TempDir::new().unwrap();
        let reg = registry_in(tmp.path()).await;

        reg.add_configured_device(sample_configured("dev1", "amp"))
            .await;
        reg.add_configured_device(sample_configured("dev2", "streamer"))
            .await;
        reg.assign_device("remote-1", "dev1").await.unwrap();
        reg.assign_device("remote-1", "dev2").await.unwrap();

        reg.remove_configured_device("dev1").await;

        assert!(!reg.is_assigned("remote-1", "dev1").await);
        assert!(reg.is_assigned("remote-1", "dev2").await);
        assert_eq!(reg.get_configured_devices().await.len(), 1);
    }

    #[tokio::test]
    async fn upsert_known_device() {
        let tmp = TempDir::new().unwrap();
        let reg = registry_in(tmp.path()).await;

        let mut dev = sample_known("dev1");
        reg.add_known_device(dev.clone()).await;
        assert_eq!(reg.get_known_devices().await.len(), 1);

        dev.host = "10.0.0.50".to_string();
        reg.add_known_device(dev).await;
        let known = reg.get_known_devices().await;
        assert_eq!(known.len(), 1);
        assert_eq!(known[0].host, "10.0.0.50");
    }

    #[tokio::test]
    async fn atomic_write_does_not_corrupt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("registry.json");

        // Save initial state
        let reg = PersistentRegistry::load(&path).await.unwrap();
        reg.add_configured_device(sample_configured("dev1", "amp"))
            .await;
        reg.save().await.unwrap();

        // Verify file is valid JSON
        let contents = std::fs::read_to_string(&path).unwrap();
        let _: RegistryData = serde_json::from_str(&contents).unwrap();

        // No .tmp file should remain
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[tokio::test]
    async fn seed_from_cli_hint_creates_entries() {
        let tmp = TempDir::new().unwrap();
        let reg = registry_in(tmp.path()).await;

        let device = reg.seed_from_cli_hint("192.168.1.50", 50000, "amp").await;
        assert_eq!(device.device_name, "amp");
        assert_eq!(device.host, "192.168.1.50");
        assert_eq!(device.port, 50000);

        assert_eq!(reg.get_known_devices().await.len(), 1);
        assert_eq!(reg.get_configured_devices().await.len(), 1);

        // Calling again returns existing
        let device2 = reg.seed_from_cli_hint("192.168.1.50", 50000, "other").await;
        assert_eq!(device2.device_name, "amp"); // Original name preserved
        assert_eq!(reg.get_configured_devices().await.len(), 1);
    }

    #[tokio::test]
    async fn get_assigned_devices_filters_correctly() {
        let tmp = TempDir::new().unwrap();
        let reg = registry_in(tmp.path()).await;

        reg.add_configured_device(sample_configured("dev1", "amp"))
            .await;
        reg.add_configured_device(sample_configured("dev2", "streamer"))
            .await;
        reg.assign_device("remote-1", "dev1").await.unwrap();
        reg.assign_device("remote-2", "dev2").await.unwrap();

        let r1_devices = reg.get_assigned_devices("remote-1").await;
        assert_eq!(r1_devices.len(), 1);
        assert_eq!(r1_devices[0].device_name, "amp");

        let r2_devices = reg.get_assigned_devices("remote-2").await;
        assert_eq!(r2_devices.len(), 1);
        assert_eq!(r2_devices[0].device_name, "streamer");
    }

    #[tokio::test]
    async fn unassign_device_cleans_up() {
        let tmp = TempDir::new().unwrap();
        let reg = registry_in(tmp.path()).await;

        reg.add_configured_device(sample_configured("dev1", "amp"))
            .await;
        reg.assign_device("remote-1", "dev1").await.unwrap();
        assert!(reg.is_assigned("remote-1", "dev1").await);

        reg.unassign_device("remote-1", "dev1").await;
        assert!(!reg.is_assigned("remote-1", "dev1").await);
    }
}
