//! Eversolo device driver implementing the `DeviceDriver` trait.

use std::time::Duration;

use serde_json::Value;
use unfolded_integration_helper::{
    ConfiguredDevice, DeviceDriver, DeviceError, Entity, EntityState, EntityUpdate,
};

use crate::dispatch;
use crate::types::{self, DEFAULT_VOLUME_STEPS, DeviceConfig};

/// Eversolo streamer device driver.
#[derive(Debug, Clone)]
pub struct EversoloDeviceDriver;

/// Extract a `DeviceConfig` from the generic `ConfiguredDevice` fields.
fn device_config(device: &ConfiguredDevice) -> DeviceConfig {
    let mac_address = device
        .driver_config
        .get("mac")
        .and_then(|v| v.as_str())
        .map(String::from);

    let wol_broadcast = device
        .driver_config
        .get("wol_broadcast")
        .and_then(|v| v.as_str())
        .unwrap_or("255.255.255.255")
        .to_string();

    let wol_port = device
        .driver_config
        .get("wol_port")
        .and_then(|v| v.as_u64())
        .map(|v| v as u16)
        .unwrap_or(9517);

    DeviceConfig {
        host: device.host.clone(),
        port: device.port,
        mac_address,
        wol_broadcast,
        wol_port,
    }
}

impl DeviceDriver for EversoloDeviceDriver {
    fn build_entities(&self, device: &ConfiguredDevice) -> Vec<Entity> {
        let name = &device.device_name;
        types::build_entities(name, &[], DEFAULT_VOLUME_STEPS)
            .into_iter()
            .map(|e| Entity {
                entity_id: e.entity_id,
                entity_type: e.entity_type,
                name: Some(e.name),
                features: Some(e.features),
                options: e.options,
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
        let config = device_config(device);
        let name = &device.device_name;

        match dispatch::fetch_snapshot(&config, timeout).await {
            Ok(snapshot) => Ok(vec![
                EntityUpdate {
                    entity_id: format!("eversolo.{name}.power"),
                    entity_type: "switch".to_string(),
                    attributes: snapshot.power_attributes,
                },
                EntityUpdate {
                    entity_id: format!("eversolo.{name}.player"),
                    entity_type: "media_player".to_string(),
                    attributes: snapshot.player_attributes,
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
        let config = device_config(device);
        let name = &device.device_name;
        let params_value = params.unwrap_or_default();

        let operation = types::resolve_command(entity_id, cmd_id, &params_value)
            .ok_or_else(|| DeviceError::UnknownCommand(format!("{entity_id}:{cmd_id}")))?;

        dispatch::execute_operation(&config, operation, timeout)
            .await
            .map(|updates| {
                updates
                    .into_iter()
                    .map(|u| EntityUpdate {
                        entity_id: format!("eversolo.{name}.{}", u.entity_kind),
                        entity_type: u.entity_type.to_string(),
                        attributes: u.attributes,
                    })
                    .collect()
            })
            .map_err(|e| DeviceError::Communication(e.to_string()))
    }
}
