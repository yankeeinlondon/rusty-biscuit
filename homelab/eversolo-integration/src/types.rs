//! UC Integration <-> Eversolo bridge types.
//!
//! Pure data types that map between Eversolo device state and Unfolded Circle
//! entity representations. No IO.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use unfolded_integration_helper::{EntityState, device_selection_schema};

/// Driver metadata constants.
pub const DRIVER_ID: &str = "eversolo-streamer";
pub const DRIVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_CORE_API: &str = "0.14.0";
pub const DEFAULT_VOLUME_STEPS: u32 = 200;

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
        name: HashMap::from([("en".to_string(), "Eversolo Streamer".to_string())]),
        version: DRIVER_VERSION.to_string(),
        min_core_api: MIN_CORE_API.to_string(),
        icon: Some("uc:integration".to_string()),
        description: Some(HashMap::from([(
            "en".to_string(),
            "Control Eversolo streamers over the local network.".to_string(),
        )])),
        developer: Some(DriverDeveloper {
            name: "Ken Snyder".to_string(),
            email: Some("ken@yankeeinlondon.com".to_string()),
            url: Some("https://github.com/yankeeinlondon".to_string()),
        }),
        home_page: Some("https://github.com/yankeeinlondon/rusty-biscuit".to_string()),
        setup_data_schema: Some(device_selection_schema(&[], &[], "", &[])),
    }
}

/// Eversolo connection details used by the transport layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    pub host: String,
    pub port: u16,
    pub mac_address: Option<String>,
    pub wol_broadcast: String,
    pub wol_port: u16,
}

/// UC entity description advertised to the Remote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EversoloEntity {
    pub entity_id: String,
    pub entity_type: String,
    pub name: HashMap<String, String>,
    pub features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, Value>>,
}

/// Eversolo operation resolved from a UC entity command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EversoloOperation {
    PowerOn,
    PowerOff,
    PowerToggle,
    PlayPause,
    Next,
    Previous,
    VolumeSet(i32),
    VolumeUp,
    VolumeDown,
    MuteOn,
    MuteOff,
    MuteToggle,
    SelectSource(String),
}

/// Build the available entities for a given Eversolo device name.
pub fn build_entities(
    device_name: &str,
    source_list: &[String],
    volume_steps: u32,
) -> Vec<EversoloEntity> {
    vec![
        EversoloEntity {
            entity_id: format!("eversolo.{device_name}.power"),
            entity_type: "switch".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Power"))]),
            features: vec!["on_off".to_string()],
            options: None,
        },
        EversoloEntity {
            entity_id: format!("eversolo.{device_name}.player"),
            entity_type: "media_player".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Player"))]),
            features: vec![
                "volume".to_string(),
                "volume_up_down".to_string(),
                "mute".to_string(),
                "unmute".to_string(),
                "mute_toggle".to_string(),
                "play_pause".to_string(),
                "next".to_string(),
                "previous".to_string(),
                "select_source".to_string(),
                "media_duration".to_string(),
                "media_position".to_string(),
                "media_title".to_string(),
                "media_artist".to_string(),
                "media_album".to_string(),
            ],
            options: Some(HashMap::from([
                ("source_list".to_string(), serde_json::json!(source_list)),
                (
                    "volume_steps".to_string(),
                    serde_json::json!(volume_steps.max(2)),
                ),
            ])),
        },
    ]
}

/// Build initial entity states (unknown until polled).
pub fn build_initial_states(device_name: &str) -> Vec<EntityState> {
    vec![
        EntityState::new(
            format!("eversolo.{device_name}.power"),
            "switch",
            HashMap::from([("state".to_string(), serde_json::json!("UNKNOWN"))]),
        ),
        EntityState::new(
            format!("eversolo.{device_name}.player"),
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

/// Map an entity_id + cmd_id to an Eversolo operation.
pub fn resolve_command(entity_id: &str, cmd_id: &str, params: &Value) -> Option<EversoloOperation> {
    let kind = entity_id.rsplit('.').next()?;

    match (kind, cmd_id) {
        ("power", "on") => Some(EversoloOperation::PowerOn),
        ("power", "off") => Some(EversoloOperation::PowerOff),
        ("power", "toggle") => Some(EversoloOperation::PowerToggle),
        ("player", "play_pause") => Some(EversoloOperation::PlayPause),
        ("player", "next") => Some(EversoloOperation::Next),
        ("player", "previous") => Some(EversoloOperation::Previous),
        ("player", "volume_set") => {
            let volume = i32::try_from(params.get("volume")?.as_i64()?).ok()?;
            Some(EversoloOperation::VolumeSet(volume))
        }
        ("player", "volume_up") => Some(EversoloOperation::VolumeUp),
        ("player", "volume_down") => Some(EversoloOperation::VolumeDown),
        ("player", "mute") => Some(EversoloOperation::MuteOn),
        ("player", "unmute") => Some(EversoloOperation::MuteOff),
        ("player", "mute_toggle") => Some(EversoloOperation::MuteToggle),
        ("player", "select_source") => {
            let source = params.get("source")?.as_str()?.to_string();
            Some(EversoloOperation::SelectSource(source))
        }
        _ => None,
    }
}

/// Extract the device name from an entity_id (`eversolo.{name}.{kind}`).
pub fn device_name_from_entity_id(entity_id: &str) -> Option<&str> {
    let rest = entity_id.strip_prefix("eversolo.")?;
    let dot = rest.rfind('.')?;
    Some(&rest[..dot])
}

/// Return whether an entity id belongs to this integration.
pub fn entity_exists(entity_id: &str) -> bool {
    matches!(entity_id.rsplit('.').next(), Some("power") | Some("player"))
        && entity_id.starts_with("eversolo.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_entities() {
        let sources = vec!["Internal Player".to_string(), "USB".to_string()];
        let entities = build_entities("living", &sources, 160);
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].entity_id, "eversolo.living.power");
        assert_eq!(entities[1].entity_id, "eversolo.living.player");
        assert_eq!(entities[1].entity_type, "media_player");
        assert_eq!(entities[1].options.as_ref().unwrap()["volume_steps"], 160);
    }

    #[test]
    fn test_build_initial_states() {
        let states = build_initial_states("living");
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].attributes["state"], "UNKNOWN");
        assert_eq!(states[1].attributes["source"], "");
    }

    #[test]
    fn test_resolve_command_power() {
        let empty = json!({});
        assert_eq!(
            resolve_command("eversolo.living.power", "on", &empty),
            Some(EversoloOperation::PowerOn)
        );
        assert_eq!(
            resolve_command("eversolo.living.power", "toggle", &empty),
            Some(EversoloOperation::PowerToggle)
        );
    }

    #[test]
    fn test_resolve_command_player() {
        let empty = json!({});
        assert_eq!(
            resolve_command("eversolo.living.player", "play_pause", &empty),
            Some(EversoloOperation::PlayPause)
        );
        assert_eq!(
            resolve_command("eversolo.living.player", "next", &empty),
            Some(EversoloOperation::Next)
        );
    }

    #[test]
    fn test_resolve_command_with_params() {
        assert_eq!(
            resolve_command(
                "eversolo.living.player",
                "volume_set",
                &json!({"volume": 42})
            ),
            Some(EversoloOperation::VolumeSet(42))
        );
        assert_eq!(
            resolve_command(
                "eversolo.living.player",
                "select_source",
                &json!({"source": "USB"})
            ),
            Some(EversoloOperation::SelectSource("USB".to_string()))
        );
    }

    #[test]
    fn test_device_name_from_entity_id() {
        assert_eq!(
            device_name_from_entity_id("eversolo.main-room.player"),
            Some("main-room")
        );
        assert_eq!(device_name_from_entity_id("invalid"), None);
    }

    #[test]
    fn test_entity_exists() {
        assert!(entity_exists("eversolo.living.power"));
        assert!(entity_exists("eversolo.living.player"));
        assert!(!entity_exists("eversolo.living.output"));
        assert!(!entity_exists("sony.living.power"));
    }

    #[test]
    fn test_driver_metadata() {
        let metadata = driver_metadata();
        assert_eq!(metadata.driver_id, DRIVER_ID);
        assert_eq!(metadata.name["en"], "Eversolo Streamer");
        assert_eq!(metadata.version, DRIVER_VERSION);
        assert_eq!(metadata.min_core_api, MIN_CORE_API);
        assert_eq!(metadata.developer.as_ref().unwrap().name, "Ken Snyder");
    }
}
