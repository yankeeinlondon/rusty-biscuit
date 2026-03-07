//! UC Integration <-> Sony receiver bridge types.
//!
//! Pure data types that map between Sony receiver state and Unfolded Circle
//! entity representations. No IO.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Driver metadata constants
pub const DRIVER_ID: &str = "sony-receiver";
pub const DRIVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_CORE_API: &str = "0.14.0";

/// UC device connection states.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceState {
    Connecting,
    Connected,
    Disconnected,
    Error,
}

/// UC entity description advertised to the Remote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonyEntity {
    pub entity_id: String,
    pub entity_type: String,
    pub name: HashMap<String, String>,
    pub features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// Current state snapshot of a Sony entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonyEntityState {
    pub entity_id: String,
    pub entity_type: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Sony operation resolved from a UC entity command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SonyOperation {
    PowerOn,
    PowerOff,
    PowerToggle,
    MuteOn,
    MuteOff,
    MuteToggle,
    VolumeSet(u32),
    VolumeUp,
    VolumeDown,
    SelectSource(String),
}

/// Input source definitions used for the media_player source_list.
pub const SOURCE_CATEGORIES: &[&str] = &["GAME", "STB", "BD", "SAT", "VIDEO", "AUX", "TV", "CD"];

/// Build the available entities for a given Sony receiver device name.
pub fn build_entities(device_name: &str) -> Vec<SonyEntity> {
    vec![
        SonyEntity {
            entity_id: format!("sony.{device_name}.power"),
            entity_type: "switch".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Power"))]),
            features: vec!["on_off".to_string(), "toggle".to_string()],
            options: None,
        },
        SonyEntity {
            entity_id: format!("sony.{device_name}.receiver"),
            entity_type: "media_player".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Receiver"))]),
            features: vec![
                "volume".to_string(),
                "volume_up_down".to_string(),
                "mute".to_string(),
                "mute_toggle".to_string(),
                "select_source".to_string(),
            ],
            options: Some(HashMap::from([(
                "source_list".to_string(),
                serde_json::json!(SOURCE_CATEGORIES),
            )])),
        },
    ]
}

/// Build initial entity states (unknown until polled).
pub fn build_initial_states(device_name: &str) -> Vec<SonyEntityState> {
    vec![
        SonyEntityState {
            entity_id: format!("sony.{device_name}.power"),
            entity_type: "switch".to_string(),
            attributes: HashMap::from([("state".to_string(), serde_json::json!("UNKNOWN"))]),
        },
        SonyEntityState {
            entity_id: format!("sony.{device_name}.receiver"),
            entity_type: "media_player".to_string(),
            attributes: HashMap::from([
                ("state".to_string(), serde_json::json!("UNKNOWN")),
                ("volume".to_string(), serde_json::json!(0)),
                ("muted".to_string(), serde_json::json!(false)),
                ("source".to_string(), serde_json::json!("")),
            ]),
        },
    ]
}

/// Map an entity_id + cmd_id to a Sony operation.
pub fn resolve_command(
    entity_id: &str,
    cmd_id: &str,
    params: &serde_json::Value,
) -> Option<SonyOperation> {
    let kind = entity_id.rsplit('.').next()?;

    match (kind, cmd_id) {
        ("power", "on") => Some(SonyOperation::PowerOn),
        ("power", "off") => Some(SonyOperation::PowerOff),
        ("power", "toggle") => Some(SonyOperation::PowerToggle),
        ("receiver", "volume_set") => {
            let level = params.get("volume").and_then(|v| v.as_u64())?;
            Some(SonyOperation::VolumeSet(level as u32))
        }
        ("receiver", "volume_up") => Some(SonyOperation::VolumeUp),
        ("receiver", "volume_down") => Some(SonyOperation::VolumeDown),
        ("receiver", "mute_toggle") => Some(SonyOperation::MuteToggle),
        ("receiver", "mute") => Some(SonyOperation::MuteOn),
        ("receiver", "unmute") => Some(SonyOperation::MuteOff),
        ("receiver", "select_source") => {
            let source = params.get("source").and_then(|v| v.as_str())?;
            Some(SonyOperation::SelectSource(source.to_string()))
        }
        _ => None,
    }
}

/// Extract the device name from an entity_id (`sony.{name}.{kind}`).
pub fn device_name_from_entity_id(entity_id: &str) -> Option<&str> {
    let rest = entity_id.strip_prefix("sony.")?;
    let dot = rest.rfind('.')?;
    Some(&rest[..dot])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_entities() {
        let entities = build_entities("living");
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].entity_id, "sony.living.power");
        assert_eq!(entities[0].entity_type, "switch");
        assert_eq!(entities[1].entity_id, "sony.living.receiver");
        assert_eq!(entities[1].entity_type, "media_player");
        assert!(entities[1].features.contains(&"volume".to_string()));
        assert!(entities[1].features.contains(&"select_source".to_string()));
    }

    #[test]
    fn test_build_initial_states() {
        let states = build_initial_states("living");
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].attributes["state"], "UNKNOWN");
        assert_eq!(states[1].attributes["volume"], 0);
    }

    #[test]
    fn test_resolve_command_power() {
        let empty = json!({});
        assert_eq!(
            resolve_command("sony.living.power", "on", &empty),
            Some(SonyOperation::PowerOn)
        );
        assert_eq!(
            resolve_command("sony.living.power", "off", &empty),
            Some(SonyOperation::PowerOff)
        );
        assert_eq!(
            resolve_command("sony.living.power", "toggle", &empty),
            Some(SonyOperation::PowerToggle)
        );
    }

    #[test]
    fn test_resolve_command_volume() {
        let params = json!({"volume": 42});
        assert_eq!(
            resolve_command("sony.living.receiver", "volume_set", &params),
            Some(SonyOperation::VolumeSet(42))
        );
        let empty = json!({});
        assert_eq!(
            resolve_command("sony.living.receiver", "volume_up", &empty),
            Some(SonyOperation::VolumeUp)
        );
        assert_eq!(
            resolve_command("sony.living.receiver", "volume_down", &empty),
            Some(SonyOperation::VolumeDown)
        );
    }

    #[test]
    fn test_resolve_command_mute() {
        let empty = json!({});
        assert_eq!(
            resolve_command("sony.living.receiver", "mute", &empty),
            Some(SonyOperation::MuteOn)
        );
        assert_eq!(
            resolve_command("sony.living.receiver", "unmute", &empty),
            Some(SonyOperation::MuteOff)
        );
        assert_eq!(
            resolve_command("sony.living.receiver", "mute_toggle", &empty),
            Some(SonyOperation::MuteToggle)
        );
    }

    #[test]
    fn test_resolve_command_select_source() {
        let params = json!({"source": "GAME"});
        assert_eq!(
            resolve_command("sony.living.receiver", "select_source", &params),
            Some(SonyOperation::SelectSource("GAME".to_string()))
        );
    }

    #[test]
    fn test_resolve_command_unknown() {
        let empty = json!({});
        assert_eq!(
            resolve_command("sony.living.power", "invalid", &empty),
            None
        );
        assert_eq!(resolve_command("sony.living.unknown", "on", &empty), None);
    }

    #[test]
    fn test_device_name_from_entity_id() {
        assert_eq!(
            device_name_from_entity_id("sony.living.power"),
            Some("living")
        );
        assert_eq!(
            device_name_from_entity_id("sony.living-room.receiver"),
            Some("living-room")
        );
        assert_eq!(device_name_from_entity_id("invalid"), None);
    }

    #[test]
    fn test_device_state_serialization() {
        assert_eq!(
            serde_json::to_string(&DeviceState::Connected).unwrap(),
            "\"CONNECTED\""
        );
        assert_eq!(
            serde_json::to_string(&DeviceState::Disconnected).unwrap(),
            "\"DISCONNECTED\""
        );
    }
}
