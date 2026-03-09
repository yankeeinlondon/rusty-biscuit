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
            email: Some("ken@ken.net".to_string()),
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
    pub screen_brightness_max: Option<i32>,
    pub knob_brightness_max: Option<i32>,
}

/// Labeled option advertised by the device for dynamic UC entities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedOption {
    pub label: String,
    pub value: String,
}

/// Read-only device identity details surfaced to the Remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeviceIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_mac: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub able_remote_boot: Option<bool>,
}

/// Dynamic entity catalog metadata fetched from the device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EntityCatalog {
    #[serde(default)]
    pub source_list: Vec<String>,
    #[serde(default)]
    pub output_list: Vec<String>,
    #[serde(default)]
    pub power_options: Vec<NamedOption>,
    #[serde(default)]
    pub vu_modes: Vec<String>,
    #[serde(default)]
    pub spectrum_modes: Vec<String>,
    #[serde(default = "default_volume_steps")]
    pub volume_steps: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_brightness_max: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knob_brightness_max: Option<i32>,
    #[serde(default)]
    pub identity: DeviceIdentity,
}

fn default_volume_steps() -> u32 {
    DEFAULT_VOLUME_STEPS
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
    Seek(i64),
    VolumeSet(i32),
    VolumeUp,
    VolumeDown,
    MuteOn,
    MuteOff,
    MuteToggle,
    SelectSource(String),
    SelectInput(SelectionCommand),
    SelectOutput(SelectionCommand),
    SelectVuMode(SelectionCommand),
    SelectSpectrumMode(SelectionCommand),
    PowerAction(String),
    ScreenBrightness(BrightnessCommand),
    KnobBrightness(BrightnessCommand),
}

/// Generic selection commands used by UC `select` entities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionCommand {
    Option(String),
    First,
    Last,
    Next,
    Previous,
}

/// UC `light` brightness commands mapped onto Eversolo display brightness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrightnessCommand {
    On(Option<u8>),
    Off,
    Toggle,
}

/// Build the available entities for a given Eversolo device name.
pub fn build_entities(device_name: &str, catalog: &EntityCatalog) -> Vec<EversoloEntity> {
    let mut entities = vec![
        EversoloEntity {
            entity_id: format!("eversolo.{device_name}.power"),
            entity_type: "switch".to_string(),
            name: HashMap::from([("en".to_string(), format!("{device_name} Power"))]),
            features: vec!["on_off".to_string(), "toggle".to_string()],
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
                "seek".to_string(),
                "select_source".to_string(),
                "media_duration".to_string(),
                "media_position".to_string(),
                "media_title".to_string(),
                "media_artist".to_string(),
                "media_album".to_string(),
            ],
            options: Some(HashMap::from([
                (
                    "source_list".to_string(),
                    serde_json::json!(catalog.source_list),
                ),
                (
                    "volume_steps".to_string(),
                    serde_json::json!(catalog.volume_steps.max(2)),
                ),
            ])),
        },
        button_entity(device_name, "power_on_button", "Power On"),
        button_entity(device_name, "power_off_button", "Power Off"),
        select_entity(device_name, "input_select", "Input Routing"),
        select_entity(device_name, "output_select", "Output Routing"),
        light_entity(device_name, "screen_brightness", "Screen Brightness"),
        light_entity(device_name, "knob_brightness", "Knob Brightness"),
        select_entity(device_name, "vu_mode_select", "VU Mode"),
        select_entity(device_name, "spectrum_mode_select", "Spectrum Mode"),
    ];

    entities.extend(catalog.power_options.iter().map(|option| {
        button_entity(
            device_name,
            &format!("power_action_{}", option.value),
            option.label.as_str(),
        )
    }));

    if catalog.identity.model.is_some() {
        entities.push(sensor_entity(device_name, "model_sensor", "Model"));
    }
    if catalog.identity.firmware.is_some() {
        entities.push(sensor_entity(device_name, "firmware_sensor", "Firmware"));
    }
    if catalog.identity.ip.is_some() {
        entities.push(sensor_entity(device_name, "network_address_sensor", "Network Address"));
    }
    if catalog.identity.net_mac.is_some() {
        entities.push(sensor_entity(device_name, "mac_sensor", "MAC Address"));
    }
    if catalog.identity.able_remote_boot.is_some() {
        entities.push(sensor_entity(device_name, "remote_boot_sensor", "Remote Boot"));
    }

    entities
}

/// Build initial entity states (unknown until polled).
pub fn build_initial_states(device_name: &str, catalog: &EntityCatalog) -> Vec<EntityState> {
    let mut states = vec![
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
        select_state(
            device_name,
            "input_select",
            catalog.source_list.clone(),
            String::new(),
        ),
        select_state(
            device_name,
            "output_select",
            catalog.output_list.clone(),
            String::new(),
        ),
        light_state(device_name, "screen_brightness", "UNKNOWN", 0),
        light_state(device_name, "knob_brightness", "UNKNOWN", 0),
        select_state(
            device_name,
            "vu_mode_select",
            catalog.vu_modes.clone(),
            String::new(),
        ),
        select_state(
            device_name,
            "spectrum_mode_select",
            catalog.spectrum_modes.clone(),
            String::new(),
        ),
    ];

    if let Some(model) = &catalog.identity.model {
        states.push(sensor_state(device_name, "model_sensor", model));
    }
    if let Some(firmware) = &catalog.identity.firmware {
        states.push(sensor_state(device_name, "firmware_sensor", firmware));
    }
    if let Some(ip) = &catalog.identity.ip {
        states.push(sensor_state(device_name, "network_address_sensor", ip));
    }
    if let Some(mac) = &catalog.identity.net_mac {
        states.push(sensor_state(device_name, "mac_sensor", mac));
    }
    if let Some(able_remote_boot) = catalog.identity.able_remote_boot {
        states.push(sensor_state(
            device_name,
            "remote_boot_sensor",
            if able_remote_boot { "true" } else { "false" },
        ));
    }

    states
}

/// Map an entity_id + cmd_id to an Eversolo operation.
pub fn resolve_command(entity_id: &str, cmd_id: &str, params: &Value) -> Option<EversoloOperation> {
    let kind = entity_key(entity_id)?;

    match (kind, cmd_id) {
        ("power", "on") => Some(EversoloOperation::PowerOn),
        ("power", "off") => Some(EversoloOperation::PowerOff),
        ("power", "toggle") => Some(EversoloOperation::PowerToggle),
        ("player", "play_pause") => Some(EversoloOperation::PlayPause),
        ("player", "next") => Some(EversoloOperation::Next),
        ("player", "previous") => Some(EversoloOperation::Previous),
        ("player", "seek") => Some(EversoloOperation::Seek(
            params.get("media_position")?.as_i64()?,
        )),
        ("player", "volume") | ("player", "volume_set") => {
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
        ("input_select", _) => {
            Some(EversoloOperation::SelectInput(parse_selection_command(cmd_id, params)?))
        }
        ("output_select", _) => Some(EversoloOperation::SelectOutput(parse_selection_command(
            cmd_id, params,
        )?)),
        ("vu_mode_select", _) => Some(EversoloOperation::SelectVuMode(parse_selection_command(
            cmd_id, params,
        )?)),
        ("spectrum_mode_select", _) => Some(EversoloOperation::SelectSpectrumMode(
            parse_selection_command(cmd_id, params)?,
        )),
        ("screen_brightness", _) => Some(EversoloOperation::ScreenBrightness(
            parse_brightness_command(cmd_id, params)?,
        )),
        ("knob_brightness", _) => Some(EversoloOperation::KnobBrightness(
            parse_brightness_command(cmd_id, params)?,
        )),
        ("power_on_button", "push") => Some(EversoloOperation::PowerOn),
        ("power_off_button", "push") => Some(EversoloOperation::PowerOff),
        _ => None,
    }
    .or_else(|| {
        if cmd_id == "push" {
            kind.strip_prefix("power_action_")
                .map(|tag| EversoloOperation::PowerAction(tag.to_string()))
        } else {
            None
        }
    })
}

/// Extract the device name from an entity_id (`eversolo.{name}.{kind}`).
pub fn device_name_from_entity_id(entity_id: &str) -> Option<&str> {
    let rest = entity_id.strip_prefix("eversolo.")?;
    let dot = rest.rfind('.')?;
    Some(&rest[..dot])
}

/// Return whether an entity id belongs to this integration.
pub fn entity_exists(entity_id: &str) -> bool {
    let Some(kind) = entity_key(entity_id) else {
        return false;
    };

    matches!(
        kind,
        "power"
            | "player"
            | "power_on_button"
            | "power_off_button"
            | "input_select"
            | "output_select"
            | "screen_brightness"
            | "knob_brightness"
            | "vu_mode_select"
            | "spectrum_mode_select"
            | "model_sensor"
            | "firmware_sensor"
            | "network_address_sensor"
            | "mac_sensor"
            | "remote_boot_sensor"
    ) || kind.starts_with("power_action_")
}

fn entity_key(entity_id: &str) -> Option<&str> {
    let device_name = device_name_from_entity_id(entity_id)?;
    let prefix = format!("eversolo.{device_name}.");
    entity_id.strip_prefix(&prefix)
}

fn button_entity(device_name: &str, kind: &str, label: &str) -> EversoloEntity {
    EversoloEntity {
        entity_id: format!("eversolo.{device_name}.{kind}"),
        entity_type: "button".to_string(),
        name: HashMap::from([("en".to_string(), label.to_string())]),
        features: vec!["press".to_string()],
        options: None,
    }
}

fn select_entity(device_name: &str, kind: &str, label: &str) -> EversoloEntity {
    EversoloEntity {
        entity_id: format!("eversolo.{device_name}.{kind}"),
        entity_type: "select".to_string(),
        name: HashMap::from([("en".to_string(), label.to_string())]),
        features: vec![],
        options: None,
    }
}

fn light_entity(device_name: &str, kind: &str, label: &str) -> EversoloEntity {
    EversoloEntity {
        entity_id: format!("eversolo.{device_name}.{kind}"),
        entity_type: "light".to_string(),
        name: HashMap::from([("en".to_string(), label.to_string())]),
        features: vec!["on_off".to_string(), "toggle".to_string(), "dim".to_string()],
        options: None,
    }
}

fn sensor_entity(device_name: &str, kind: &str, label: &str) -> EversoloEntity {
    EversoloEntity {
        entity_id: format!("eversolo.{device_name}.{kind}"),
        entity_type: "sensor".to_string(),
        name: HashMap::from([("en".to_string(), label.to_string())]),
        features: vec![],
        options: None,
    }
}

fn select_state(device_name: &str, kind: &str, options: Vec<String>, current_option: String) -> EntityState {
    EntityState::new(
        format!("eversolo.{device_name}.{kind}"),
        "select",
        HashMap::from([
            ("current_option".to_string(), serde_json::json!(current_option)),
            ("options".to_string(), serde_json::json!(options)),
        ]),
    )
}

fn light_state(device_name: &str, kind: &str, state: &str, brightness: i32) -> EntityState {
    EntityState::new(
        format!("eversolo.{device_name}.{kind}"),
        "light",
        HashMap::from([
            ("state".to_string(), serde_json::json!(state)),
            ("brightness".to_string(), serde_json::json!(brightness)),
        ]),
    )
}

fn sensor_state(device_name: &str, kind: &str, value: &str) -> EntityState {
    EntityState::new(
        format!("eversolo.{device_name}.{kind}"),
        "sensor",
        HashMap::from([("value".to_string(), serde_json::json!(value))]),
    )
}

fn parse_selection_command(cmd_id: &str, params: &Value) -> Option<SelectionCommand> {
    match cmd_id {
        "select_option" => Some(SelectionCommand::Option(
            params.get("option")?.as_str()?.to_string(),
        )),
        "select_first" => Some(SelectionCommand::First),
        "select_last" => Some(SelectionCommand::Last),
        "select_next" => Some(SelectionCommand::Next),
        "select_previous" => Some(SelectionCommand::Previous),
        _ => None,
    }
}

fn parse_brightness_command(cmd_id: &str, params: &Value) -> Option<BrightnessCommand> {
    match cmd_id {
        "on" => Some(BrightnessCommand::On(
            params
                .get("brightness")
                .and_then(|value| value.as_i64())
                .and_then(|value| u8::try_from(value).ok()),
        )),
        "off" => Some(BrightnessCommand::Off),
        "toggle" => Some(BrightnessCommand::Toggle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_entities() {
        let catalog = EntityCatalog {
            source_list: vec!["Internal Player".to_string(), "USB".to_string()],
            output_list: vec!["XLR".to_string()],
            power_options: vec![NamedOption {
                label: "Reboot".to_string(),
                value: "reboot".to_string(),
            }],
            vu_modes: vec!["Classic".to_string()],
            spectrum_modes: vec!["Spectrum".to_string()],
            volume_steps: 160,
            screen_brightness_max: Some(10),
            knob_brightness_max: Some(10),
            identity: DeviceIdentity {
                model: Some("DMP-A8".to_string()),
                firmware: Some("1.0".to_string()),
                ip: Some("192.168.20.90".to_string()),
                net_mac: Some("00:11:22:33:44:55".to_string()),
                able_remote_boot: Some(true),
            },
        };
        let entities = build_entities("living", &catalog);
        assert!(entities.len() >= 10);
        assert_eq!(entities[0].entity_id, "eversolo.living.power");
        assert_eq!(entities[1].entity_id, "eversolo.living.player");
        assert_eq!(entities[1].entity_type, "media_player");
        assert_eq!(entities[1].options.as_ref().unwrap()["volume_steps"], 160);
        assert!(entities
            .iter()
            .any(|entity| entity.entity_id == "eversolo.living.power_action_reboot"));
        assert!(entities
            .iter()
            .any(|entity| entity.entity_id == "eversolo.living.screen_brightness"));
    }

    #[test]
    fn test_build_initial_states() {
        let catalog = EntityCatalog {
            source_list: vec!["Internal Player".to_string()],
            output_list: vec!["XLR".to_string()],
            vu_modes: vec!["Classic".to_string()],
            spectrum_modes: vec!["Spectrum".to_string()],
            identity: DeviceIdentity {
                model: Some("DMP-A8".to_string()),
                ..DeviceIdentity::default()
            },
            ..EntityCatalog::default()
        };
        let states = build_initial_states("living", &catalog);
        assert!(states.len() >= 7);
        assert_eq!(states[0].attributes["state"], "UNKNOWN");
        assert_eq!(states[1].attributes["source"], "");
        assert!(states
            .iter()
            .any(|state| state.entity_id == "eversolo.living.input_select"));
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
        assert_eq!(
            resolve_command("eversolo.living.power_on_button", "push", &empty),
            Some(EversoloOperation::PowerOn)
        );
        assert_eq!(
            resolve_command("eversolo.living.power_action_reboot", "push", &empty),
            Some(EversoloOperation::PowerAction("reboot".to_string()))
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
        assert_eq!(
            resolve_command(
                "eversolo.living.player",
                "seek",
                &json!({"media_position": 90})
            ),
            Some(EversoloOperation::Seek(90))
        );
    }

    #[test]
    fn test_resolve_command_with_params() {
        assert_eq!(
            resolve_command("eversolo.living.player", "volume", &json!({"volume": 42})),
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
        assert_eq!(
            resolve_command(
                "eversolo.living.input_select",
                "select_option",
                &json!({"option": "USB DAC"})
            ),
            Some(EversoloOperation::SelectInput(SelectionCommand::Option(
                "USB DAC".to_string()
            )))
        );
        assert_eq!(
            resolve_command(
                "eversolo.living.screen_brightness",
                "on",
                &json!({"brightness": 128})
            ),
            Some(EversoloOperation::ScreenBrightness(BrightnessCommand::On(
                Some(128)
            )))
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
        assert!(entity_exists("eversolo.living.output_select"));
        assert!(entity_exists("eversolo.living.power_action_reboot"));
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
