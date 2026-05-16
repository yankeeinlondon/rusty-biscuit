//! Eversolo device discovery via HTTP probe.

use std::time::Duration;

use homelab::eversolo::Eversolo;
use homelab::network::parse_host;
use serde_json::json;
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
            let model = client.get_model().await.ok()?;
            Some(DeviceMetadata {
                model: Some(model.model.clone()),
                friendly_name: Some(format!("{} ({host})", model.model)),
                firmware: Some(model.firmware.clone()),
                mac_address: Some(model.net_mac.clone()),
                extras: [
                    ("ip".to_string(), json!(model.ip)),
                    ("wifi_mac".to_string(), json!(model.wifi_mac)),
                    (
                        "able_remote_boot".to_string(),
                        json!(model.able_remote_boot.unwrap_or(false)),
                    ),
                    (
                        "has_eq_setting".to_string(),
                        json!(model.has_eq_setting.unwrap_or(false)),
                    ),
                    (
                        "android_version".to_string(),
                        json!(model.android_version.unwrap_or_default()),
                    ),
                ]
                .into_iter()
                .collect(),
            })
        })
        .await;

        result.ok().flatten()
    }
}
