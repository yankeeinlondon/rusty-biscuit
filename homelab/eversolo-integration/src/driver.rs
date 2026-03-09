//! Eversolo device driver implementing the `DeviceDriver` trait.

use std::time::Duration;

use serde_json::Value;
use unfolded_integration_helper::{
    ConfiguredDevice, DeviceDriver, DeviceError, Entity, EntityState, EntityUpdate,
};

use crate::dispatch;
use crate::types::{self, DeviceConfig, EntityCatalog};

/// Eversolo streamer device driver.
#[derive(Debug, Clone)]
pub struct EversoloDeviceDriver;

/// Extract a `DeviceConfig` from the generic `ConfiguredDevice` fields.
fn device_config(device: &ConfiguredDevice) -> DeviceConfig {
    let catalog = catalog_from_device(device);
    let mac_address = device
        .driver_config
        .get("mac")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| device.metadata.mac_address.clone())
        .or_else(|| catalog.identity.net_mac.clone());

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
        screen_brightness_max: catalog.screen_brightness_max,
        knob_brightness_max: catalog.knob_brightness_max,
    }
}

fn catalog_from_device(device: &ConfiguredDevice) -> EntityCatalog {
    device
        .driver_config
        .get("catalog")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

impl DeviceDriver for EversoloDeviceDriver {
    async fn prepare_device(
        &self,
        mut device: ConfiguredDevice,
        timeout: Duration,
    ) -> Result<ConfiguredDevice, DeviceError> {
        if let Ok(snapshot) = dispatch::fetch_snapshot(&device_config(&device), timeout).await {
            device.driver_config.insert(
                "catalog".to_string(),
                serde_json::to_value(snapshot.catalog).unwrap_or_default(),
            );
        }

        Ok(device)
    }

    fn build_entities(&self, device: &ConfiguredDevice) -> Vec<Entity> {
        let name = &device.device_name;
        let catalog = catalog_from_device(device);

        types::build_entities(name, &catalog)
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
        types::build_initial_states(&device.device_name, &catalog_from_device(device))
    }

    async fn fetch_snapshot(
        &self,
        device: &ConfiguredDevice,
        timeout: Duration,
    ) -> Result<Vec<EntityUpdate>, DeviceError> {
        let config = device_config(device);
        let name = &device.device_name;

        match dispatch::fetch_snapshot(&config, timeout).await {
            Ok(snapshot) => Ok(snapshot
                .updates
                .into_iter()
                .map(|update| EntityUpdate {
                    entity_id: format!("eversolo.{name}.{}", update.entity_kind),
                    entity_type: update.entity_type.to_string(),
                    attributes: update.attributes,
                })
                .collect()),
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
