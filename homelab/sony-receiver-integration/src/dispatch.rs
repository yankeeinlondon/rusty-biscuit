//! Sony receiver HTTP command execution.

use std::collections::HashMap;
use std::time::Duration;

use homelab::network::parse_host;
use homelab::sony_receiver::{InputSource, NativeInputConfig, PlayingContentInfo, SonyReceiver};
use serde_json::{Value, json};
use unfolded_integration_helper::Attributes;

use crate::error::SonyIntegrationError;
use crate::types::{SOURCE_CATEGORIES, SonyOperation};

/// Full receiver snapshot used to populate UC entity state.
#[derive(Debug, Clone, PartialEq)]
pub struct SonySnapshot {
    pub power_attributes: Attributes,
    pub receiver_attributes: Attributes,
}

/// Execute a Sony receiver operation and return updated entity state attributes.
///
/// Returns a tuple of (entity_type_suffix, attributes) where the suffix is
/// "power" or "receiver" to identify which entity was affected.
pub async fn execute_operation(
    host: &str,
    port: u16,
    operation: SonyOperation,
    timeout: Duration,
) -> Result<(), SonyIntegrationError> {
    let sony_host = parse_host(host).map_err(|e| SonyIntegrationError::InvalidHost(e.0))?;
    let sony = SonyReceiver::new(sony_host, port);

    let result = tokio::time::timeout(timeout, async {
        match operation {
            SonyOperation::PowerOn => {
                sony.set_power(true).await?;
                Ok(())
            }
            SonyOperation::PowerOff => {
                sony.set_power(false).await?;
                Ok(())
            }
            SonyOperation::PowerToggle => {
                let status = sony.get_power_status().await?;
                let is_on = status == "on" || status == "active";
                sony.set_power(!is_on).await?;
                Ok(())
            }
            SonyOperation::MuteOn => {
                sony.set_mute(true).await?;
                Ok(())
            }
            SonyOperation::MuteOff => {
                sony.set_mute(false).await?;
                Ok(())
            }
            SonyOperation::MuteToggle => {
                let current = sony.get_mute_status().await?;
                sony.set_mute(!current).await?;
                Ok(())
            }
            SonyOperation::VolumeSet(level) => {
                sony.set_volume(level).await?;
                Ok(())
            }
            SonyOperation::VolumeUp => {
                let info = sony.get_volume().await?;
                let new_level = (info.volume + 1).min(info.max_volume);
                sony.set_volume(new_level).await?;
                Ok(())
            }
            SonyOperation::VolumeDown => {
                let info = sony.get_volume().await?;
                let new_level = info.volume.saturating_sub(1).max(info.min_volume);
                sony.set_volume(new_level).await?;
                Ok(())
            }
            SonyOperation::SelectSource(ref category) => {
                let (native, inputs) = tokio::join!(sony.get_native_inputs(), sony.list_inputs(),);
                let native = native?;
                let inputs = inputs?;

                let uri = resolve_category_to_uri(&native, &inputs, category)?;
                sony.set_input(&uri).await?;
                Ok(())
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

/// Fetch a full receiver snapshot for both UC entities.
pub async fn fetch_receiver_state(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<SonySnapshot, SonyIntegrationError> {
    let sony_host = parse_host(host).map_err(|e| SonyIntegrationError::InvalidHost(e.0))?;
    let sony = SonyReceiver::new(sony_host, port);

    let result = tokio::time::timeout(timeout, async {
        let power = sony.get_power_status().await?;
        let is_on = power == "on" || power == "active";

        let power_attributes = power_attrs(is_on);
        let mut receiver_attributes = offline_receiver_attrs();
        receiver_attributes.insert("state".to_string(), json!(if is_on { "ON" } else { "OFF" }));

        if is_on {
            let (volume_info, mute_status, current_input, native, inputs) = tokio::try_join!(
                sony.get_volume(),
                sony.get_mute_status(),
                sony.get_current_input(),
                sony.get_native_inputs(),
                sony.list_inputs(),
            )?;

            receiver_attributes.insert("volume".to_string(), json!(volume_info.volume));
            receiver_attributes.insert("muted".to_string(), json!(mute_status));
            receiver_attributes.insert(
                "source".to_string(),
                json!(
                    resolve_uri_to_category(&native, &inputs, &current_input).unwrap_or_default()
                ),
            );
        }

        Ok(SonySnapshot {
            power_attributes,
            receiver_attributes,
        })
    })
    .await;

    match result {
        Ok(Ok(attrs)) => Ok(attrs),
        Ok(Err(e)) => Err(SonyIntegrationError::Sony(e)),
        Err(_) => Err(SonyIntegrationError::Timeout),
    }
}

/// Fetch the currently valid source categories for entity advertisement.
pub async fn fetch_source_list(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<Vec<String>, SonyIntegrationError> {
    let sony_host = parse_host(host).map_err(|e| SonyIntegrationError::InvalidHost(e.0))?;
    let sony = SonyReceiver::new(sony_host, port);

    let result = tokio::time::timeout(timeout, async {
        let (native, inputs) = tokio::try_join!(sony.get_native_inputs(), sony.list_inputs())?;
        Ok::<_, homelab::sony_receiver::SonyError>(available_source_categories(&native, &inputs))
    })
    .await;

    match result {
        Ok(Ok(source_list)) => Ok(source_list),
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

fn resolve_uri_to_category(
    native: &[NativeInputConfig],
    inputs: &[InputSource],
    current_input: &PlayingContentInfo,
) -> Option<String> {
    let uri = current_input.uri.as_str();

    if let Some(category) = native.iter().find_map(|item| {
        let matches_hdmi = extract_hdmi_port(&item.hdmi_assign)
            .map(|port| format!("extInput:hdmi?port={port}"))
            == Some(uri.to_string());
        let matches_name = item
            .name
            .eq_ignore_ascii_case(current_input.title.as_deref().unwrap_or(""));
        let matches_category = item.category.eq_ignore_ascii_case(
            current_input
                .source
                .as_deref()
                .unwrap_or(current_input.title.as_deref().unwrap_or("")),
        );

        if matches_hdmi || matches_name || matches_category {
            Some(item.category.clone())
        } else {
            None
        }
    }) {
        return Some(category);
    }

    inputs.iter().find_map(|input| {
        if input.uri == uri {
            SOURCE_CATEGORIES
                .iter()
                .find(|category| input.title.eq_ignore_ascii_case(category))
                .map(|category| (*category).to_string())
        } else {
            None
        }
    })
}

fn available_source_categories(native: &[NativeInputConfig], inputs: &[InputSource]) -> Vec<String> {
    let mut source_list = Vec::new();

    for item in native {
        if !item.visible {
            continue;
        }

        if resolve_category_to_uri(native, inputs, &item.category).is_ok()
            && !source_list.contains(&item.category)
        {
            source_list.push(item.category.clone());
        }
    }

    for input in inputs {
        if let Some(category) = SOURCE_CATEGORIES
            .iter()
            .find(|category| input.title.eq_ignore_ascii_case(category))
            .map(|category| (*category).to_string())
        {
            if !source_list.contains(&category) {
                source_list.push(category);
            }
        }
    }

    source_list
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

pub fn unknown_power_attrs() -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!("UNKNOWN"))])
}

pub fn offline_receiver_attrs() -> HashMap<String, Value> {
    HashMap::from([
        ("state".to_string(), json!("UNKNOWN")),
        ("volume".to_string(), json!(0)),
        ("muted".to_string(), json!(false)),
        ("source".to_string(), json!("")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SonyOperation;
    use homelab::{network::parse_host, sony_receiver::SonyReceiver};
    use std::env;
    use std::time::Duration;

    #[test]
    fn test_power_attrs() {
        assert_eq!(power_attrs(true)["state"], "ON");
        assert_eq!(power_attrs(false)["state"], "OFF");
    }

    #[test]
    fn test_unknown_power_attrs() {
        assert_eq!(unknown_power_attrs()["state"], "UNKNOWN");
    }

    #[test]
    fn test_offline_receiver_attrs() {
        let attrs = offline_receiver_attrs();
        assert_eq!(attrs["state"], "UNKNOWN");
        assert_eq!(attrs["source"], "");
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

    #[test]
    fn test_resolve_uri_to_category() {
        let native = vec![NativeInputConfig {
            category: "GAME".to_string(),
            name: "PlayStation".to_string(),
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
        let inputs = vec![InputSource {
            title: "HDMI 1".to_string(),
            uri: "extInput:hdmi?port=1".to_string(),
            icon_url: None,
            connection: None,
            label: None,
            active: None,
        }];
        let current = PlayingContentInfo {
            uri: "extInput:hdmi?port=1".to_string(),
            source: Some("GAME".to_string()),
            title: Some("PlayStation".to_string()),
            state: None,
            content_kind: None,
        };

        assert_eq!(
            resolve_uri_to_category(&native, &inputs, &current).as_deref(),
            Some("GAME")
        );
    }

    #[test]
    fn test_available_source_categories_only_includes_visible_resolvable_inputs() {
        let native = vec![
            NativeInputConfig {
                category: "GAME".to_string(),
                name: "Game".to_string(),
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
            },
            NativeInputConfig {
                category: "SAT".to_string(),
                name: "Sat".to_string(),
                hdmi_assign: "in2".to_string(),
                icon: "sat".to_string(),
                visible: false,
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
            },
        ];
        let inputs = vec![
            InputSource {
                title: "HDMI 1".to_string(),
                uri: "extInput:hdmi?port=1".to_string(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
            InputSource {
                title: "TV".to_string(),
                uri: "tv:dvbt".to_string(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
        ];

        assert_eq!(available_source_categories(&native, &inputs), vec!["GAME", "TV"]);
    }

    fn sony_real_target() -> Option<(String, u16)> {
        let host = env::var("SONY_REAL_HOST").ok()?;
        let port = env::var("SONY_REAL_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(10000);
        Some((host, port))
    }

    fn require_sony_destructive() {
        let enabled = env::var("SONY_REAL_ALLOW_DESTRUCTIVE").ok();
        assert_eq!(
            enabled.as_deref(),
            Some("1"),
            "set SONY_REAL_ALLOW_DESTRUCTIVE=1 to run this test"
        );
    }

    #[tokio::test]
    #[ignore = "requires a reachable Sony receiver configured via SONY_REAL_HOST"]
    async fn real_device_fetch_receiver_state_returns_full_snapshot() {
        let (host, port) = sony_real_target().expect("set SONY_REAL_HOST to run this test");
        let snapshot = fetch_receiver_state(&host, port, Duration::from_secs(10))
            .await
            .expect("real Sony snapshot fetch should succeed");

        assert!(snapshot.power_attributes.contains_key("state"));
        assert!(snapshot.receiver_attributes.contains_key("state"));
        assert!(snapshot.receiver_attributes.contains_key("volume"));
        assert!(snapshot.receiver_attributes.contains_key("muted"));
        assert!(snapshot.receiver_attributes.contains_key("source"));
    }

    #[tokio::test]
    #[ignore = "requires a reachable Sony receiver and writes volume before restoring it"]
    async fn destructive_real_device_volume_command_roundtrips() {
        require_sony_destructive();
        let (host, port) = sony_real_target().expect("set SONY_REAL_HOST to run this test");
        let timeout = Duration::from_secs(10);
        let before = fetch_receiver_state(&host, port, timeout)
            .await
            .expect("initial Sony snapshot should succeed");
        let current = before
            .receiver_attributes
            .get("volume")
            .and_then(Value::as_u64)
            .expect("volume should be present") as u32;
        if current == 0 {
            eprintln!("Skipping Sony destructive volume test because current volume is already 0");
            return;
        }
        let target = current - 1;

        execute_operation(&host, port, SonyOperation::VolumeSet(target), timeout)
            .await
            .expect("Sony volume change should succeed");
        let changed = fetch_receiver_state(&host, port, timeout)
            .await
            .expect("post-volume snapshot should succeed");
        let observed = changed
            .receiver_attributes
            .get("volume")
            .and_then(Value::as_u64)
            .expect("volume should be present") as u32;
        assert_eq!(observed, target);

        execute_operation(&host, port, SonyOperation::VolumeSet(current), timeout)
            .await
            .expect("Sony volume restoration should succeed");
    }

    #[tokio::test]
    #[ignore = "requires a reachable Sony receiver and switches source before restoring it"]
    async fn destructive_real_device_source_switch_roundtrips() {
        require_sony_destructive();
        let (host, port) = sony_real_target().expect("set SONY_REAL_HOST to run this test");
        let timeout = Duration::from_secs(10);
        let before = fetch_receiver_state(&host, port, timeout)
            .await
            .expect("initial Sony snapshot should succeed");
        if before
            .receiver_attributes
            .get("state")
            .and_then(Value::as_str)
            != Some("ON")
        {
            eprintln!("Skipping Sony destructive source test because the receiver is not ON");
            return;
        }

        let sony_host = parse_host(&host).expect("real Sony host should parse");
        let sony = SonyReceiver::new(sony_host, port);
        let (native, inputs, current_input) = tokio::try_join!(
            sony.get_native_inputs(),
            sony.list_inputs(),
            sony.get_current_input(),
        )
        .expect("Sony input discovery should succeed");

        let Some(current_category) = resolve_uri_to_category(&native, &inputs, &current_input)
        else {
            eprintln!(
                "Skipping Sony destructive source test because the current source category could not be resolved"
            );
            return;
        };

        let Some(target_category) = native.iter().find_map(|item| {
            if item.category == current_category {
                return None;
            }
            resolve_category_to_uri(&native, &inputs, &item.category)
                .ok()
                .map(|_| item.category.clone())
        }) else {
            eprintln!(
                "Skipping Sony destructive source test because no alternate source category is available"
            );
            return;
        };

        execute_operation(
            &host,
            port,
            SonyOperation::SelectSource(target_category.clone()),
            timeout,
        )
        .await
        .expect("Sony source change should succeed");
        let changed = fetch_receiver_state(&host, port, timeout)
            .await
            .expect("post-source snapshot should succeed");
        let observed = changed
            .receiver_attributes
            .get("source")
            .and_then(Value::as_str)
            .expect("source should be present");
        assert_eq!(observed, target_category);

        execute_operation(
            &host,
            port,
            SonyOperation::SelectSource(current_category),
            timeout,
        )
        .await
        .expect("Sony source restoration should succeed");
    }
}
