//! Sony receiver HTTP command execution.

use std::collections::HashMap;
use std::time::Duration;

use homelab::network::parse_host;
use homelab::sony_receiver::SonyReceiver;
use serde_json::{Value, json};

use crate::error::SonyIntegrationError;
use crate::types::SonyOperation;

/// Execute a Sony receiver operation and return updated entity state attributes.
///
/// Returns a tuple of (entity_type_suffix, attributes) where the suffix is
/// "power" or "receiver" to identify which entity was affected.
pub async fn execute_operation(
    host: &str,
    port: u16,
    operation: SonyOperation,
    timeout: Duration,
) -> Result<(&'static str, HashMap<String, Value>), SonyIntegrationError> {
    let sony_host = parse_host(host).map_err(|e| SonyIntegrationError::InvalidHost(e.0))?;
    let sony = SonyReceiver::new(sony_host, port);

    let result = tokio::time::timeout(timeout, async {
        match operation {
            SonyOperation::PowerOn => {
                sony.set_power(true).await?;
                Ok(("power", power_attrs(true)))
            }
            SonyOperation::PowerOff => {
                sony.set_power(false).await?;
                Ok(("power", power_attrs(false)))
            }
            SonyOperation::PowerToggle => {
                let status = sony.get_power_status().await?;
                let is_on = status == "on" || status == "active";
                sony.set_power(!is_on).await?;
                Ok(("power", power_attrs(!is_on)))
            }
            SonyOperation::MuteOn => {
                sony.set_mute(true).await?;
                Ok(("receiver", mute_attrs(true)))
            }
            SonyOperation::MuteOff => {
                sony.set_mute(false).await?;
                Ok(("receiver", mute_attrs(false)))
            }
            SonyOperation::MuteToggle => {
                let current = sony.get_mute_status().await?;
                sony.set_mute(!current).await?;
                Ok(("receiver", mute_attrs(!current)))
            }
            SonyOperation::VolumeSet(level) => {
                sony.set_volume(level).await?;
                Ok(("receiver", volume_attrs(level)))
            }
            SonyOperation::VolumeUp => {
                let info = sony.get_volume().await?;
                let new_level = (info.volume + 1).min(info.max_volume);
                sony.set_volume(new_level).await?;
                Ok(("receiver", volume_attrs(new_level)))
            }
            SonyOperation::VolumeDown => {
                let info = sony.get_volume().await?;
                let new_level = info.volume.saturating_sub(1).max(info.min_volume);
                sony.set_volume(new_level).await?;
                Ok(("receiver", volume_attrs(new_level)))
            }
            SonyOperation::SelectSource(ref category) => {
                let (native, inputs) = tokio::join!(sony.get_native_inputs(), sony.list_inputs(),);
                let native = native?;
                let inputs = inputs?;

                let uri = resolve_category_to_uri(&native, &inputs, category)?;
                sony.set_input(&uri).await?;
                Ok(("receiver", source_attrs(category)))
            }
        }
    })
    .await;

    match result {
        Ok(Ok(attrs)) => Ok(attrs),
        Ok(Err(e)) => Err(SonyIntegrationError::Sony(e)),
        Err(_) => Err(SonyIntegrationError::Timeout),
    }
}

/// Fetch full receiver state for the media_player entity.
pub async fn fetch_receiver_state(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<HashMap<String, Value>, SonyIntegrationError> {
    let sony_host = parse_host(host).map_err(|e| SonyIntegrationError::InvalidHost(e.0))?;
    let sony = SonyReceiver::new(sony_host, port);

    let result = tokio::time::timeout(timeout, async {
        let (power, volume_info, mute_status) = tokio::join!(
            sony.get_power_status(),
            sony.get_volume(),
            sony.get_mute_status(),
        );

        let power = power?;
        let is_on = power == "on" || power == "active";

        let mut attrs = HashMap::new();
        attrs.insert("state".to_string(), json!(if is_on { "ON" } else { "OFF" }));

        if let Ok(info) = volume_info {
            attrs.insert("volume".to_string(), json!(info.volume));
        }
        if let Ok(muted) = mute_status {
            attrs.insert("muted".to_string(), json!(muted));
        }

        Ok(attrs)
    })
    .await;

    match result {
        Ok(Ok(attrs)) => Ok(attrs),
        Ok(Err(e)) => Err(SonyIntegrationError::Sony(e)),
        Err(_) => Err(SonyIntegrationError::Timeout),
    }
}

/// Resolve a source category (e.g. "GAME") to a Sony input URI.
fn resolve_category_to_uri(
    native: &[homelab::sony_receiver::NativeInputConfig],
    inputs: &[homelab::sony_receiver::InputSource],
    category: &str,
) -> Result<String, homelab::sony_receiver::SonyError> {
    let native_input = native.iter().find(|n| n.category == category);

    if let Some(ni) = native_input {
        // Extract HDMI port number from hdmi_assign (e.g. "in1" -> 1, "HDMI 3" -> 3)
        let hdmi_port = extract_hdmi_port(&ni.hdmi_assign);
        if let Some(port) = hdmi_port {
            let target = format!("extInput:hdmi?port={port}");
            if inputs.iter().any(|i| i.uri == target) {
                return Ok(target);
            }
        }

        // Fallback: try matching by icon against input URIs
        for input in inputs {
            if input.uri.contains(&ni.icon) {
                return Ok(input.uri.clone());
            }
        }
    }

    // Last resort: try to match category name against input titles
    for input in inputs {
        if input.title.eq_ignore_ascii_case(category) {
            return Ok(input.uri.clone());
        }
    }

    Err(homelab::sony_receiver::SonyError::InvalidResponse(format!(
        "could not resolve category \"{category}\" to an input URI"
    )))
}

/// Extract HDMI port number from native API hdmi_assign value.
fn extract_hdmi_port(hdmi_assign: &str) -> Option<u8> {
    // Handles formats: "in1", "in2", "HDMI 1", "HDMI 3"
    hdmi_assign
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
        .filter(|&port: &u8| port > 0)
}

fn power_attrs(on: bool) -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!(if on { "ON" } else { "OFF" }))])
}

fn mute_attrs(muted: bool) -> HashMap<String, Value> {
    HashMap::from([("muted".to_string(), json!(muted))])
}

fn volume_attrs(level: u32) -> HashMap<String, Value> {
    HashMap::from([("volume".to_string(), json!(level))])
}

fn source_attrs(category: &str) -> HashMap<String, Value> {
    HashMap::from([("source".to_string(), json!(category))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_attrs() {
        assert_eq!(power_attrs(true)["state"], "ON");
        assert_eq!(power_attrs(false)["state"], "OFF");
    }

    #[test]
    fn test_mute_attrs() {
        assert_eq!(mute_attrs(true)["muted"], true);
        assert_eq!(mute_attrs(false)["muted"], false);
    }

    #[test]
    fn test_volume_attrs() {
        assert_eq!(volume_attrs(42)["volume"], 42);
    }

    #[test]
    fn test_source_attrs() {
        assert_eq!(source_attrs("GAME")["source"], "GAME");
    }

    #[test]
    fn test_extract_hdmi_port() {
        assert_eq!(extract_hdmi_port("in1"), Some(1));
        assert_eq!(extract_hdmi_port("in2"), Some(2));
        assert_eq!(extract_hdmi_port("HDMI 3"), Some(3));
        assert_eq!(extract_hdmi_port("HDMI 1"), Some(1));
        assert_eq!(extract_hdmi_port("eARC/OUT A"), None);
        assert_eq!(extract_hdmi_port(""), None);
    }

    #[test]
    fn test_resolve_category_basic() {
        let native = vec![homelab::sony_receiver::NativeInputConfig {
            category: "GAME".to_string(),
            name: "GAME".to_string(),
            hdmi_assign: "in1".to_string(),
            icon: "game".to_string(),
            visible: true,
            sound_field: String::new(),
            digital_assign: String::new(),
            input_mode: String::new(),
            subwoofer_level: String::new(),
            subwoofer_lpf: String::new(),
            in_ceiling_mode: false,
            trigger_1: false,
            trigger_2: false,
            trigger_3: false,
            preset_gain: String::new(),
            av_sync: String::new(),
        }];
        let inputs = vec![homelab::sony_receiver::InputSource {
            title: "HDMI 1".to_string(),
            uri: "extInput:hdmi?port=1".to_string(),
            icon_url: None,
            connection: None,
            label: None,
            active: None,
        }];

        let result = resolve_category_to_uri(&native, &inputs, "GAME");
        assert_eq!(result.unwrap(), "extInput:hdmi?port=1");
    }
}
