//! UC Integration WebSocket handler for Arcam amplifiers.

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
    available_entities_response, driver_version_response, entity_states_response, result_response,
};

use crate::dispatch::{execute_operation, fetch_snapshot, unknown_state_attrs};
use crate::error::ArcamIntegrationError;
use crate::types::{self, ArcamEntity, device_name_from_entity_id, resolve_command};

/// Arcam integration handler implementing the UC WsHandler trait.
pub struct ArcamIntegrationHandler {
    /// Arcam device configs: name -> (host, port)
    devices: HashMap<String, (String, u16)>,
    /// Entity registry
    entities: Vec<ArcamEntity>,
    /// Cached entity states keyed by entity id.
    states: Arc<RwLock<StateCache>>,
    /// Connectivity tracker keyed by configured device name.
    connectivity: Arc<RwLock<ConnectivityTracker>>,
    /// Shared event subscription bridge.
    subscriptions: SubscriptionRegistry,
    /// Request timeout for Arcam TCP operations
    request_timeout: Duration,
}

impl ArcamIntegrationHandler {
    pub fn new(
        devices: HashMap<String, (String, u16)>,
        request_timeout: Duration,
        hub: UnfoldedCircleEventHub,
    ) -> Self {
        let mut entities = Vec::new();
        let mut states = Vec::new();

        for name in devices.keys() {
            entities.extend(types::build_entities(name));
            states.extend(types::build_initial_states(name));
        }

        Self {
            devices,
            entities,
            states: Arc::new(RwLock::new(StateCache::new(states))),
            connectivity: Arc::new(RwLock::new(ConnectivityTracker::default())),
            subscriptions: SubscriptionRegistry::new(hub),
            request_timeout,
        }
    }

    pub async fn refresh_all(&self, emit_events: bool) {
        for (device_name, (host, port)) in self.devices.clone() {
            self.refresh_device(&device_name, &host, port, emit_events)
                .await;
        }
    }

    pub fn start_polling(self: &Arc<Self>, interval: Duration) {
        for (device_name, (host, port)) in self.devices.clone() {
            let handler = Arc::clone(self);
            tokio::spawn(async move {
                loop {
                    handler
                        .refresh_device(&device_name, &host, port, true)
                        .await;
                    tokio::time::sleep(interval).await;
                }
            });
        }
    }

    async fn refresh_device(&self, device_name: &str, host: &str, port: u16, emit_events: bool) {
        let (changed_states, aggregate_state) =
            match fetch_snapshot(host, port, self.request_timeout).await {
                Ok(snapshot) => {
                    let changed_states = {
                        let mut states = self.states.write().await;
                        let mut changed = Vec::new();

                        if let Some(state) = states.replace_attributes(
                            &format!("arcam.{device_name}.power"),
                            "switch",
                            snapshot.power_attributes,
                        ) {
                            changed.push(state);
                        }
                        if let Some(state) = states.replace_attributes(
                            &format!("arcam.{device_name}.mute"),
                            "switch",
                            snapshot.mute_attributes,
                        ) {
                            changed.push(state);
                        }

                        changed
                    };

                    let aggregate_state = {
                        let mut connectivity = self.connectivity.write().await;
                        if connectivity.mark_connected(device_name) {
                            Some(connectivity.aggregate_state())
                        } else {
                            None
                        }
                    };

                    (changed_states, aggregate_state)
                }
                Err(error) => {
                    warn!(device_name, error = %error, "failed to refresh Arcam device state");

                    let changed_states = {
                        let mut states = self.states.write().await;
                        let mut changed = Vec::new();

                        if let Some(state) = states.replace_attributes(
                            &format!("arcam.{device_name}.power"),
                            "switch",
                            unknown_state_attrs(),
                        ) {
                            changed.push(state);
                        }
                        if let Some(state) = states.replace_attributes(
                            &format!("arcam.{device_name}.mute"),
                            "switch",
                            unknown_state_attrs(),
                        ) {
                            changed.push(state);
                        }

                        changed
                    };

                    let aggregate_state = {
                        let mut connectivity = self.connectivity.write().await;
                        if connectivity.mark_disconnected(device_name) {
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

    async fn current_device_state(&self) -> ConnectivityState {
        self.connectivity.read().await.aggregate_state()
    }

    async fn handle_entity_command(&self, request: &IntegrationRequest) -> Option<Value> {
        let entity_id = request.msg_data.get("entity_id")?.as_str()?;
        let cmd_id = request.msg_data.get("cmd_id")?.as_str()?;

        let operation = match resolve_command(entity_id, cmd_id) {
            Some(operation) => operation,
            None => {
                warn!(entity_id, cmd_id, "unknown entity command");
                let error = if self
                    .entities
                    .iter()
                    .any(|entity| entity.entity_id == entity_id)
                {
                    ArcamIntegrationError::UnknownCommand(cmd_id.to_string())
                } else {
                    ArcamIntegrationError::UnknownEntity(entity_id.to_string())
                };
                return Some(result_response(request.id, error.uc_error_code()));
            }
        };

        let device_name = match device_name_from_entity_id(entity_id) {
            Some(device_name) => device_name,
            None => return Some(result_response(request.id, 404)),
        };

        let (host, port) = match self.devices.get(device_name) {
            Some(device) => device,
            None => return Some(result_response(request.id, 404)),
        };

        match execute_operation(host, *port, operation, self.request_timeout).await {
            Ok(()) => {
                debug!(entity_id, cmd_id, "entity command executed");
                self.refresh_device(device_name, host, *port, true).await;
                Some(result_response(request.id, 200))
            }
            Err(error) => {
                warn!(entity_id, cmd_id, error = %error, "entity command failed");
                Some(result_response(request.id, error.uc_error_code()))
            }
        }
    }
}

impl WsHandler for ArcamIntegrationHandler {
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
            "get_device_state" => Some(unfolded_integration_helper::device_state_event(
                self.current_device_state().await.as_uc_label(),
            )),
            "get_available_entities" => {
                Some(available_entities_response(request.id, &self.entities))
            }
            "subscribe_events" => {
                self.subscriptions.subscribe(&context).await;
                Some(result_response(request.id, 200))
            }
            "get_entity_states" => {
                self.refresh_all(false).await;
                let states = self.states.read().await.snapshot();
                Some(entity_states_response(request.id, &states))
            }
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

    fn handler_no_devices() -> ArcamIntegrationHandler {
        ArcamIntegrationHandler::new(
            HashMap::new(),
            Duration::from_secs(5),
            UnfoldedCircleEventHub::new(),
        )
    }

    fn handler_with_device() -> ArcamIntegrationHandler {
        let mut devices = HashMap::new();
        devices.insert("office".to_string(), ("192.168.99.99".to_string(), 50000));
        ArcamIntegrationHandler::new(
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
    async fn test_entity_command_unknown_entity() {
        let handler = handler_no_devices();
        let resp = handler
            .handle_message(
                entity_command_request(7, "arcam.office.unknown", "on", json!({})),
                context(),
            )
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["code"], 404);
    }

    #[tokio::test]
    async fn test_entity_command_unknown_cmd() {
        let handler = handler_with_device();
        let resp = handler
            .handle_message(
                entity_command_request(8, "arcam.office.power", "invalid", json!({})),
                context(),
            )
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["code"], 400);
    }
}
