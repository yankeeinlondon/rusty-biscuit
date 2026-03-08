//! UC Integration WebSocket handler for Eversolo streamers.

use std::time::Duration;

use schematic_schema::unfolded_circle_integration_ws::{WsConnectionContext, WsHandler};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
use tracing::{debug, info, warn};
use unfolded_integration_helper::{
    ConfiguredDevice, DeviceDiscovery, DeviceManager, DiscoverySource, IntegrationRequest,
    KnownDevice, SetupSession, SetupSessions, SetupState, available_entities_response,
    bounded_scan, build_candidate_list, device_selection_schema, driver_metadata_response,
    driver_version_response, entity_states_response, envelope_kind, envelope_message_name,
    local_ipv4_candidates, remote_id_from_context, remote_setup_abort_error, result_response,
    setup_driver_response, setup_progress_event, setup_wait_user_action_event,
};

use crate::discovery::EversoloDiscovery;
use crate::driver::EversoloDeviceDriver;
use crate::types;

const EVERSOLO_DEFAULT_PORT: u16 = 9529;
const SETUP_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(8);
const SETUP_VALIDATE_TIMEOUT: Duration = Duration::from_secs(5);

/// Eversolo integration handler implementing the UC WsHandler trait.
#[derive(Clone)]
pub struct EversoloIntegrationHandler {
    manager: DeviceManager<EversoloDeviceDriver>,
    driver: EversoloDeviceDriver,
    discovery: EversoloDiscovery,
    sessions: SetupSessions,
}

impl EversoloIntegrationHandler {
    pub fn new(manager: DeviceManager<EversoloDeviceDriver>) -> Self {
        Self {
            manager,
            driver: EversoloDeviceDriver,
            discovery: EversoloDiscovery,
            sessions: SetupSessions::default(),
        }
    }

    async fn handle_entity_command(&self, request: &IntegrationRequest) -> Option<Value> {
        let entity_id = request.msg_data.get("entity_id")?.as_str()?;
        let cmd_id = request.msg_data.get("cmd_id")?.as_str()?;
        let params = request.msg_data.get("params").cloned();

        match self
            .manager
            .handle_entity_command(entity_id, cmd_id, params)
            .await
        {
            Ok(_) => {
                debug!(entity_id, cmd_id, "entity command executed");
                Some(result_response(request.id, 200))
            }
            Err(error) => {
                warn!(entity_id, cmd_id, error = %error, "entity command failed");
                Some(result_response(request.id, error.uc_error_code()))
            }
        }
    }

    async fn handle_setup_driver(
        &self,
        request: &IntegrationRequest,
        context: &WsConnectionContext,
    ) -> Option<Value> {
        if let Err(error) = context.send(setup_driver_response(request.id, 200)).await {
            warn!(
                error = %error,
                req_id = request.id,
                "failed to acknowledge Eversolo setup start"
            );
            return None;
        }

        let remote_id = remote_id_from_context(context);
        info!(remote_id, req_id = request.id, "Eversolo setup started");
        let registry = self.manager.registry().clone();
        let known_devices = registry.get_known_devices().await;
        let configured_devices = registry.get_configured_devices().await;
        let assignments = registry.get_assignments().await;

        let candidates = self.discover_candidates(known_devices).await;
        info!(
            remote_id,
            req_id = request.id,
            candidate_count = candidates.len(),
            "Eversolo setup discovery finished"
        );
        for device in &candidates {
            registry.add_known_device(device.clone()).await;
        }
        if let Err(error) = registry.save().await {
            warn!(error = %error, "failed to persist Eversolo setup candidates");
        }

        self.sessions
            .set(
                remote_id.clone(),
                SetupSession {
                    state: Some(SetupState::DeviceSelection {
                        candidates: candidates.clone(),
                    }),
                    candidates: candidates.clone(),
                },
            )
            .await;

        if let Err(error) = context
            .send(setup_progress_event(&SetupState::Discovering))
            .await
        {
            warn!(error = %error, "failed to send Eversolo discovery progress");
        }

        let selection_schema = device_selection_schema(
            &candidates,
            &configured_devices,
            &remote_id,
            &assignments,
        );
        let selection_event = setup_wait_user_action_event(
            &selection_schema["title"],
            selection_schema["settings"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        if let Err(error) = context.send(selection_event).await {
            warn!(error = %error, "failed to send Eversolo setup selection");
        }

        None
    }

    async fn handle_set_driver_user_data(
        &self,
        request: &IntegrationRequest,
        context: &WsConnectionContext,
    ) -> Option<Value> {
        if let Err(error) = context.send(result_response(request.id, 200)).await {
            warn!(
                error = %error,
                req_id = request.id,
                "failed to acknowledge Eversolo setup user data"
            );
            return None;
        }

        let remote_id = remote_id_from_context(context);
        let Some(session) = self.sessions.get(&remote_id).await else {
            let _ = self
                .setup_error(
                    request.id,
                    context,
                    "setup session not found".to_string(),
                    400,
                )
                .await;
            return None;
        };

        let registry = self.manager.registry().clone();
        let configured_devices = registry.get_configured_devices().await;
        let selected_device_id = setup_string(&request.msg_data, "device_select");
        let manual_host = setup_string(&request.msg_data, "device_host");
        let requested_name = setup_string(&request.msg_data, "device_name");
        info!(
            remote_id,
            req_id = request.id,
            selected_device_id,
            manual_host,
            requested_name,
            candidate_count = session.candidates.len(),
            "Eversolo setup user data received"
        );

        let mut config = if let Some(device_id) = selected_device_id.as_deref().filter(|value| !value.is_empty())
        {
            if registry.is_assigned(&remote_id, device_id).await {
                return self
                    .setup_error(
                        request.id,
                        context,
                        format!("device {device_id} is already assigned to this Remote"),
                        409,
                    )
                    .await
                    .and(None);
            }

            if let Some(existing) = configured_devices
                .iter()
                .find(|device| device.device_id == device_id)
                .cloned()
            {
                existing
            } else if let Some(known) = session
                .candidates
                .iter()
                .find(|device| device.device_id == device_id)
                .cloned()
            {
                ConfiguredDevice {
                    device_id: known.device_id,
                    device_name: requested_name
                        .clone()
                        .filter(|value| !value.is_empty())
                        .or(known.metadata.friendly_name.clone())
                        .unwrap_or_else(|| known.host.clone()),
                    host: known.host,
                    port: known.port,
                    metadata: known.metadata,
                    driver_config: Default::default(),
                }
            } else {
                return self
                    .setup_error(
                        request.id,
                        context,
                        format!("unknown Eversolo device selection {device_id}"),
                        400,
                    )
                    .await
                    .and(None);
            }
        } else if let Some(host) = manual_host.filter(|value| !value.is_empty()) {
            let Some(metadata) = self
                .discovery
                .validate_host(&host, EVERSOLO_DEFAULT_PORT, SETUP_VALIDATE_TIMEOUT)
                .await
            else {
                return self
                    .setup_error(
                        request.id,
                        context,
                        format!("unable to validate Eversolo device at {host}"),
                        400,
                    )
                    .await
                    .and(None);
            };

            ConfiguredDevice {
                device_id: metadata
                    .mac_address
                    .clone()
                    .unwrap_or_else(|| format!("{host}:{EVERSOLO_DEFAULT_PORT}")),
                device_name: requested_name
                    .clone()
                    .filter(|value| !value.is_empty())
                    .or(metadata.friendly_name.clone())
                    .unwrap_or_else(|| host.clone()),
                host,
                port: EVERSOLO_DEFAULT_PORT,
                metadata,
                driver_config: Default::default(),
            }
        } else {
            return self
                .setup_error(
                    request.id,
                    context,
                    "no device selection or host provided".to_string(),
                    400,
                )
                .await
                .and(None);
        };

        if let Some(name) = requested_name.filter(|value| !value.is_empty()) {
            config.device_name = name;
        }

        let known = KnownDevice {
            device_id: config.device_id.clone(),
            source: DiscoverySource::RemoteSetup,
            host: config.host.clone(),
            port: config.port,
            metadata: config.metadata.clone(),
            last_validated: None,
        };

        registry.add_known_device(known).await;
        registry.add_configured_device(config.clone()).await;
        if let Err(error) = registry.assign_device(&remote_id, &config.device_id).await {
            return self
                .setup_error(request.id, context, error.to_string(), 409)
                .await
                .and(None);
        }
        if let Err(error) = registry.save().await {
            return self
                .setup_error(request.id, context, error.to_string(), 503)
                .await
                .and(None);
        }
        let completed_device_id = config.device_id.clone();
        let completed_host = config.host.clone();
        let completed_name = config.device_name.clone();
        if let Err(error) = self.manager.add_device(config, self.driver.clone()).await {
            return self
                .setup_error(request.id, context, error.to_string(), 503)
                .await
                .and(None);
        }

        self.sessions.clear(&remote_id).await;
        info!(
            remote_id,
            req_id = request.id,
            device_id = completed_device_id,
            host = completed_host,
            device_name = completed_name,
            "Eversolo setup completed"
        );
        if let Err(error) = context.send(setup_progress_event(&SetupState::Complete)).await {
            warn!(error = %error, "failed to send Eversolo setup completion");
        }

        None
    }

    async fn discover_candidates(&self, known_devices: Vec<KnownDevice>) -> Vec<KnownDevice> {
        let candidates = build_candidate_list(
            &known_devices,
            &[],
            &local_ipv4_candidates(EVERSOLO_DEFAULT_PORT),
        );

        let scanned = bounded_scan(
            &self.discovery,
            candidates,
            Duration::from_secs(3),
            SETUP_DISCOVERY_TIMEOUT,
        )
        .await;

        if scanned.is_empty() {
            known_devices
        } else {
            scanned
        }
    }

    async fn setup_error(
        &self,
        req_id: u64,
        context: &WsConnectionContext,
        message: String,
        code: u16,
    ) -> Option<Value> {
        let remote_id = remote_id_from_context(context);
        info!(
            remote_id,
            req_id,
            code,
            error = %message,
            "Eversolo setup failed"
        );
        if let Err(error) = context
            .send(setup_progress_event(&SetupState::Error(message)))
            .await
        {
            warn!(error = %error, "failed to send Eversolo setup error");
        }
        Some(result_response(req_id, code))
    }
}

impl WsHandler for EversoloIntegrationHandler {
    async fn handle_message(&self, message: Value, context: WsConnectionContext) -> Option<Value> {
        if let Some(error) = remote_setup_abort_error(&message) {
            let remote_id = remote_id_from_context(&context);
            self.sessions.clear(&remote_id).await;
            warn!(remote_id, error, "UC Remote aborted Eversolo setup");
            return None;
        }

        if envelope_kind(&message) != Some("req") {
            debug!(
                kind = ?envelope_kind(&message),
                msg = ?envelope_message_name(&message),
                "ignoring non-request message"
            );
            return None;
        }

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
        let remote_id = remote_id_from_context(&context);

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
            "get_device_state" => {
                let conn = self.manager.connectivity().read().await;
                Some(unfolded_integration_helper::device_state_event(
                    conn.overall().as_uc_label(),
                ))
            }
            "get_available_entities" => {
                self.manager
                    .ensure_assigned_devices_active(&remote_id, self.driver.clone())
                    .await;
                let entities = self.manager.get_entities_for_remote(&remote_id).await;
                Some(available_entities_response(request.id, &entities))
            }
            "subscribe_events" => {
                self.manager.subscriptions().subscribe(&context).await;
                Some(result_response(request.id, 200))
            }
            "get_entity_states" => {
                self.manager
                    .ensure_assigned_devices_active(&remote_id, self.driver.clone())
                    .await;
                self.manager.refresh_remote(&remote_id).await;
                let states = self.manager.get_states_for_remote(&remote_id).await;
                Some(entity_states_response(request.id, &states))
            }
            "entity_command" => self.handle_entity_command(&request).await,
            "setup_driver" => self.handle_setup_driver(&request, &context).await,
            "set_driver_user_data" => self.handle_set_driver_user_data(&request, &context).await,
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
    use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleEventHub;
    use unfolded_integration_helper::{
        ConfiguredDevice, PersistentRegistry, SubscriptionRegistry,
        test_fixtures::{entity_command_request, request, subscribe_events_request},
    };

    async fn make_handler(add_device: bool) -> EversoloIntegrationHandler {
        let tmp = tempfile::TempDir::new().unwrap();
        let registry = PersistentRegistry::load(tmp.path().join("registry.json"))
            .await
            .unwrap();
        let hub = UnfoldedCircleEventHub::new();
        let subs = SubscriptionRegistry::new(hub.clone());
        let manager = DeviceManager::new(
            registry,
            subs,
            Duration::from_millis(50),
            Duration::from_secs(60),
        );

        if add_device {
            let config = ConfiguredDevice {
                device_id: "127.0.0.1:9529".to_string(),
                device_name: "living".to_string(),
                host: "127.0.0.1".to_string(),
                port: 9529,
                metadata: Default::default(),
                driver_config: Default::default(),
            };
            manager.registry().add_configured_device(config.clone()).await;
            manager.add_device(config, EversoloDeviceDriver).await.unwrap();
        }

        EversoloIntegrationHandler::new(manager)
    }

    fn context() -> WsConnectionContext {
        WsConnectionContext::new(1, UnfoldedCircleEventHub::new())
    }

    #[tokio::test]
    async fn test_handle_get_driver_version() {
        let handler = make_handler(false).await;
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
        let handler = make_handler(false).await;
        let resp = handler
            .handle_message(request(2, "get_device_state", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["msg_data"]["state"], "DISCONNECTED");
    }

    #[tokio::test]
    async fn test_handle_get_driver_metadata() {
        let handler = make_handler(false).await;
        let resp = handler
            .handle_message(request(8, "get_driver_metadata", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["kind"], "resp");
        assert_eq!(resp["req_id"], 8);
        assert_eq!(resp["msg"], "driver_metadata");
        assert_eq!(resp["code"], 200);
        assert_eq!(resp["msg_data"]["driver_id"], "eversolo-streamer");
        assert!(resp["msg_data"]["setup_data_schema"].is_object());
    }

    #[tokio::test]
    async fn test_handle_get_available_entities() {
        let handler = make_handler(true).await;
        handler
            .manager
            .registry()
            .assign_device("connection-1", "127.0.0.1:9529")
            .await
            .unwrap();
        let resp = handler
            .handle_message(request(3, "get_available_entities", json!({})), context())
            .await
            .unwrap();

        let entities = resp["msg_data"]["available_entities"].as_array().unwrap();
        assert_eq!(entities.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_subscribe_events() {
        let handler = make_handler(false).await;
        let resp = handler
            .handle_message(subscribe_events_request(4), context())
            .await
            .unwrap();

        assert_eq!(resp["code"], 200);
    }

    #[tokio::test]
    async fn test_handle_get_entity_states() {
        let handler = make_handler(true).await;
        handler
            .manager
            .registry()
            .assign_device("connection-1", "127.0.0.1:9529")
            .await
            .unwrap();
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
        let handler = make_handler(false).await;
        let resp = handler
            .handle_message(request(6, "bogus_message", json!({})), context())
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["code"], 400);
    }

    #[tokio::test]
    async fn test_entity_command_unknown_entity() {
        let handler = make_handler(false).await;
        let resp = handler
            .handle_message(
                entity_command_request(7, "eversolo.living.unknown", "on", json!({})),
                context(),
            )
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        assert_eq!(resp["code"], 404);
    }

    #[tokio::test]
    async fn test_entity_command_unknown_cmd() {
        let handler = make_handler(true).await;
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

fn setup_string(msg_data: &Value, key: &str) -> Option<String> {
    msg_data
        .get("input_values")
        .and_then(|value| value.get(key))
        .or_else(|| msg_data.get("setup_data").and_then(|value| value.get(key)))
        .or_else(|| msg_data.get("user_data").and_then(|value| value.get(key)))
        .or_else(|| msg_data.get(key))
        .and_then(setup_value_to_string)
}

fn setup_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Object(map) => map
            .get("value")
            .and_then(setup_value_to_string)
            .or_else(|| map.get("id").and_then(setup_value_to_string)),
        _ => None,
    }
}
