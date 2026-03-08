//! UC Integration <-> Sony receiver bridge types.
//!
//! Pure data types that map between Sony receiver state and Unfolded Circle
//! entity representations. No IO.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use unfolded_integration_helper::{EntityState, device_selection_schema};

/// Driver metadata constants
pub const DRIVER_ID: &str = "sony-receiver";
pub const DRIVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_CORE_API: &str = "0.14.0";

/// Developer details presented in the UC configurator.
#[derive(Debug, Clone, Serialize)]
pub struct DriverDeveloper {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Driver metadata returned to the UC configurator.
#[derive(Debug, Clone, Serialize)]
pub struct DriverMetadata {
    pub driver_id: String,
    pub name: HashMap<String, String>,
    pub version: String,
    pub min_core_api: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer: Option<DriverDeveloper>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_data_schema: Option<Value>,
}

/// Build the metadata payload required by the UC configurator.
pub fn driver_metadata() -> DriverMetadata {
    DriverMetadata {
        driver_id: DRIVER_ID.to_string(),
        name: HashMap::from([("en".to_string(), "Sony Receiver".to_string())]),
        version: DRIVER_VERSION.to_string(),
        min_core_api: MIN_CORE_API.to_string(),
        icon: Some("uc:integration".to_string()),
        description: Some(HashMap::from([(
            "en".to_string(),
            "Control Sony ES receivers over the local network.".to_string(),
        )])),
        developer: Some(DriverDeveloper {
            name: "Ken Snyder".to_string(),
            email: Some("ken@ken.net".to_string()),
            url: Some("https://github.com/yankeeinlondon".to_string()),
        }),
        home_page: Some("https://github.com/yankeeinlondon/rusty-biscuit".to_string()),
        setup_data_schema: Some(device_selection_schema(&[], &[], "", &[])),
    }
}

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
pub fn build_entities(device_name: &str, source_list: &[String]) -> Vec<SonyEntity> {
    vec![
        SonyEntity {
            entity_id: format!("sony.{device_name}.power"),
            entity_type: "switch".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Power"))]),
            features: vec!["on_off".to_string()],
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
                serde_json::json!(source_list),
            )])),
        },
    ]
}

/// Build initial entity states (unknown until polled).
pub fn build_initial_states(device_name: &str) -> Vec<EntityState> {
    vec![
        EntityState::new(
            format!("sony.{device_name}.power"),
            "switch",
            HashMap::from([("state".to_string(), serde_json::json!("UNKNOWN"))]),
        ),
        EntityState::new(
            format!("sony.{device_name}.receiver"),
            "media_player",
            HashMap::from([
                ("state".to_string(), serde_json::json!("UNKNOWN")),
                ("volume".to_string(), serde_json::json!(0)),
                ("muted".to_string(), serde_json::json!(false)),
                ("source".to_string(), serde_json::json!("")),
            ]),
        ),
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
        let entities = build_entities("living", &["GAME".to_string(), "TV".to_string()]);
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].entity_id, "sony.living.power");
        assert_eq!(entities[0].entity_type, "switch");
        assert_eq!(entities[1].entity_id, "sony.living.receiver");
        assert_eq!(entities[1].entity_type, "media_player");
        assert!(entities[1].features.contains(&"volume".to_string()));
        assert!(entities[1].features.contains(&"select_source".to_string()));
        assert_eq!(
            entities[1].options.as_ref().unwrap()["source_list"],
            serde_json::json!(["GAME", "TV"])
        );
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

    #[test]
    fn test_driver_metadata() {
        let metadata = driver_metadata();
        assert_eq!(metadata.driver_id, DRIVER_ID);
        assert_eq!(metadata.name["en"], "Sony Receiver");
        assert_eq!(metadata.version, DRIVER_VERSION);
        assert_eq!(metadata.min_core_api, MIN_CORE_API);
        assert_eq!(metadata.developer.as_ref().unwrap().name, "Ken Snyder");
    }
}
