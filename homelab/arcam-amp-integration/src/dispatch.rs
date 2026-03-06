//! Arcam TCP command execution.

use std::collections::HashMap;
use std::time::Duration;

use homelab::arcam::Arcam;
use homelab::network::parse_host;
use serde_json::{Value, json};

use crate::error::ArcamIntegrationError;
use crate::types::ArcamOperation;

/// Execute an Arcam operation and return updated entity state attributes.
pub async fn execute_operation(
    host: &str,
    port: u16,
    operation: ArcamOperation,
    timeout: Duration,
) -> Result<HashMap<String, Value>, ArcamIntegrationError> {
    let arcam_host = parse_host(host).map_err(|e| ArcamIntegrationError::InvalidHost(e.0))?;
    let arcam = Arcam::new(arcam_host, port);

    let result = tokio::time::timeout(timeout, async {
        match operation {
            ArcamOperation::PowerOn => {
                arcam.power_on().await?;
                Ok(state_attrs(true))
            }
            ArcamOperation::PowerOff => {
                arcam.power_off().await?;
                Ok(state_attrs(false))
            }
            ArcamOperation::PowerToggle => {
                let current = arcam.request_power_state().await?;
                if current {
                    arcam.power_off().await?;
                    Ok(state_attrs(false))
                } else {
                    arcam.power_on().await?;
                    Ok(state_attrs(true))
                }
            }
            ArcamOperation::MuteOn => {
                arcam.mute_on().await?;
                Ok(state_attrs(true))
            }
            ArcamOperation::MuteOff => {
                arcam.mute_off().await?;
                Ok(state_attrs(false))
            }
            ArcamOperation::MuteToggle => {
                let current = arcam.get_mute_status().await?;
                if current {
                    arcam.mute_off().await?;
                    Ok(state_attrs(false))
                } else {
                    arcam.mute_on().await?;
                    Ok(state_attrs(true))
                }
            }
        }
    })
    .await;

    match result {
        Ok(Ok(attrs)) => Ok(attrs),
        Ok(Err(e)) => Err(ArcamIntegrationError::Arcam(e)),
        Err(_) => Err(ArcamIntegrationError::Timeout),
    }
}

fn state_attrs(on: bool) -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!(if on { "ON" } else { "OFF" }))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_attrs_on() {
        assert_eq!(state_attrs(true)["state"], "ON");
    }

    #[test]
    fn test_state_attrs_off() {
        assert_eq!(state_attrs(false)["state"], "OFF");
    }
}
