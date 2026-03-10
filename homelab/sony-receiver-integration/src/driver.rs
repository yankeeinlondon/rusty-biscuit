//! Sony device driver implementing the `DeviceDriver` trait.

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
    async fn prepare_device(
        &self,
        mut device: ConfiguredDevice,
        timeout: Duration,
    ) -> Result<ConfiguredDevice, DeviceError> {
        if let Ok(source_list) =
            dispatch::fetch_source_list(&device.host, device.port, timeout).await
        {
            device
                .driver_config
                .insert("source_list".to_string(), serde_json::json!(source_list));
        }

        Ok(device)
    }

    fn build_entities(&self, device: &ConfiguredDevice) -> Vec<Entity> {
        let source_list = device
            .driver_config
            .get("source_list")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                types::SOURCE_CATEGORIES
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect()
            });

        types::build_entities(&device.device_name, &source_list)
            .into_iter()
            .map(|entity| Entity {
                entity_id: entity.entity_id,
                entity_type: entity.entity_type,
                name: Some(entity.name),
                features: Some(entity.features),
                options: entity.options,
            })
            .collect()
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
