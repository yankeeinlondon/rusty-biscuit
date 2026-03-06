//! UC Integration <-> Arcam bridge types.
//!
//! Pure data types that map between Arcam device state and Unfolded Circle
//! entity representations. No IO.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Driver metadata constants
pub const DRIVER_ID: &str = "arcam-amplifier";
pub const DRIVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_CORE_API: &str = "0.14.0";

/// UC device connection states (used when polling is implemented).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceState {
    Connecting,
    Connected,
    Disconnected,
    Error,
}

/// UC entity description advertised to the Remote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcamEntity {
    pub entity_id: String,
    pub entity_type: String,
    pub name: HashMap<String, String>,
    pub features: Vec<String>,
}

/// Current state snapshot of an Arcam entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcamEntityState {
    pub entity_id: String,
    pub entity_type: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Arcam operation resolved from a UC entity command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcamOperation {
    PowerOn,
    PowerOff,
    PowerToggle,
    MuteOn,
    MuteOff,
    MuteToggle,
}

/// Build the available entities for a given Arcam device config name.
pub fn build_entities(device_name: &str) -> Vec<ArcamEntity> {
    vec![
        ArcamEntity {
            entity_id: format!("arcam.{device_name}.power"),
            entity_type: "switch".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Power"))]),
            features: vec!["on_off".to_string(), "toggle".to_string()],
        },
        ArcamEntity {
            entity_id: format!("arcam.{device_name}.mute"),
            entity_type: "switch".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Mute"))]),
            features: vec!["on_off".to_string(), "toggle".to_string()],
        },
    ]
}

/// Build initial entity states (all OFF until polled).
pub fn build_initial_states(device_name: &str) -> Vec<ArcamEntityState> {
    vec![
        ArcamEntityState {
            entity_id: format!("arcam.{device_name}.power"),
            entity_type: "switch".to_string(),
            attributes: HashMap::from([("state".to_string(), serde_json::json!("OFF"))]),
        },
        ArcamEntityState {
            entity_id: format!("arcam.{device_name}.mute"),
            entity_type: "switch".to_string(),
            attributes: HashMap::from([("state".to_string(), serde_json::json!("OFF"))]),
        },
    ]
}

/// Map an entity_id + cmd_id to an Arcam operation.
pub fn resolve_command(entity_id: &str, cmd_id: &str) -> Option<ArcamOperation> {
    let kind = entity_id.rsplit('.').next()?;

    match (kind, cmd_id) {
        ("power", "on") => Some(ArcamOperation::PowerOn),
        ("power", "off") => Some(ArcamOperation::PowerOff),
        ("power", "toggle") => Some(ArcamOperation::PowerToggle),
        ("mute", "on") => Some(ArcamOperation::MuteOn),
        ("mute", "off") => Some(ArcamOperation::MuteOff),
        ("mute", "toggle") => Some(ArcamOperation::MuteToggle),
        _ => None,
    }
}

/// Extract the device name from an entity_id (`arcam.{name}.{kind}`).
pub fn device_name_from_entity_id(entity_id: &str) -> Option<&str> {
    let rest = entity_id.strip_prefix("arcam.")?;
    let dot = rest.rfind('.')?;
    Some(&rest[..dot])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_entities() {
        let entities = build_entities("office");
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].entity_id, "arcam.office.power");
        assert_eq!(entities[0].entity_type, "switch");
        assert_eq!(entities[0].features, vec!["on_off", "toggle"]);
        assert_eq!(entities[1].entity_id, "arcam.office.mute");
    }

    #[test]
    fn test_build_initial_states() {
        let states = build_initial_states("office");
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].attributes["state"], "OFF");
        assert_eq!(states[1].attributes["state"], "OFF");
    }

    #[test]
    fn test_resolve_command() {
        assert_eq!(resolve_command("arcam.office.power", "on"), Some(ArcamOperation::PowerOn));
        assert_eq!(resolve_command("arcam.office.power", "off"), Some(ArcamOperation::PowerOff));
        assert_eq!(resolve_command("arcam.office.power", "toggle"), Some(ArcamOperation::PowerToggle));
        assert_eq!(resolve_command("arcam.office.mute", "on"), Some(ArcamOperation::MuteOn));
        assert_eq!(resolve_command("arcam.office.mute", "toggle"), Some(ArcamOperation::MuteToggle));
        assert_eq!(resolve_command("arcam.office.power", "invalid"), None);
        assert_eq!(resolve_command("arcam.office.unknown", "on"), None);
    }

    #[test]
    fn test_device_name_from_entity_id() {
        assert_eq!(device_name_from_entity_id("arcam.office.power"), Some("office"));
        assert_eq!(device_name_from_entity_id("arcam.living-room.mute"), Some("living-room"));
        assert_eq!(device_name_from_entity_id("invalid"), None);
    }

    #[test]
    fn test_device_state_serialization() {
        assert_eq!(serde_json::to_string(&DeviceState::Connected).unwrap(), "\"CONNECTED\"");
        assert_eq!(serde_json::to_string(&DeviceState::Disconnected).unwrap(), "\"DISCONNECTED\"");
    }
}
