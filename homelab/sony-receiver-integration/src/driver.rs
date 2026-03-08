//! Sony device driver implementing the `DeviceDriver` trait.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::Value;
use unfolded_integration_helper::{
    ConfiguredDevice, DeviceDriver, DeviceError, Entity, EntityState, EntityUpdate,
};

use crate::dispatch;
use crate::types;

/// Sony receiver device driver.
#[derive(Debug, Clone)]
pub struct SonyDeviceDriver;

impl DeviceDriver for SonyDeviceDriver {
    fn build_entities(&self, device: &ConfiguredDevice) -> Vec<Entity> {
        let name = &device.device_name;
        vec![
            Entity {
                entity_id: format!("sony.{name}.power"),
                entity_type: "switch".to_string(),
                name: Some(HashMap::from([("en".to_string(), format!("{name} Power"))])),
                features: Some(vec!["on_off".to_string()]),
                options: None,
            },
            Entity {
                entity_id: format!("sony.{name}.receiver"),
                entity_type: "media_player".to_string(),
                name: Some(HashMap::from([(
                    "en".to_string(),
                    format!("{name} Receiver"),
                )])),
                features: Some(vec![
                    "volume".to_string(),
                    "volume_up_down".to_string(),
                    "mute".to_string(),
                    "mute_toggle".to_string(),
                    "select_source".to_string(),
                ]),
                options: Some(HashMap::from([(
                    "source_list".to_string(),
                    serde_json::json!(types::SOURCE_CATEGORIES),
                )])),
            },
        ]
    }

    fn build_initial_states(&self, device: &ConfiguredDevice) -> Vec<EntityState> {
        types::build_initial_states(&device.device_name)
    }

    async fn fetch_snapshot(
        &self,
        device: &ConfiguredDevice,
        timeout: Duration,
    ) -> Result<Vec<EntityUpdate>, DeviceError> {
        let name = &device.device_name;

        match dispatch::fetch_receiver_state(&device.host, device.port, timeout).await {
            Ok(snapshot) => Ok(vec![
                EntityUpdate {
                    entity_id: format!("sony.{name}.power"),
                    entity_type: "switch".to_string(),
                    attributes: snapshot.power_attributes,
                },
                EntityUpdate {
                    entity_id: format!("sony.{name}.receiver"),
                    entity_type: "media_player".to_string(),
                    attributes: snapshot.receiver_attributes,
                },
            ]),
            Err(e) => Err(DeviceError::Communication(e.to_string())),
        }
    }

    async fn execute_command(
        &self,
        device: &ConfiguredDevice,
        entity_id: &str,
        cmd_id: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Vec<EntityUpdate>, DeviceError> {
        let params_value = params.unwrap_or_default();
        let operation = types::resolve_command(entity_id, cmd_id, &params_value)
            .ok_or_else(|| DeviceError::UnknownCommand(format!("{entity_id}:{cmd_id}")))?;

        dispatch::execute_operation(&device.host, device.port, operation, timeout)
            .await
            .map_err(|e| DeviceError::Communication(e.to_string()))?;

        // Fetch fresh state after command
        self.fetch_snapshot(device, timeout).await
    }
}
