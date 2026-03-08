//! Eversolo device discovery via HTTP probe.

use std::time::Duration;

use homelab::eversolo::Eversolo;
use homelab::network::parse_host;
use unfolded_integration_helper::{DeviceDiscovery, DeviceMetadata};

/// Eversolo streamer discovery implementation.
///
/// Used by the setup flow to probe candidate addresses for Eversolo streamers.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EversoloDiscovery;

impl DeviceDiscovery for EversoloDiscovery {
    async fn validate_host(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Option<DeviceMetadata> {
        let eversolo_host = parse_host(host).ok()?;
        let client = Eversolo::new(eversolo_host.to_string(), port);

        let result = tokio::time::timeout(timeout, async {
            // Probe by requesting device state - validates the device speaks Eversolo protocol
            let _state = client.get_state().await.ok()?;
            Some(DeviceMetadata {
                model: Some("Eversolo Streamer".to_string()),
                friendly_name: Some(format!("Eversolo @ {host}")),
                ..Default::default()
            })
        })
        .await;

        result.ok().flatten()
    }
}
