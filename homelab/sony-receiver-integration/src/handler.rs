//! UC Integration WebSocket handler for Sony receivers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use schematic_schema::unfolded_circle_integration_ws::WsHandler;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::dispatch::{execute_operation, fetch_receiver_state};
use crate::error::SonyIntegrationError;
use crate::responses::{
    available_entities_response, device_state_event, driver_version_response, entity_change_event,
    entity_states_response, result_response,
};
use crate::types::{
    self, SonyEntity, SonyEntityState, device_name_from_entity_id, resolve_command,
};

/// Sony receiver integration handler implementing the UC WsHandler trait.
pub struct SonyIntegrationHandler {
    /// Sony device configs: name -> (host, port)
    devices: HashMap<String, (String, u16)>,
    /// Entity registry
    entities: Vec<SonyEntity>,
    /// Cached entity states
    states: Arc<RwLock<Vec<SonyEntityState>>>,
    /// Request timeout for Sony HTTP operations
    request_timeout: Duration,
}

impl SonyIntegrationHandler {
    pub fn new(devices: HashMap<String, (String, u16)>, request_timeout: Duration) -> Self {
        let mut entities = Vec::new();
        let mut states = Vec::new();

        for name in devices.keys() {
            entities.extend(types::build_entities(name));
            states.extend(types::build_initial_states(name));
        }

        Self {
            devices,
            entities,
            states: Arc::new(RwLock::new(states)),
            request_timeout,
        }
    }

    async fn handle_entity_command(&self, req_id: u64, msg_data: &Value) -> Option<Value> {
        let entity_id = msg_data.get("entity_id")?.as_str()?;
        let cmd_id = msg_data.get("cmd_id")?.as_str()?;
        let params = msg_data.get("params").cloned().unwrap_or_default();

        let operation = match resolve_command(entity_id, cmd_id, &params) {
            Some(op) => op,
            None => {
                warn!(entity_id, cmd_id, "unknown entity command");
                let err = if self.entities.iter().any(|e| e.entity_id == entity_id) {
                    SonyIntegrationError::UnknownCommand(cmd_id.to_string())
                } else {
                    SonyIntegrationError::UnknownEntity(entity_id.to_string())
                };
                return Some(result_response(req_id, err.uc_error_code()));
            }
        };

        let device_name = match device_name_from_entity_id(entity_id) {
            Some(n) => n,
            None => return Some(result_response(req_id, 404)),
        };

        let (host, port) = match self.devices.get(device_name) {
            Some(hp) => hp,
            None => return Some(result_response(req_id, 404)),
        };

        match execute_operation(host, *port, operation, self.request_timeout).await {
            Ok((entity_kind, attrs)) => {
                // Determine the full entity_id and type for the affected entity
                let affected_entity_id = format!("sony.{device_name}.{entity_kind}");
                let entity_type = match entity_kind {
                    "power" => "switch",
                    "receiver" => "media_player",
                    _ => "switch",
                };

                // Update cached state
                let mut states = self.states.write().await;
                if let Some(state) = states
                    .iter_mut()
                    .find(|s| s.entity_id == affected_entity_id)
                {
                    for (k, v) in &attrs {
                        state.attributes.insert(k.clone(), v.clone());
                    }
                }
                drop(states);

                debug!(entity_id, cmd_id, "entity command executed");

                Some(entity_change_event(
                    &affected_entity_id,
                    entity_type,
                    &attrs,
                ))
            }
            Err(e) => {
                warn!(entity_id, cmd_id, error = %e, "entity command failed");
                Some(result_response(req_id, e.uc_error_code()))
            }
        }
    }

    async fn handle_get_entity_states(&self, req_id: u64) -> Value {
        // Try to fetch fresh state from each device
        for (name, (host, port)) in &self.devices {
            if let Ok(attrs) = fetch_receiver_state(host, *port, self.request_timeout).await {
                let mut states = self.states.write().await;

                // Update power entity
                let power_id = format!("sony.{name}.power");
                if let Some(state) = states.iter_mut().find(|s| s.entity_id == power_id)
                    && let Some(power_state) = attrs.get("state")
                {
                    state
                        .attributes
                        .insert("state".to_string(), power_state.clone());
                }

                // Update receiver entity
                let receiver_id = format!("sony.{name}.receiver");
                if let Some(state) = states.iter_mut().find(|s| s.entity_id == receiver_id) {
                    for (k, v) in &attrs {
                        state.attributes.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        let states = self.states.read().await;
        entity_states_response(req_id, &states)
    }
}

impl WsHandler for SonyIntegrationHandler {
    async fn handle_message(&self, message: Value) -> Option<Value> {
        let kind = message.get("kind")?.as_str()?;
        let msg = message.get("msg")?.as_str()?;
        let req_id = message.get("req_id").and_then(|v| v.as_u64()).unwrap_or(0);

        if kind != "req" {
            debug!(kind, msg, "ignoring non-request message");
            return None;
        }

        debug!(msg, req_id, "handling request");

        match msg {
            "get_driver_version" => Some(driver_version_response(req_id)),

            "get_device_state" => {
                let state = if self.devices.is_empty() {
                    "DISCONNECTED"
                } else {
                    "CONNECTED"
                };
                Some(device_state_event(state))
            }

            "get_available_entities" => Some(available_entities_response(req_id, &self.entities)),

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn handler_no_devices() -> SonyIntegrationHandler {
        SonyIntegrationHandler::new(HashMap::new(), Duration::from_secs(5))
    }

    fn handler_with_device() -> SonyIntegrationHandler {
        let mut devices = HashMap::new();
        devices.insert("living".to_string(), ("192.168.99.99".to_string(), 10000));
        SonyIntegrationHandler::new(devices, Duration::from_secs(5))
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
            .await;
        let resp = resp.unwrap();
        assert_eq!(resp["kind"], "resp");
        assert_eq!(resp["req_id"], 1);
        assert_eq!(resp["msg"], "driver_version");
        assert_eq!(resp["msg_data"]["name"], "sony-receiver");
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
            .await;
        let resp = resp.unwrap();
        assert_eq!(resp["msg_data"]["state"], "DISCONNECTED");
    }

    #[tokio::test]
    async fn test_handle_get_device_state_with_devices() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 2,
                "msg": "get_device_state"
            }))
            .await;
        let resp = resp.unwrap();
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
            .await;
        let resp = resp.unwrap();
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
            .await;
        let resp = resp.unwrap();
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
            .await;
        let resp = resp.unwrap();
        assert_eq!(resp["msg"], "entity_states");
        // Returns cached state (may fail to fetch from fake device, falls back to initial)
        let states = resp["msg_data"].as_array().unwrap();
        assert_eq!(states.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_unknown_message() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 6,
                "msg": "bogus_message"
            }))
            .await;
        let resp = resp.unwrap();
        assert_eq!(resp["msg_data"]["code"], 400);
    }

    #[tokio::test]
    async fn test_handle_non_req_message() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(json!({
                "kind": "event",
                "msg": "something"
            }))
            .await;
        assert!(resp.is_none());
    }

    #[tokio::test]
    async fn test_entity_command_unknown_entity() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 7,
                "msg": "entity_command",
                "msg_data": {
                    "entity_id": "sony.living.unknown",
                    "cmd_id": "on"
                }
            }))
            .await;
        let resp = resp.unwrap();
        assert_eq!(resp["msg_data"]["code"], 404);
    }

    #[tokio::test]
    async fn test_entity_command_unknown_cmd() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(json!({
                "kind": "req",
                "req_id": 8,
                "msg": "entity_command",
                "msg_data": {
                    "entity_id": "sony.living.power",
                    "cmd_id": "invalid"
                }
            }))
            .await;
        let resp = resp.unwrap();
        assert_eq!(resp["msg_data"]["code"], 400);
    }
}
