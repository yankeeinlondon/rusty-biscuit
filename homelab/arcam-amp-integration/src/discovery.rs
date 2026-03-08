//! Arcam device discovery via TCP probe.

use std::time::Duration;

use homelab::arcam::Arcam;
use homelab::network::parse_host;
use unfolded_integration_helper::{DeviceDiscovery, DeviceMetadata};

/// Arcam amplifier discovery implementation.
///
/// Used by the setup flow to probe candidate addresses for Arcam amplifiers.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ArcamDiscovery;

impl DeviceDiscovery for ArcamDiscovery {
    async fn validate_host(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Option<DeviceMetadata> {
        let arcam_host = parse_host(host).ok()?;
        let arcam = Arcam::new(arcam_host, port);

        let result = tokio::time::timeout(timeout, async {
            // Probe by requesting power state - this validates the device speaks Arcam protocol
            let _power = arcam.request_power_state().await.ok()?;
            Some(DeviceMetadata {
                model: Some("Arcam Amplifier".to_string()),
                friendly_name: Some(format!("Arcam @ {host}")),
                ..Default::default()
            })
        })
        .await;

        result.ok().flatten()
    }
}
