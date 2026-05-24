//! Sony device discovery via TCP probe.

use std::time::Duration;

use homelab::network::parse_host;
use homelab::sony_receiver::SonyReceiver;
use serde_json::json;
use unfolded_integration_helper::{DeviceDiscovery, DeviceMetadata};

/// Sony receiver discovery implementation.
///
/// Used by the setup flow to probe candidate addresses for Sony receivers.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SonyDiscovery;

impl DeviceDiscovery for SonyDiscovery {
    async fn validate_host(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Option<DeviceMetadata> {
        let sony_host = parse_host(host).ok()?;
        let sony = SonyReceiver::new(sony_host, port);

        let result = tokio::time::timeout(timeout, async {
            let info = sony.get_system_information().await.ok()?;
            Some(DeviceMetadata {
                model: Some(info.model.clone()),
                friendly_name: info.name.or_else(|| Some(format!("Sony @ {host}"))),
                firmware: Some(info.version.clone()),
                mac_address: Some(info.mac_addr.clone()),
                extras: [
                    ("serial".to_string(), info.serial.map(|value| json!(value))),
                    (
                        "wireless_mac_addr".to_string(),
                        info.wireless_mac_addr.map(|value| json!(value)),
                    ),
                    (
                        "generation".to_string(),
                        info.generation.map(|value| json!(value)),
                    ),
                    ("region".to_string(), info.region.map(|value| json!(value))),
                    (
                        "product".to_string(),
                        info.product.map(|value| json!(value)),
                    ),
                ]
                .into_iter()
                .filter_map(|(key, value)| value.map(|value| (key, value)))
                .collect(),
            })
        })
        .await;

        result.ok().flatten()
    }
}
