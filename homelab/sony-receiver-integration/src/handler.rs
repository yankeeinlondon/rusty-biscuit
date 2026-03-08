//! UC Integration WebSocket handler for Sony receivers.

use schematic_schema::unfolded_circle_integration_ws::{WsConnectionContext, WsHandler};
use serde_json::Value;
use tracing::{debug, warn};
use unfolded_integration_helper::{
    DeviceManager, IntegrationRequest, available_entities_response, driver_metadata_response,
    driver_version_response, entity_states_response, result_response,
};

use crate::driver::SonyDeviceDriver;
use crate::types;

/// Sony receiver integration handler implementing the UC WsHandler trait.
pub struct SonyIntegrationHandler {
    manager: DeviceManager<SonyDeviceDriver>,
}

impl SonyIntegrationHandler {
    pub fn new(manager: DeviceManager<SonyDeviceDriver>) -> Self {
        Self { manager }
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
}

impl WsHandler for SonyIntegrationHandler {
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
            "get_device_state" => {
                let conn = self.manager.connectivity().read().await;
                Some(unfolded_integration_helper::device_state_event(
                    conn.overall().as_uc_label(),
                ))
            }
            "get_available_entities" => {
                let entities = self.manager.get_all_entities().await;
                Some(available_entities_response(request.id, &entities))
            }
            "subscribe_events" => {
                self.manager.subscriptions().subscribe(&context).await;
                Some(result_response(request.id, 200))
            }
            "get_entity_states" => {
                self.manager.refresh_all().await;
                let states = self.manager.get_all_states().await;
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
    use std::time::Duration;

    use super::*;
    use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleEventHub;
    use serde_json::json;
    use unfolded_integration_helper::{
        ConfiguredDevice, PersistentRegistry, SubscriptionRegistry,
        test_fixtures::{entity_command_request, request, subscribe_events_request},
    };

    async fn make_handler(add_device: bool) -> SonyIntegrationHandler {
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
                device_id: "192.168.99.99:10000".to_string(),
                device_name: "living".to_string(),
                host: "192.168.99.99".to_string(),
                port: 10000,
                metadata: Default::default(),
                driver_config: Default::default(),
            };
            manager.add_device(config, SonyDeviceDriver).await;
        }

        SonyIntegrationHandler::new(manager)
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
        assert_eq!(resp["msg_data"]["name"], "sony-receiver");
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
        assert_eq!(resp["msg_data"]["driver_id"], "sony-receiver");
        assert_eq!(resp["msg_data"]["developer"]["name"], "Ken Snyder");
    }

    #[tokio::test]
    async fn test_handle_get_available_entities() {
        let handler = make_handler(true).await;
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
                entity_command_request(7, "sony.living.unknown", "on", json!({})),
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
                entity_command_request(8, "sony.living.power", "invalid", json!({})),
                context(),
            )
            .await
            .unwrap();

        assert_eq!(resp["msg"], "result");
        // 503 because the driver tries to execute and fails on unreachable host
        // rather than 400, since resolve_command doesn't match "invalid"
        assert!(resp["code"].as_u64().unwrap() >= 400);
    }

    #[tokio::test]
    async fn toggle_not_in_power_features() {
        let handler = make_handler(true).await;
        let resp = handler
            .handle_message(request(9, "get_available_entities", json!({})), context())
            .await
            .unwrap();

        let entities = resp["msg_data"]["available_entities"].as_array().unwrap();
        for entity in entities {
            if entity["entity_type"] == "switch" {
                let features = entity["features"].as_array().unwrap();
                let feature_strings: Vec<&str> =
                    features.iter().filter_map(|f| f.as_str()).collect();
                assert!(
                    !feature_strings.contains(&"toggle"),
                    "toggle should not be in power switch features"
                );
            }
        }
    }
}
