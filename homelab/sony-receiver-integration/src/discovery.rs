//! Sony device discovery via TCP probe.

use std::time::Duration;

use homelab::network::parse_host;
use homelab::sony_receiver::SonyReceiver;
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
            // Probe by requesting power state - validates the device speaks Sony protocol
            let _power = sony.get_power_status().await.ok()?;
            Some(DeviceMetadata {
                model: Some("Sony ES Receiver".to_string()),
                friendly_name: Some(format!("Sony @ {host}")),
                ..Default::default()
            })
        })
        .await;

        result.ok().flatten()
    }
}
