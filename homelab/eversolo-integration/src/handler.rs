//! UC Integration WebSocket handler for Eversolo streamers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use schematic_schema::unfolded_circle_integration_ws::WsHandler;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::dispatch::{EntityCatalog, execute_operation, fetch_snapshot};
use crate::error::EversoloIntegrationError;
use crate::responses::{
    available_entities_response, device_state_event, driver_version_response, entity_change_event,
    entity_states_response, result_response,
};
use crate::types::{
    self, DEFAULT_VOLUME_STEPS, DeviceConfig, EversoloEntityState, device_name_from_entity_id,
    entity_exists, resolve_command,
};

/// Eversolo integration handler implementing the UC WsHandler trait.
pub struct EversoloIntegrationHandler {
    /// Eversolo device configs keyed by UC device name.
    devices: HashMap<String, DeviceConfig>,
    /// Cached entity states.
    states: Arc<RwLock<Vec<EversoloEntityState>>>,
    /// Cached entity catalog metadata such as dynamic source lists.
    catalogs: Arc<RwLock<HashMap<String, EntityCatalog>>>,
    /// Device reachability keyed by configured device name.
    connectivity: Arc<RwLock<HashMap<String, bool>>>,
    /// Request timeout for Eversolo operations.
    request_timeout: Duration,
}

impl EversoloIntegrationHandler {
    pub fn new(devices: HashMap<String, DeviceConfig>, request_timeout: Duration) -> Self {
        let mut states = Vec::new();
        for name in devices.keys() {
            states.extend(types::build_initial_states(name));
        }

        Self {
            devices,
            states: Arc::new(RwLock::new(states)),
            catalogs: Arc::new(RwLock::new(HashMap::new())),
            connectivity: Arc::new(RwLock::new(HashMap::new())),
            request_timeout,
        }
    }

    pub fn start_polling(self: &Arc<Self>, interval: Duration) {
        for (name, config) in self.devices.clone() {
            let handler = Arc::clone(self);
            tokio::spawn(async move {
                loop {
                    handler.refresh_device(&name, &config).await;
                    tokio::time::sleep(interval).await;
                }
            });
        }
    }

    pub async fn refresh_all(&self) {
        for (name, config) in &self.devices {
            self.refresh_device(name, config).await;
        }
    }

    async fn refresh_device(&self, name: &str, config: &DeviceConfig) {
        match fetch_snapshot(config, self.request_timeout).await {
            Ok(snapshot) => {
                self.catalogs
                    .write()
                    .await
                    .insert(name.to_string(), snapshot.catalog.clone());
                self.connectivity
                    .write()
                    .await
                    .insert(name.to_string(), true);

                let mut states = self.states.write().await;
                self.replace_entity_state(
                    &mut states,
                    &format!("eversolo.{name}.power"),
                    snapshot.power_attributes,
                );
                self.replace_entity_state(
                    &mut states,
                    &format!("eversolo.{name}.player"),
                    snapshot.player_attributes,
                );
            }
            Err(error) => {
                debug!(device_name = name, error = %error, "poll refresh failed");
                self.connectivity
                    .write()
                    .await
                    .insert(name.to_string(), false);

                let mut states = self.states.write().await;
                self.replace_entity_state(
                    &mut states,
                    &format!("eversolo.{name}.power"),
                    offline_power_attributes(),
                );
                self.replace_entity_state(
                    &mut states,
                    &format!("eversolo.{name}.player"),
                    offline_player_attributes(),
                );
            }
        }
    }

    fn replace_entity_state(
        &self,
        states: &mut [EversoloEntityState],
        entity_id: &str,
        attributes: HashMap<String, Value>,
    ) {
        if let Some(state) = states.iter_mut().find(|state| state.entity_id == entity_id) {
            state.attributes = attributes;
        }
    }

    async fn handle_get_device_state(&self) -> &'static str {
        if self.devices.is_empty() {
            return "DISCONNECTED";
        }

        if self
            .connectivity
            .read()
            .await
            .values()
            .copied()
            .any(|connected| connected)
        {
            "CONNECTED"
        } else {
            "DISCONNECTED"
        }
    }

    async fn handle_get_available_entities(&self, req_id: u64) -> Value {
        let mut entities = Vec::new();
        let catalogs = self.catalogs.read().await;

        for name in self.devices.keys() {
            let catalog = catalogs.get(name);
            let source_list = catalog
                .map(|catalog| catalog.source_list.as_slice())
                .unwrap_or(&[]);
            let volume_steps = catalog
                .map(|catalog| catalog.volume_steps)
                .unwrap_or(DEFAULT_VOLUME_STEPS);
            entities.extend(types::build_entities(name, source_list, volume_steps));
        }

        available_entities_response(req_id, &entities)
    }

    async fn handle_get_entity_states(&self, req_id: u64) -> Value {
        let states = self.states.read().await;
        entity_states_response(req_id, &states)
    }

    async fn handle_entity_command(&self, req_id: u64, msg_data: &Value) -> Option<Value> {
        let entity_id = msg_data.get("entity_id")?.as_str()?;
        let cmd_id = msg_data.get("cmd_id")?.as_str()?;
        let params = msg_data.get("params").cloned().unwrap_or_default();

        let operation = match resolve_command(entity_id, cmd_id, &params) {
            Some(operation) => operation,
            None => {
                warn!(entity_id, cmd_id, "unknown entity command");
                let error = if entity_exists(entity_id) {
                    EversoloIntegrationError::UnknownCommand(cmd_id.to_string())
                } else {
                    EversoloIntegrationError::UnknownEntity(entity_id.to_string())
                };
                return Some(result_response(req_id, error.uc_error_code()));
            }
        };

        let device_name = match device_name_from_entity_id(entity_id) {
            Some(name) => name,
            None => return Some(result_response(req_id, 404)),
        };

        let config = match self.devices.get(device_name) {
            Some(config) => config,
            None => return Some(result_response(req_id, 404)),
        };

        match execute_operation(config, operation, self.request_timeout).await {
            Ok(updates) => {
                let target_kind = entity_id.rsplit('.').next()?;

                let mut observed_connected = None;
                let mut states = self.states.write().await;
                for update in &updates {
                    let full_entity_id = format!("eversolo.{device_name}.{}", update.entity_kind);
                    if let Some(state) = states
                        .iter_mut()
                        .find(|state| state.entity_id == full_entity_id)
                    {
                        for (key, value) in &update.attributes {
                            state.attributes.insert(key.clone(), value.clone());
                        }
                    }

                    if update.entity_kind == "power"
                        && let Some(state) = update.attributes.get("state").and_then(|v| v.as_str())
                    {
                        observed_connected = Some(state != "OFF");
                    }
                }
                drop(states);
                if let Some(connected) = observed_connected {
                    self.connectivity
                        .write()
                        .await
                        .insert(device_name.to_string(), connected);
                }

                let selected = updates
                    .iter()
                    .find(|update| update.entity_kind == target_kind)
                    .or_else(|| updates.first())?;

                Some(entity_change_event(
                    entity_id,
                    selected.entity_type,
                    &selected.attributes,
                ))
            }
            Err(error) => {
                warn!(entity_id, cmd_id, error = %error, "entity command failed");
                Some(result_response(req_id, error.uc_error_code()))
            }
        }
    }
}

impl WsHandler for EversoloIntegrationHandler {
    async fn handle_message(&self, message: Value) -> Option<Value> {
        let kind = message.get("kind")?.as_str()?;
        let msg = message.get("msg")?.as_str()?;
        let req_id = message
            .get("req_id")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);

        if kind != "req" {
            debug!(kind, msg, "ignoring non-request message");
            return None;
        }

        debug!(msg, req_id, "handling request");

        match msg {
            "get_driver_version" => Some(driver_version_response(req_id)),
            "get_device_state" => Some(device_state_event(self.handle_get_device_state().await)),
            "get_available_entities" => Some(self.handle_get_available_entities(req_id).await),
            "subscribe_events" => Some(result_response(req_id, 200)),
            "get_entity_states" => Some(self.handle_get_entity_states(req_id).await),
            "entity_command" => {
                let msg_data = message.get("msg_data").cloned().unwrap_or_default();
                self.handle_entity_command(req_id, &msg_data).await
            }
            _ => {
                warn!(msg, "unknown message type");
                Some(result_response(req_id, 400))
            }
        }
    }
}

fn offline_power_attributes() -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!("OFF"))])
}

fn offline_player_attributes() -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!("OFF"))])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn handler_no_devices() -> EversoloIntegrationHandler {
        EversoloIntegrationHandler::new(HashMap::new(), Duration::from_millis(50))
    }

    fn handler_with_device() -> EversoloIntegrationHandler {
        let mut devices = HashMap::new();
        devices.insert(
            "living".to_string(),
            DeviceConfig {
                host: "127.0.0.1".to_string(),
                port: 9,
                mac_address: None,
                wol_broadcast: "255.255.255.255".to_string(),
                wol_port: 9517,
            },
        );
        EversoloIntegrationHandler::new(devices, Duration::from_millis(50))
    }

    #[tokio::test]
    async fn test_handle_get_driver_version() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 1,
                "msg": "get_driver_version"
            }))
            .await
            .unwrap();
        assert_eq!(resp["kind"], "resp");
        assert_eq!(resp["req_id"], 1);
        assert_eq!(resp["msg"], "driver_version");
    }

    #[tokio::test]
    async fn test_handle_get_device_state_no_devices() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 2,
                "msg": "get_device_state"
            }))
            .await
            .unwrap();
        assert_eq!(resp["msg_data"]["state"], "DISCONNECTED");
    }

    #[tokio::test]
    async fn test_handle_get_device_state_uses_cached_connectivity() {
        let handler = handler_with_device();
        handler
            .connectivity
            .write()
            .await
            .insert("living".to_string(), true);

        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 2,
                "msg": "get_device_state"
            }))
            .await
            .unwrap();
        assert_eq!(resp["msg_data"]["state"], "CONNECTED");
    }

    #[tokio::test]
    async fn test_handle_get_available_entities() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 3,
                "msg": "get_available_entities"
            }))
            .await
            .unwrap();
        let entities = resp["msg_data"]["available_entities"].as_array().unwrap();
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0]["entity_type"], "switch");
        assert_eq!(entities[1]["entity_type"], "media_player");
    }

    #[tokio::test]
    async fn test_handle_subscribe_events() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 4,
                "msg": "subscribe_events"
            }))
            .await
            .unwrap();
        assert_eq!(resp["msg_data"]["code"], 200);
    }

    #[tokio::test]
    async fn test_handle_get_entity_states() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 5,
                "msg": "get_entity_states"
            }))
            .await
            .unwrap();
        assert_eq!(resp["msg"], "entity_states");
        let states = resp["msg_data"].as_array().unwrap();
        assert_eq!(states.len(), 2);
    }

    #[test]
    fn test_offline_attributes() {
        assert_eq!(offline_power_attributes()["state"], "OFF");
        assert_eq!(offline_player_attributes()["state"], "OFF");
    }

    #[tokio::test]
    async fn test_handle_entity_command_missing_mac_returns_400() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 6,
                "msg": "entity_command",
                "msg_data": {
                    "entity_id": "eversolo.living.power",
                    "cmd_id": "on"
                }
            }))
            .await
            .unwrap();
        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["msg_data"]["code"], 400);
    }

    #[tokio::test]
    async fn test_handle_unknown_message() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 7,
                "msg": "bogus_message"
            }))
            .await
            .unwrap();
        assert_eq!(resp["msg_data"]["code"], 400);
    }

    #[tokio::test]
    async fn test_handle_non_request_message() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(json!({
                "kind": "event",
                "msg": "something"
            }))
            .await;
        assert!(resp.is_none());
    }
}
