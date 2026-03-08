//! UC Integration WebSocket handler for Eversolo streamers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use schematic_schema::unfolded_circle_integration_ws::{
    UnfoldedCircleEventHub, WsConnectionContext, WsHandler,
};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use unfolded_integration_helper::{
    ConnectivityState, ConnectivityTracker, IntegrationRequest, StateCache, SubscriptionRegistry,
    available_entities_response, device_state_event, driver_metadata_response,
    driver_version_response, entity_states_response, result_response,
};

use crate::dispatch::{
    EntityCatalog, execute_operation, fetch_snapshot, offline_player_attributes,
    offline_power_attributes,
};
use crate::error::EversoloIntegrationError;
use crate::types::{
    self, DEFAULT_VOLUME_STEPS, DeviceConfig, device_name_from_entity_id, entity_exists,
    resolve_command,
};

/// Eversolo integration handler implementing the UC WsHandler trait.
pub struct EversoloIntegrationHandler {
    /// Eversolo device configs keyed by UC device name.
    devices: HashMap<String, DeviceConfig>,
    /// Cached entity states.
    states: Arc<RwLock<StateCache>>,
    /// Cached entity catalog metadata such as dynamic source lists.
    catalogs: Arc<RwLock<HashMap<String, EntityCatalog>>>,
    /// Device reachability keyed by configured device name.
    connectivity: Arc<RwLock<ConnectivityTracker>>,
    /// Shared subscription bridge.
    subscriptions: SubscriptionRegistry,
    /// Request timeout for Eversolo operations.
    request_timeout: Duration,
}

impl EversoloIntegrationHandler {
    pub fn new(
        devices: HashMap<String, DeviceConfig>,
        request_timeout: Duration,
        hub: UnfoldedCircleEventHub,
    ) -> Self {
        let mut states = Vec::new();
        for name in devices.keys() {
            states.extend(types::build_initial_states(name));
        }

        Self {
            devices,
            states: Arc::new(RwLock::new(StateCache::new(states))),
            catalogs: Arc::new(RwLock::new(HashMap::new())),
            connectivity: Arc::new(RwLock::new(ConnectivityTracker::default())),
            subscriptions: SubscriptionRegistry::new(hub),
            request_timeout,
        }
    }

    pub fn start_polling(self: &Arc<Self>, interval: Duration) {
        for (name, config) in self.devices.clone() {
            let handler = Arc::clone(self);
            tokio::spawn(async move {
                loop {
                    handler.refresh_device(&name, &config, true).await;
                    tokio::time::sleep(interval).await;
                }
            });
        }
    }

    pub async fn refresh_all(&self, emit_events: bool) {
        for (name, config) in &self.devices {
            self.refresh_device(name, config, emit_events).await;
        }
    }

    async fn refresh_device(&self, name: &str, config: &DeviceConfig, emit_events: bool) {
        let (changed_states, aggregate_state) =
            match fetch_snapshot(config, self.request_timeout).await {
                Ok(snapshot) => {
                    self.catalogs
                        .write()
                        .await
                        .insert(name.to_string(), snapshot.catalog.clone());

                    let changed_states = {
                        let mut states = self.states.write().await;
                        let mut changed = Vec::new();

                        if let Some(state) = states.replace_attributes(
                            &format!("eversolo.{name}.power"),
                            "switch",
                            snapshot.power_attributes,
                        ) {
                            changed.push(state);
                        }
                        if let Some(state) = states.replace_attributes(
                            &format!("eversolo.{name}.player"),
                            "media_player",
                            snapshot.player_attributes,
                        ) {
                            changed.push(state);
                        }

                        changed
                    };

                    let aggregate_state = {
                        let mut connectivity = self.connectivity.write().await;
                        if connectivity.mark_connected(name) {
                            Some(connectivity.aggregate_state())
                        } else {
                            None
                        }
                    };

                    (changed_states, aggregate_state)
                }
                Err(error) => {
                    debug!(device_name = name, error = %error, "poll refresh failed");

                    let changed_states = {
                        let mut states = self.states.write().await;
                        let mut changed = Vec::new();

                        if let Some(state) = states.replace_attributes(
                            &format!("eversolo.{name}.power"),
                            "switch",
                            offline_power_attributes(),
                        ) {
                            changed.push(state);
                        }
                        if let Some(state) = states.replace_attributes(
                            &format!("eversolo.{name}.player"),
                            "media_player",
                            offline_player_attributes(),
                        ) {
                            changed.push(state);
                        }

                        changed
                    };

                    let aggregate_state = {
                        let mut connectivity = self.connectivity.write().await;
                        if connectivity.mark_disconnected(name) {
                            Some(connectivity.aggregate_state())
                        } else {
                            None
                        }
                    };

                    (changed_states, aggregate_state)
                }
            };

        if emit_events {
            for state in changed_states {
                self.subscriptions
                    .broadcast_entity_change(
                        &state.entity_id,
                        &state.entity_type,
                        &state.attributes,
                    )
                    .await;
            }

            if let Some(state) = aggregate_state {
                self.subscriptions.broadcast_device_state(state).await;
            }
        }
    }

    async fn handle_get_device_state(&self) -> ConnectivityState {
        self.connectivity.read().await.aggregate_state()
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
        self.refresh_all(false).await;
        let states = self.states.read().await.snapshot();
        entity_states_response(req_id, &states)
    }

    async fn handle_entity_command(&self, request: &IntegrationRequest) -> Option<Value> {
        let entity_id = request.msg_data.get("entity_id")?.as_str()?;
        let cmd_id = request.msg_data.get("cmd_id")?.as_str()?;
        let params = request.msg_data.get("params").cloned().unwrap_or_default();

        let operation = match resolve_command(entity_id, cmd_id, &params) {
            Some(operation) => operation,
            None => {
                warn!(entity_id, cmd_id, "unknown entity command");
                let error = if entity_exists(entity_id) {
                    EversoloIntegrationError::UnknownCommand(cmd_id.to_string())
                } else {
                    EversoloIntegrationError::UnknownEntity(entity_id.to_string())
                };
                return Some(result_response(request.id, error.uc_error_code()));
            }
        };

        let device_name = match device_name_from_entity_id(entity_id) {
            Some(name) => name,
            None => return Some(result_response(request.id, 404)),
        };

        let config = match self.devices.get(device_name) {
            Some(config) => config,
            None => return Some(result_response(request.id, 404)),
        };

        match execute_operation(config, operation, self.request_timeout).await {
            Ok(updates) => {
                let changed_states = {
                    let mut states = self.states.write().await;
                    let mut changed = Vec::new();

                    for update in updates {
                        let entity_id = format!("eversolo.{device_name}.{}", update.entity_kind);
                        if let Some(state) = states.replace_attributes(
                            &entity_id,
                            update.entity_type,
                            update.attributes,
                        ) {
                            changed.push(state);
                        }
                    }

                    changed
                };

                for state in changed_states {
                    self.subscriptions
                        .broadcast_entity_change(
                            &state.entity_id,
                            &state.entity_type,
                            &state.attributes,
                        )
                        .await;
                }

                Some(result_response(request.id, 200))
            }
            Err(error) => {
                warn!(entity_id, cmd_id, error = %error, "entity command failed");
                Some(result_response(request.id, error.uc_error_code()))
            }
        }
    }
}

impl WsHandler for EversoloIntegrationHandler {
    async fn handle_message(&self, message: Value, context: WsConnectionContext) -> Option<Value> {
        let request = match IntegrationRequest::parse(message) {
            Ok(request) => request,
            Err(unfolded_integration_helper::EnvelopeError::UnexpectedKind(kind)) => {
                debug!(kind = ?kind, "ignoring non-request message");
                return None;
            }
            Err(error) => {
                warn!(error = %error, "invalid UC request envelope");
                return None;
            }
        };

        debug!(msg = %request.msg, req_id = request.id, "handling request");

        match request.msg.as_str() {
            "get_driver_version" => Some(driver_version_response(
                request.id,
                types::DRIVER_ID,
                types::DRIVER_VERSION,
                types::MIN_CORE_API,
            )),
            "get_driver_metadata" => Some(driver_metadata_response(
                request.id,
                &types::driver_metadata(),
            )),
            "get_device_state" => Some(device_state_event(
                self.handle_get_device_state().await.as_uc_label(),
            )),
            "get_available_entities" => Some(self.handle_get_available_entities(request.id).await),
            "subscribe_events" => {
                self.subscriptions.subscribe(&context).await;
                Some(result_response(request.id, 200))
            }
            "get_entity_states" => Some(self.handle_get_entity_states(request.id).await),
            "entity_command" => self.handle_entity_command(&request).await,
            _ => {
                warn!(msg = %request.msg, "unknown message type");
                Some(result_response(request.id, 400))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use unfolded_integration_helper::test_fixtures::{
        entity_command_request, request, subscribe_events_request,
    };

    fn handler_no_devices() -> EversoloIntegrationHandler {
        EversoloIntegrationHandler::new(
            HashMap::new(),
            Duration::from_millis(50),
            UnfoldedCircleEventHub::new(),
        )
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
        EversoloIntegrationHandler::new(
            devices,
            Duration::from_millis(50),
            UnfoldedCircleEventHub::new(),
        )
    }

    fn context() -> WsConnectionContext {
        WsConnectionContext::new(1, UnfoldedCircleEventHub::new())
    }

    #[tokio::test]
    async fn test_handle_get_driver_version() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(request(1, "get_driver_version", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["kind"], "resp");
        assert_eq!(resp["req_id"], 1);
        assert_eq!(resp["msg"], "driver_version");
        assert_eq!(resp["msg_data"]["name"], "eversolo-streamer");
        assert_eq!(resp["code"], 200);
    }

    #[tokio::test]
    async fn test_handle_get_device_state_no_devices() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(request(2, "get_device_state", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["msg_data"]["state"], "DISCONNECTED");
    }

    #[tokio::test]
    async fn test_handle_get_driver_metadata() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(request(8, "get_driver_metadata", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["kind"], "resp");
        assert_eq!(resp["req_id"], 8);
        assert_eq!(resp["msg"], "driver_metadata");
        assert_eq!(resp["code"], 200);
        assert_eq!(resp["msg_data"]["driver_id"], "eversolo-streamer");
        assert_eq!(resp["msg_data"]["developer"]["name"], "Ken Snyder");
    }

    #[tokio::test]
    async fn test_handle_get_available_entities() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(request(3, "get_available_entities", json!({})), context())
            .await
            .unwrap();

        let entities = resp["msg_data"]["available_entities"].as_array().unwrap();
        assert_eq!(entities.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_subscribe_events() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(subscribe_events_request(4), context())
            .await
            .unwrap();

        assert_eq!(resp["code"], 200);
    }

    #[tokio::test]
    async fn test_handle_get_entity_states() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(request(5, "get_entity_states", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["msg"], "entity_states");
        let states = resp["msg_data"].as_array().unwrap();
        assert_eq!(states.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_unknown_message() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(request(6, "bogus_message", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["code"], 400);
    }

    #[tokio::test]
    async fn test_handle_entity_command_missing_mac_returns_400() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(
                entity_command_request(7, "eversolo.living.power", "on", json!({})),
                context(),
            )
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["code"], 400);
    }

    #[tokio::test]
    async fn test_entity_command_unknown_cmd() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(
                entity_command_request(8, "eversolo.living.player", "invalid", json!({})),
                context(),
            )
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["code"], 400);
    }
}
