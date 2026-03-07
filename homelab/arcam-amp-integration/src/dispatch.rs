//! Arcam TCP command execution.

use std::collections::HashMap;
use std::time::Duration;

use homelab::arcam::Arcam;
use homelab::network::parse_host;
use serde_json::{Value, json};
use unfolded_integration_helper::Attributes;

use crate::error::ArcamIntegrationError;
use crate::types::ArcamOperation;

/// Full amplifier snapshot used to refresh UC entities.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcamSnapshot {
    pub power_attributes: Attributes,
    pub mute_attributes: Attributes,
}

/// Execute an Arcam operation.
pub async fn execute_operation(
    host: &str,
    port: u16,
    operation: ArcamOperation,
    timeout: Duration,
) -> Result<(), ArcamIntegrationError> {
    let arcam_host = parse_host(host).map_err(|e| ArcamIntegrationError::InvalidHost(e.0))?;
    let arcam = Arcam::new(arcam_host, port);

    let result = tokio::time::timeout(timeout, async {
        match operation {
            ArcamOperation::PowerOn => {
                arcam.power_on().await?;
                Ok(())
            }
            ArcamOperation::PowerOff => {
                arcam.power_off().await?;
                Ok(())
            }
            ArcamOperation::PowerToggle => {
                let current = arcam.request_power_state().await?;
                if current {
                    arcam.power_off().await?;
                } else {
                    arcam.power_on().await?;
                }
                Ok(())
            }
            ArcamOperation::MuteOn => {
                arcam.mute_on().await?;
                Ok(())
            }
            ArcamOperation::MuteOff => {
                arcam.mute_off().await?;
                Ok(())
            }
            ArcamOperation::MuteToggle => {
                let current = arcam.get_mute_status().await?;
                if current {
                    arcam.mute_off().await?;
                } else {
                    arcam.mute_on().await?;
                }
                Ok(())
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

/// Fetch the current amplifier power and mute state.
pub async fn fetch_snapshot(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<ArcamSnapshot, ArcamIntegrationError> {
    let arcam_host = parse_host(host).map_err(|e| ArcamIntegrationError::InvalidHost(e.0))?;
    let arcam = Arcam::new(arcam_host, port);

    let result = tokio::time::timeout(timeout, async {
        let (power, muted) =
            tokio::try_join!(arcam.request_power_state(), arcam.get_mute_status())?;

        Ok(ArcamSnapshot {
            power_attributes: state_attrs(power),
            mute_attributes: state_attrs(muted),
        })
    })
    .await;

    match result {
        Ok(Ok(snapshot)) => Ok(snapshot),
        Ok(Err(e)) => Err(ArcamIntegrationError::Arcam(e)),
        Err(_) => Err(ArcamIntegrationError::Timeout),
    }
}

pub fn state_attrs(on: bool) -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!(if on { "ON" } else { "OFF" }))])
}

pub fn unknown_state_attrs() -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!("UNKNOWN"))])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ArcamOperation;
    use std::env;
    use std::time::Duration;

    #[test]
    fn test_state_attrs_on() {
        assert_eq!(state_attrs(true)["state"], "ON");
    }

    #[test]
    fn test_state_attrs_off() {
        assert_eq!(state_attrs(false)["state"], "OFF");
    }

    #[test]
    fn test_unknown_state_attrs() {
        assert_eq!(unknown_state_attrs()["state"], "UNKNOWN");
    }

    fn arcam_real_target() -> Option<(String, u16)> {
        let host = env::var("ARCAM_REAL_HOST").ok()?;
        let port = env::var("ARCAM_REAL_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(50000);
        Some((host, port))
    }

    fn require_arcam_destructive() {
        let enabled = env::var("ARCAM_REAL_ALLOW_DESTRUCTIVE").ok();
        assert_eq!(
            enabled.as_deref(),
            Some("1"),
            "set ARCAM_REAL_ALLOW_DESTRUCTIVE=1 to run this test"
        );
    }

    #[tokio::test]
    #[ignore = "requires a reachable Arcam amplifier configured via ARCAM_REAL_HOST"]
    async fn real_device_fetch_snapshot_returns_power_and_mute() {
        let (host, port) = arcam_real_target().expect("set ARCAM_REAL_HOST to run this test");
        let snapshot = fetch_snapshot(&host, port, Duration::from_secs(5))
            .await
            .expect("real Arcam snapshot fetch should succeed");

        assert!(snapshot.power_attributes.contains_key("state"));
        assert!(snapshot.mute_attributes.contains_key("state"));
    }

    #[tokio::test]
    #[ignore = "requires a reachable Arcam amplifier and writes mute state before restoring it"]
    async fn destructive_real_device_mute_command_roundtrips() {
        require_arcam_destructive();
        let (host, port) = arcam_real_target().expect("set ARCAM_REAL_HOST to run this test");
        let timeout = Duration::from_secs(5);
        let before = fetch_snapshot(&host, port, timeout)
            .await
            .expect("initial Arcam snapshot should succeed");
        let was_muted = before
            .mute_attributes
            .get("state")
            .and_then(Value::as_str)
            .expect("mute state should be present")
            == "ON";

        let target = if was_muted {
            ArcamOperation::MuteOff
        } else {
            ArcamOperation::MuteOn
        };
        let restore = if was_muted {
            ArcamOperation::MuteOn
        } else {
            ArcamOperation::MuteOff
        };

        execute_operation(&host, port, target, timeout)
            .await
            .expect("mute operation should succeed");
        let changed = fetch_snapshot(&host, port, timeout)
            .await
            .expect("post-mute snapshot should succeed");
        let now_muted = changed
            .mute_attributes
            .get("state")
            .and_then(Value::as_str)
            .expect("mute state should be present")
            == "ON";
        assert_ne!(now_muted, was_muted);

        execute_operation(&host, port, restore, timeout)
            .await
            .expect("mute restoration should succeed");
    }
}
