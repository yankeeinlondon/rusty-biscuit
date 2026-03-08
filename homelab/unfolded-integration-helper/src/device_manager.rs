//! Multi-device runtime manager for UC integrations.
//!
//! [`DeviceManager`] owns the lifecycle of multiple [`DeviceDriver`] instances,
//! polling each independently and routing entity commands to the correct device.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::connectivity::{ConnectivityState, ConnectivityTracker};
use crate::persistent_registry::PersistentRegistry;
use crate::registry::ConfiguredDevice;
use crate::state_cache::{EntityState, StateCache};
use crate::subscriptions::SubscriptionRegistry;
use crate::{device_state_event, entity_change_event};

/// A command result containing entity updates to broadcast.
pub type CommandResult = Result<Vec<EntityUpdate>, DeviceError>;

/// An entity state update from a device driver.
#[derive(Debug, Clone)]
pub struct EntityUpdate {
    pub entity_id: String,
    pub entity_type: String,
    pub attributes: HashMap<String, Value>,
}

/// Errors from device driver operations.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("device not found: {0}")]
    NotFound(String),
    #[error("entity not found: {0}")]
    EntityNotFound(String),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("device communication error: {0}")]
    Communication(String),
    #[error("timeout")]
    Timeout,
}

impl DeviceError {
    /// Map to UC error code.
    #[must_use]
    pub fn uc_error_code(&self) -> u16 {
        match self {
            Self::NotFound(_) | Self::EntityNotFound(_) => 404,
            Self::UnknownCommand(_) => 400,
            Self::Communication(_) | Self::Timeout => 503,
        }
    }
}

/// Entity definition for registration.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Entity {
    pub entity_id: String,
    pub entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, Value>>,
}

/// Integration-specific device driver.
///
/// Each integration implements this trait to define how entities are built,
/// state is fetched, and commands are executed for a specific device type.
pub trait DeviceDriver: Send + Sync + 'static {
    /// Enrich a configured device with any live capability metadata needed to
    /// advertise truthful entities.
    fn prepare_device(
        &self,
        device: ConfiguredDevice,
        _timeout: Duration,
    ) -> impl std::future::Future<Output = Result<ConfiguredDevice, DeviceError>> + Send {
        async move { Ok(device) }
    }

    /// Build entities for this device.
    fn build_entities(&self, device: &ConfiguredDevice) -> Vec<Entity>;

    /// Build initial (unknown) states for this device's entities.
    fn build_initial_states(&self, device: &ConfiguredDevice) -> Vec<EntityState>;

    /// Fetch current device state.
    fn fetch_snapshot(
        &self,
        device: &ConfiguredDevice,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<Vec<EntityUpdate>, DeviceError>> + Send;

    /// Execute a command against the device.
    fn execute_command(
        &self,
        device: &ConfiguredDevice,
        entity_id: &str,
        cmd_id: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> impl std::future::Future<Output = CommandResult> + Send;
}

/// Per-device runtime state.
struct ActiveDevice<D: DeviceDriver> {
    config: ConfiguredDevice,
    driver: D,
    poll_handle: Option<JoinHandle<()>>,
}

/// Multi-device manager that owns device lifecycles and routes operations.
pub struct DeviceManager<D: DeviceDriver> {
    registry: PersistentRegistry,
    devices: Arc<RwLock<HashMap<String, ActiveDevice<D>>>>,
    connectivity: Arc<RwLock<ConnectivityTracker>>,
    states: Arc<RwLock<StateCache>>,
    subscriptions: SubscriptionRegistry,
    request_timeout: Duration,
    poll_interval: Duration,
}

impl<D: DeviceDriver + Clone> DeviceManager<D> {
    /// Create a new device manager.
    pub fn new(
        registry: PersistentRegistry,
        subscriptions: SubscriptionRegistry,
        request_timeout: Duration,
        poll_interval: Duration,
    ) -> Self {
        Self {
            registry,
            devices: Arc::new(RwLock::new(HashMap::new())),
            connectivity: Arc::new(RwLock::new(ConnectivityTracker::default())),
            states: Arc::new(RwLock::new(StateCache::default())),
            subscriptions,
            request_timeout,
            poll_interval,
        }
    }

    /// Add a device, create its driver, register entities, and start polling.
    pub async fn add_device(&self, config: ConfiguredDevice, driver: D) -> Result<(), DeviceError> {
        let config = driver.prepare_device(config, self.request_timeout).await?;
        let device_id = config.device_id.clone();

        // Register initial states
        {
            let initial = driver.build_initial_states(&config);
            let mut states = self.states.write().await;
            for state in initial {
                states.replace(state.entity_id, state.entity_type, state.attributes);
            }
        }

        // Set up connectivity tracking
        {
            let mut conn = self.connectivity.write().await;
            conn.set(&device_id, ConnectivityState::Unknown);
        }

        // Start polling
        let poll_handle = self.start_device_polling(config.clone(), driver.clone());

        let active = ActiveDevice {
            config,
            driver,
            poll_handle: Some(poll_handle),
        };

        if let Some(mut previous) = self.devices.write().await.insert(device_id, active) {
            if let Some(handle) = previous.poll_handle.take() {
                handle.abort();
            }
        }

        // Broadcast connectivity state
        self.broadcast_connectivity().await;

        Ok(())
    }

    /// Remove a device, stop its polling, and remove its entities.
    pub async fn remove_device(&self, device_id: &str) {
        let removed = self.devices.write().await.remove(device_id);

        if let Some(mut device) = removed {
            // Stop polling
            if let Some(handle) = device.poll_handle.take() {
                handle.abort();
            }

            // Remove entities from state cache
            let entity_ids: Vec<String> = device
                .driver
                .build_entities(&device.config)
                .into_iter()
                .map(|e| e.entity_id)
                .collect();

            let mut states = self.states.write().await;
            // StateCache doesn't have remove, so we rebuild without these entities
            let remaining: Vec<EntityState> = states
                .snapshot()
                .into_iter()
                .filter(|s| !entity_ids.contains(&s.entity_id))
                .collect();
            *states = StateCache::new(remaining);
        }

        // Update connectivity
        {
            let mut conn = self.connectivity.write().await;
            conn.set(device_id, ConnectivityState::Disconnected);
        }
        self.broadcast_connectivity().await;
    }

    /// Get all entities from all active devices.
    pub async fn get_all_entities(&self) -> Vec<Entity> {
        let devices = self.devices.read().await;
        devices
            .values()
            .flat_map(|d| d.driver.build_entities(&d.config))
            .collect()
    }

    /// Get entities assigned to a specific Remote.
    pub async fn get_entities_for_remote(&self, remote_id: &str) -> Vec<Entity> {
        let assigned_ids = self.assigned_device_ids(remote_id).await;
        let devices = self.devices.read().await;

        devices
            .iter()
            .filter(|(device_id, _)| assigned_ids.contains(device_id.as_str()))
            .flat_map(|(_, device)| device.driver.build_entities(&device.config))
            .collect()
    }

    /// Get all entity states.
    pub async fn get_all_states(&self) -> Vec<EntityState> {
        self.states.read().await.snapshot()
    }

    /// Get entity states assigned to a specific Remote.
    pub async fn get_states_for_remote(&self, remote_id: &str) -> Vec<EntityState> {
        let entity_ids = self.entity_ids_for_remote(remote_id).await;
        self.states
            .read()
            .await
            .snapshot()
            .into_iter()
            .filter(|state| entity_ids.contains(state.entity_id.as_str()))
            .collect()
    }

    /// Handle an entity command by routing to the correct device.
    pub async fn handle_entity_command(
        &self,
        entity_id: &str,
        cmd_id: &str,
        params: Option<Value>,
    ) -> CommandResult {
        let devices = self.devices.read().await;

        // Find the device that owns this entity
        let device = devices.values().find(|d| {
            d.driver
                .build_entities(&d.config)
                .iter()
                .any(|e| e.entity_id == entity_id)
        });

        let device = device.ok_or_else(|| DeviceError::EntityNotFound(entity_id.to_string()))?;

        let updates = device
            .driver
            .execute_command(
                &device.config,
                entity_id,
                cmd_id,
                params,
                self.request_timeout,
            )
            .await?;

        // Apply updates to state cache and broadcast
        self.apply_updates(&updates).await;

        Ok(updates)
    }

    /// Refresh a specific device on demand.
    pub async fn refresh_device(&self, device_id: &str) -> Result<(), DeviceError> {
        let devices = self.devices.read().await;
        let device = devices
            .get(device_id)
            .ok_or_else(|| DeviceError::NotFound(device_id.to_string()))?;

        match device
            .driver
            .fetch_snapshot(&device.config, self.request_timeout)
            .await
        {
            Ok(updates) => {
                drop(devices);
                self.apply_updates(&updates).await;
                self.mark_connected(device_id).await;
                Ok(())
            }
            Err(e) => {
                drop(devices);
                self.mark_disconnected(device_id).await;
                Err(e)
            }
        }
    }

    /// Refresh all devices.
    pub async fn refresh_all(&self) {
        let device_ids: Vec<String> = self.devices.read().await.keys().cloned().collect();
        for id in device_ids {
            let _ = self.refresh_device(&id).await;
        }
    }

    /// Refresh only devices assigned to a specific Remote.
    pub async fn refresh_remote(&self, remote_id: &str) {
        let device_ids: Vec<String> = self.assigned_device_ids(remote_id).await.into_iter().collect();
        for id in device_ids {
            let _ = self.refresh_device(&id).await;
        }
    }

    /// Ensure every device assigned to this Remote is active in the runtime.
    pub async fn ensure_assigned_devices_active(&self, remote_id: &str, driver: D) {
        let assigned_devices = self.registry.get_assigned_devices(remote_id).await;
        for config in assigned_devices {
            if !self.devices.read().await.contains_key(&config.device_id) {
                let _ = self.add_device(config, driver.clone()).await;
            }
        }
    }

    /// Get the shared connectivity tracker.
    pub fn connectivity(&self) -> &Arc<RwLock<ConnectivityTracker>> {
        &self.connectivity
    }

    /// Get the shared state cache.
    pub fn states(&self) -> &Arc<RwLock<StateCache>> {
        &self.states
    }

    /// Get the persistent registry.
    pub fn registry(&self) -> &PersistentRegistry {
        &self.registry
    }

    /// Get the subscription registry.
    pub fn subscriptions(&self) -> &SubscriptionRegistry {
        &self.subscriptions
    }

    fn start_device_polling(&self, config: ConfiguredDevice, driver: D) -> JoinHandle<()> {
        let states = Arc::clone(&self.states);
        let connectivity = Arc::clone(&self.connectivity);
        let subscriptions = self.subscriptions.clone();
        let timeout = self.request_timeout;
        let interval = self.poll_interval;
        let device_id = config.device_id.clone();

        tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            loop {
                tick.tick().await;

                match driver.fetch_snapshot(&config, timeout).await {
                    Ok(updates) => {
                        // Apply updates
                        let mut cache = states.write().await;
                        for update in &updates {
                            if let Some(changed) = cache.replace(
                                &update.entity_id,
                                &update.entity_type,
                                update.attributes.clone(),
                            ) {
                                subscriptions.broadcast_raw(entity_change_event(
                                    &changed.entity_id,
                                    &changed.entity_type,
                                    &changed.attributes,
                                )).await;
                            }
                        }
                        drop(cache);

                        // Mark connected
                        let mut conn = connectivity.write().await;
                        if conn.mark_connected(&device_id) {
                            let state = conn.overall();
                            subscriptions
                                .broadcast_raw(device_state_event(state.as_uc_state())).await;
                        }
                    }
                    Err(_) => {
                        let mut conn = connectivity.write().await;
                        if conn.mark_disconnected(&device_id) {
                            let state = conn.overall();
                            subscriptions
                                .broadcast_raw(device_state_event(state.as_uc_state())).await;
                        }
                    }
                }
            }
        })
    }

    async fn apply_updates(&self, updates: &[EntityUpdate]) {
        let mut cache = self.states.write().await;
        for update in updates {
            if let Some(changed) = cache.replace(
                &update.entity_id,
                &update.entity_type,
                update.attributes.clone(),
            ) {
                self.subscriptions.broadcast_raw(entity_change_event(
                    &changed.entity_id,
                    &changed.entity_type,
                    &changed.attributes,
                )).await;
            }
        }
    }

    async fn mark_connected(&self, device_id: &str) {
        let mut conn = self.connectivity.write().await;
        if conn.mark_connected(device_id) {
            let state = conn.overall();
            self.subscriptions
                .broadcast_raw(device_state_event(state.as_uc_state())).await;
        }
    }

    async fn mark_disconnected(&self, device_id: &str) {
        let mut conn = self.connectivity.write().await;
        if conn.mark_disconnected(device_id) {
            let state = conn.overall();
            self.subscriptions
                .broadcast_raw(device_state_event(state.as_uc_state())).await;
        }
    }

    async fn broadcast_connectivity(&self) {
        let conn = self.connectivity.read().await;
        let state = conn.overall();
        self.subscriptions
            .broadcast_raw(device_state_event(state.as_uc_state())).await;
    }

    async fn assigned_device_ids(&self, remote_id: &str) -> HashSet<String> {
        self.registry
            .get_assigned_devices(remote_id)
            .await
            .into_iter()
            .map(|device| device.device_id)
            .collect()
    }

    async fn entity_ids_for_remote(&self, remote_id: &str) -> HashSet<String> {
        let assigned_ids = self.assigned_device_ids(remote_id).await;
        let devices = self.devices.read().await;

        devices
            .iter()
            .filter(|(device_id, _)| assigned_ids.contains(device_id.as_str()))
            .flat_map(|(_, device)| {
                device
                    .driver
                    .build_entities(&device.config)
                    .into_iter()
                    .map(|entity| entity.entity_id)
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}
