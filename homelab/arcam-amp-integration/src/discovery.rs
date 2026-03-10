//! Arcam device discovery via TCP probe.

use std::time::Duration;

use homelab::arcam::Arcam;
use homelab::network::parse_host;
use serde_json::json;
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
            let status = arcam.get_system_status().await.ok()?;
            Some(DeviceMetadata {
                model: status.model.or_else(|| Some("Arcam Amplifier".to_string())),
                friendly_name: status
                    .friendly_name
                    .or_else(|| Some(format!("Arcam @ {host}"))),
                firmware: status.software_version,
                extras: [
                    (
                        "ip_address".to_string(),
                        status.ip_address.map(|value| json!(value)),
                    ),
                    (
                        "amplifier_mode".to_string(),
                        status.amplifier_mode.map(|value| json!(value)),
                    ),
                ]
                .into_iter()
                .filter_map(|(key, value)| value.map(|value| (key, value)))
                .collect(),
                ..Default::default()
            })
        })
        .await;

        result.ok().flatten()
    }
}
