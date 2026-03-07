//! Eversolo HTTP and Wake-on-LAN command execution.

use std::collections::HashMap;
use std::time::Duration;

use homelab::eversolo::{Eversolo, GetStateResponse, InputOutputListResponse};
use homelab::network::parse_host;
use homelab::wol::send_magic_packet;
use schematic_schema::eversolo::VolumeData;
use serde_json::{Value, json};

use crate::error::EversoloIntegrationError;
use crate::types::{DEFAULT_VOLUME_STEPS, DeviceConfig, EversoloOperation};

/// Dynamic entity catalog metadata fetched from the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityCatalog {
    pub source_list: Vec<String>,
    pub volume_steps: u32,
}

/// Full device snapshot used to populate UC entity state.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceSnapshot {
    pub power_attributes: HashMap<String, Value>,
    pub player_attributes: HashMap<String, Value>,
    pub catalog: EntityCatalog,
}

/// State update for a single entity.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityUpdate {
    pub entity_kind: &'static str,
    pub entity_type: &'static str,
    pub attributes: HashMap<String, Value>,
}

/// Probe whether the device is currently reachable over HTTP.
pub async fn probe_device(config: &DeviceConfig, timeout: Duration) -> bool {
    let client = match build_client(config) {
        Ok(client) => client,
        Err(_) => return false,
    };

    match run_with_timeout(timeout, client.get_model()).await {
        Ok(model) => model.status == 200,
        Err(_) => false,
    }
}

/// Fetch a full device snapshot for UC entity state.
pub async fn fetch_snapshot(
    config: &DeviceConfig,
    timeout: Duration,
) -> Result<DeviceSnapshot, EversoloIntegrationError> {
    let client = build_client(config)?;
    let (state, routing) = run_with_timeout(timeout, async {
        tokio::try_join!(client.get_state(), client.get_inputs_outputs())
    })
    .await?;

    ensure_optional_status(state.status)?;
    ensure_status(routing.status)?;

    let catalog = EntityCatalog {
        source_list: available_source_names(&routing),
        volume_steps: volume_steps_from_state(&state),
    };

    Ok(DeviceSnapshot {
        power_attributes: power_attrs(true),
        player_attributes: player_attrs(&state, &routing),
        catalog,
    })
}

/// Execute an Eversolo operation and return updated entity state attributes.
pub async fn execute_operation(
    config: &DeviceConfig,
    operation: EversoloOperation,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    match operation {
        EversoloOperation::PowerOn => power_on(config),
        EversoloOperation::PowerOff => power_off(config, timeout).await,
        EversoloOperation::PowerToggle => {
            if probe_device(config, timeout).await {
                power_off(config, timeout).await
            } else {
                power_on(config)
            }
        }
        EversoloOperation::PlayPause => {
            let client = build_client(config)?;
            let resp = run_with_timeout(timeout, client.play_or_pause()).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::Next => {
            let client = build_client(config)?;
            let resp = run_with_timeout(timeout, client.play_next()).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::Previous => {
            let client = build_client(config)?;
            let resp = run_with_timeout(timeout, client.play_previous()).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::VolumeSet(level) => {
            let client = build_client(config)?;
            let state = run_with_timeout(timeout, client.get_state()).await?;
            ensure_optional_status(state.status)?;
            let level = clamp_volume(level, state.volume_data.as_ref());
            let resp = run_with_timeout(timeout, client.set_volume(i64::from(level))).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::VolumeUp => {
            let client = build_client(config)?;
            let state = run_with_timeout(timeout, client.get_state()).await?;
            ensure_optional_status(state.status)?;
            let volume = state
                .volume_data
                .as_ref()
                .ok_or_else(|| {
                    EversoloIntegrationError::InvalidParameter(
                        "device did not return volume data".to_string(),
                    )
                })?
                .current_volume
                + 1;
            let next = clamp_volume(volume, state.volume_data.as_ref());
            let resp = run_with_timeout(timeout, client.set_volume(i64::from(next))).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::VolumeDown => {
            let client = build_client(config)?;
            let state = run_with_timeout(timeout, client.get_state()).await?;
            ensure_optional_status(state.status)?;
            let volume = state
                .volume_data
                .as_ref()
                .ok_or_else(|| {
                    EversoloIntegrationError::InvalidParameter(
                        "device did not return volume data".to_string(),
                    )
                })?
                .current_volume
                - 1;
            let next = clamp_volume(volume, state.volume_data.as_ref());
            let resp = run_with_timeout(timeout, client.set_volume(i64::from(next))).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::MuteOn => {
            let client = build_client(config)?;
            let resp = run_with_timeout(timeout, client.set_mute(true)).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::MuteOff => {
            let client = build_client(config)?;
            let resp = run_with_timeout(timeout, client.set_mute(false)).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::MuteToggle => {
            let client = build_client(config)?;
            let state = run_with_timeout(timeout, client.get_state()).await?;
            ensure_optional_status(state.status)?;
            let muted = state
                .volume_data
                .as_ref()
                .map(|v| v.is_mute)
                .unwrap_or(false);
            let resp = run_with_timeout(timeout, client.set_mute(!muted)).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::SelectSource(source) => {
            let client = build_client(config)?;
            let routing = run_with_timeout(timeout, client.get_inputs_outputs()).await?;
            ensure_status(routing.status)?;
            let tag = resolve_input_tag(&routing, &source).ok_or_else(|| {
                EversoloIntegrationError::InvalidParameter(format!(
                    "unknown input source \"{source}\""
                ))
            })?;
            let resp = run_with_timeout(timeout, client.set_input(&tag)).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
    }
}

fn power_on(config: &DeviceConfig) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let mac = config
        .mac_address
        .as_deref()
        .ok_or(EversoloIntegrationError::MissingMacAddress)?;
    send_magic_packet(mac, &config.wol_broadcast, config.wol_port)?;

    Ok(vec![
        EntityUpdate {
            entity_kind: "power",
            entity_type: "switch",
            attributes: power_attrs(true),
        },
        EntityUpdate {
            entity_kind: "player",
            entity_type: "media_player",
            attributes: player_power_attrs(true),
        },
    ])
}

async fn power_off(
    config: &DeviceConfig,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let client = build_client(config)?;
    let resp = run_with_timeout(timeout, client.set_power_option("poweroff")).await?;
    ensure_status(resp.status)?;

    Ok(vec![
        EntityUpdate {
            entity_kind: "power",
            entity_type: "switch",
            attributes: power_attrs(false),
        },
        EntityUpdate {
            entity_kind: "player",
            entity_type: "media_player",
            attributes: player_power_attrs(false),
        },
    ])
}

async fn snapshot_updates(
    config: &DeviceConfig,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let snapshot = fetch_snapshot(config, timeout).await?;
    Ok(vec![
        EntityUpdate {
            entity_kind: "power",
            entity_type: "switch",
            attributes: snapshot.power_attributes,
        },
        EntityUpdate {
            entity_kind: "player",
            entity_type: "media_player",
            attributes: snapshot.player_attributes,
        },
    ])
}

fn build_client(config: &DeviceConfig) -> Result<Eversolo, EversoloIntegrationError> {
    let host = parse_host(&config.host).map_err(|e| EversoloIntegrationError::InvalidHost(e.0))?;
    Ok(Eversolo::new(host.to_string(), config.port))
}

async fn run_with_timeout<T>(
    timeout: Duration,
    future: impl std::future::Future<Output = Result<T, homelab::eversolo::EversoloError>>,
) -> Result<T, EversoloIntegrationError> {
    match tokio::time::timeout(timeout, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(EversoloIntegrationError::Eversolo(error)),
        Err(_) => Err(EversoloIntegrationError::Timeout),
    }
}

fn ensure_status(status: i32) -> Result<(), EversoloIntegrationError> {
    if status == 200 {
        Ok(())
    } else {
        Err(EversoloIntegrationError::UnexpectedStatus(status))
    }
}

fn ensure_optional_status(status: Option<i32>) -> Result<(), EversoloIntegrationError> {
    if let Some(status) = status {
        ensure_status(status)?;
    }
    Ok(())
}

fn power_attrs(on: bool) -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!(if on { "ON" } else { "OFF" }))])
}

fn player_power_attrs(on: bool) -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!(if on { "ON" } else { "OFF" }))])
}

fn player_attrs(
    state: &GetStateResponse,
    routing: &InputOutputListResponse,
) -> HashMap<String, Value> {
    let mut attrs = HashMap::from([(
        "state".to_string(),
        json!(playback_state_label(state.state)),
    )]);

    if let Some(volume_data) = &state.volume_data {
        attrs.insert("volume".to_string(), json!(volume_data.current_volume));
        attrs.insert("muted".to_string(), json!(volume_data.is_mute));
    }

    if let Some(source) = current_input_name(routing) {
        attrs.insert("source".to_string(), json!(source));
    }

    if let Some(position) = state.position {
        attrs.insert("media_position".to_string(), json!(position));
    }

    if let Some(duration) = state.duration {
        attrs.insert("media_duration".to_string(), json!(duration));
    }

    if let Some(track) = &state.playing_music {
        if let Some(title) = &track.title {
            attrs.insert("media_title".to_string(), json!(title));
        }
        if let Some(artist) = &track.artist {
            attrs.insert("media_artist".to_string(), json!(artist));
        }
        if let Some(album) = &track.album {
            attrs.insert("media_album".to_string(), json!(album));
        }
    }

    attrs
}

fn playback_state_label(state: i32) -> &'static str {
    match state {
        1 => "PLAYING",
        2 => "PAUSED",
        _ => "ON",
    }
}

fn current_input_name(routing: &InputOutputListResponse) -> Option<String> {
    let index = usize::try_from(routing.input_index?).ok()?;
    routing.input_data.get(index).map(|item| item.name.clone())
}

fn available_source_names(routing: &InputOutputListResponse) -> Vec<String> {
    routing
        .input_data
        .iter()
        .map(|item| item.name.clone())
        .collect()
}

fn volume_steps_from_state(state: &GetStateResponse) -> u32 {
    state
        .volume_data
        .as_ref()
        .and_then(|value| u32::try_from(value.max_volume).ok())
        .filter(|value| *value >= 2)
        .unwrap_or(DEFAULT_VOLUME_STEPS)
}

fn clamp_volume(level: i32, volume_data: Option<&VolumeData>) -> i32 {
    let min = volume_data.and_then(|value| value.min_volume).unwrap_or(0);
    let max = volume_data
        .map(|value| value.max_volume)
        .unwrap_or(DEFAULT_VOLUME_STEPS as i32);
    level.clamp(min, max)
}

fn resolve_input_tag(routing: &InputOutputListResponse, source: &str) -> Option<String> {
    routing
        .input_data
        .iter()
        .find(|item| {
            item.name.eq_ignore_ascii_case(source) || item.tag.eq_ignore_ascii_case(source)
        })
        .map(|item| item.tag.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use homelab::eversolo::GetStateResponse;
    use schematic_schema::eversolo::{InputItem, OutputItem, PlayingMusic, VolumeData};

    fn sample_routing() -> InputOutputListResponse {
        InputOutputListResponse {
            status: 200,
            input_data: vec![
                InputItem {
                    name: "Internal Player".to_string(),
                    tag: "local".to_string(),
                    icon: None,
                    sorted_index: Some(0),
                },
                InputItem {
                    name: "USB DAC".to_string(),
                    tag: "XMOS".to_string(),
                    icon: None,
                    sorted_index: Some(1),
                },
            ],
            output_data: vec![OutputItem {
                name: "XLR".to_string(),
                tag: "XLR".to_string(),
                enable: true,
                icon: None,
                sorted_index: Some(0),
            }],
            output_info: None,
            input_index: Some(1),
            output_index: Some(0),
        }
    }

    fn sample_state() -> GetStateResponse {
        GetStateResponse {
            status: Some(200),
            state: 1,
            position: Some(10_000),
            duration: Some(240_000),
            playing_music: Some(PlayingMusic {
                title: Some("Song".to_string()),
                artist: Some("Artist".to_string()),
                album: Some("Album".to_string()),
                track_number: None,
            }),
            volume_data: Some(VolumeData {
                current_volume: 55,
                max_volume: 160,
                min_volume: Some(0),
                is_mute: false,
                volume_db: Some("-24 dB".to_string()),
                is_volume_enable: Some(true),
            }),
        }
    }

    #[test]
    fn test_playback_state_label() {
        assert_eq!(playback_state_label(1), "PLAYING");
        assert_eq!(playback_state_label(2), "PAUSED");
        assert_eq!(playback_state_label(0), "ON");
    }

    #[test]
    fn test_current_input_name() {
        assert_eq!(
            current_input_name(&sample_routing()),
            Some("USB DAC".to_string())
        );
    }

    #[test]
    fn test_available_source_names() {
        let sources = available_source_names(&sample_routing());
        assert_eq!(
            sources,
            vec!["Internal Player".to_string(), "USB DAC".to_string()]
        );
    }

    #[test]
    fn test_volume_steps_from_state() {
        assert_eq!(volume_steps_from_state(&sample_state()), 160);
    }

    #[test]
    fn test_clamp_volume() {
        let state = sample_state();
        assert_eq!(clamp_volume(200, state.volume_data.as_ref()), 160);
        assert_eq!(clamp_volume(-10, state.volume_data.as_ref()), 0);
        assert_eq!(clamp_volume(42, state.volume_data.as_ref()), 42);
    }

    #[test]
    fn test_resolve_input_tag() {
        let routing = sample_routing();
        assert_eq!(
            resolve_input_tag(&routing, "USB DAC"),
            Some("XMOS".to_string())
        );
        assert_eq!(
            resolve_input_tag(&routing, "XMOS"),
            Some("XMOS".to_string())
        );
        assert_eq!(resolve_input_tag(&routing, "unknown"), None);
    }

    #[test]
    fn test_player_attrs() {
        let attrs = player_attrs(&sample_state(), &sample_routing());
        assert_eq!(attrs["state"], "PLAYING");
        assert_eq!(attrs["volume"], 55);
        assert_eq!(attrs["source"], "USB DAC");
        assert_eq!(attrs["media_title"], "Song");
    }

    #[test]
    fn test_power_attrs() {
        assert_eq!(power_attrs(true)["state"], "ON");
        assert_eq!(power_attrs(false)["state"], "OFF");
    }
}
