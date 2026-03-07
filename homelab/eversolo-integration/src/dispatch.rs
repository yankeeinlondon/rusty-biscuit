//! Eversolo HTTP and Wake-on-LAN command execution.

use std::collections::HashMap;
use std::time::Duration;

use homelab::eversolo::{
    EffectivePowerState, Eversolo, GetStateResponse, InputOutputListResponse,
    infer_effective_power_state, is_actively_playing,
};
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

#[derive(Debug, Clone, PartialEq)]
struct PlaybackContext {
    state: GetStateResponse,
    effective_power_state: EffectivePowerState,
}

/// Fetch a full device snapshot for UC entity state.
pub async fn fetch_snapshot(
    config: &DeviceConfig,
    timeout: Duration,
) -> Result<DeviceSnapshot, EversoloIntegrationError> {
    let client = build_client(config)?;
    let playback = fetch_playback_context(&client, timeout).await?;
    let routing = run_with_timeout(timeout, client.get_inputs_outputs()).await?;
    ensure_status(routing.status)?;

    let catalog = EntityCatalog {
        source_list: available_source_names(&routing),
        volume_steps: volume_steps_from_state(&playback.state),
    };

    Ok(DeviceSnapshot {
        power_attributes: effective_power_attrs(playback.effective_power_state),
        player_attributes: player_attrs(&playback.state, &routing, playback.effective_power_state),
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
        EversoloOperation::PowerOn => power_on(config, timeout).await,
        EversoloOperation::PowerOff => power_off(config, timeout).await,
        EversoloOperation::PowerToggle => {
            let client = build_client(config)?;
            match fetch_playback_context(&client, timeout).await {
                Ok(playback) => match playback.effective_power_state {
                    EffectivePowerState::Active => power_off(config, timeout).await,
                    EffectivePowerState::Standby => power_on(config, timeout).await,
                },
                Err(EversoloIntegrationError::Eversolo(_))
                | Err(EversoloIntegrationError::Timeout) => power_on(config, timeout).await,
                Err(error) => Err(error),
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

async fn power_on(
    config: &DeviceConfig,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let client = build_client(config)?;

    match fetch_playback_context(&client, timeout).await {
        Ok(playback) => match playback.effective_power_state {
            EffectivePowerState::Active => snapshot_updates(config, timeout).await,
            EffectivePowerState::Standby => {
                wake_from_standby(&client, timeout).await?;
                snapshot_updates(config, timeout).await
            }
        },
        Err(EversoloIntegrationError::Eversolo(_)) | Err(EversoloIntegrationError::Timeout) => {
            power_on_via_wol(config)
        }
        Err(error) => Err(error),
    }
}

fn power_on_via_wol(config: &DeviceConfig) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
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
    let playback = fetch_playback_context(&client, timeout).await?;

    if is_actively_playing(playback.state.state) {
        let resp = run_with_timeout(timeout, client.play_or_pause()).await?;
        ensure_status(resp.status)?;
    }

    if playback.effective_power_state != EffectivePowerState::Standby {
        let resp = run_with_timeout(timeout, client.set_power_option("screen")).await?;
        ensure_status(resp.status)?;
    }

    snapshot_updates(config, timeout).await
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

async fn fetch_playback_context(
    client: &Eversolo,
    timeout: Duration,
) -> Result<PlaybackContext, EversoloIntegrationError> {
    let (state, screen_brightness) = run_with_timeout(timeout, async {
        let (state, screen_brightness) =
            tokio::join!(client.get_state(), client.get_screen_brightness());
        Ok::<_, homelab::eversolo::EversoloError>((state?, screen_brightness.ok()))
    })
    .await?;

    ensure_optional_status(state.status)?;
    let screen_brightness = screen_brightness
        .filter(|response| response.status.is_none_or(|status| status == 200))
        .map(|response| response.current_value);

    Ok(PlaybackContext {
        effective_power_state: infer_effective_power_state(state.state, screen_brightness),
        state,
    })
}

async fn wake_from_standby(
    client: &Eversolo,
    timeout: Duration,
) -> Result<(), EversoloIntegrationError> {
    let resp = run_with_timeout(timeout, client.set_power_option("screen")).await?;
    ensure_status(resp.status)?;
    Ok(())
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

fn effective_power_attrs(effective_power_state: EffectivePowerState) -> HashMap<String, Value> {
    power_attrs(effective_power_state == EffectivePowerState::Active)
}

fn power_attrs(on: bool) -> HashMap<String, Value> {
    HashMap::from([("state".to_string(), json!(if on { "ON" } else { "OFF" }))])
}

fn player_power_attrs(on: bool) -> HashMap<String, Value> {
    let mut attrs = offline_player_attributes();
    attrs.insert("state".to_string(), json!(if on { "ON" } else { "OFF" }));
    attrs
}

fn player_attrs(
    state: &GetStateResponse,
    routing: &InputOutputListResponse,
    effective_power_state: EffectivePowerState,
) -> HashMap<String, Value> {
    let mut attrs = HashMap::from([(
        "state".to_string(),
        json!(playback_state_label(state.state, effective_power_state)),
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

fn playback_state_label(state: i32, effective_power_state: EffectivePowerState) -> &'static str {
    match effective_power_state {
        EffectivePowerState::Standby => "STANDBY",
        EffectivePowerState::Active => match state {
            1 => "PLAYING",
            2 => "PAUSED",
            _ => "ON",
        },
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

pub fn offline_power_attributes() -> HashMap<String, Value> {
    power_attrs(false)
}

pub fn offline_player_attributes() -> HashMap<String, Value> {
    HashMap::from([
        ("state".to_string(), json!("OFF")),
        ("volume".to_string(), json!(0)),
        ("muted".to_string(), json!(false)),
        ("source".to_string(), json!("")),
        ("media_position".to_string(), Value::Null),
        ("media_duration".to_string(), Value::Null),
        ("media_title".to_string(), Value::Null),
        ("media_artist".to_string(), Value::Null),
        ("media_album".to_string(), Value::Null),
    ])
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
    use crate::types::EversoloOperation;
    use homelab::eversolo::GetStateResponse;
    use schematic_schema::eversolo::{InputItem, OutputItem, PlayingMusic, VolumeData};
    use std::env;
    use std::time::Duration;

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
        assert_eq!(
            playback_state_label(1, EffectivePowerState::Active),
            "PLAYING"
        );
        assert_eq!(
            playback_state_label(2, EffectivePowerState::Active),
            "PAUSED"
        );
        assert_eq!(playback_state_label(0, EffectivePowerState::Active), "ON");
        assert_eq!(
            playback_state_label(0, EffectivePowerState::Standby),
            "STANDBY"
        );
    }

    #[test]
    fn test_effective_power_attrs_treats_standby_as_switch_off() {
        assert_eq!(
            effective_power_attrs(EffectivePowerState::Active)["state"],
            "ON"
        );
        assert_eq!(
            effective_power_attrs(EffectivePowerState::Standby)["state"],
            "OFF"
        );
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

    fn eversolo_real_target() -> Option<DeviceConfig> {
        let host = env::var("EVERSOLO_REAL_HOST").ok()?;
        let port = env::var("EVERSOLO_REAL_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(9529);
        Some(DeviceConfig {
            host,
            port,
            mac_address: None,
            wol_broadcast: "255.255.255.255".to_string(),
            wol_port: 9517,
        })
    }

    fn require_eversolo_destructive() {
        let enabled = env::var("EVERSOLO_REAL_ALLOW_DESTRUCTIVE").ok();
        assert_eq!(
            enabled.as_deref(),
            Some("1"),
            "set EVERSOLO_REAL_ALLOW_DESTRUCTIVE=1 to run this test"
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

    #[tokio::test]
    #[ignore = "requires a reachable Eversolo device configured via EVERSOLO_REAL_HOST"]
    async fn real_device_fetch_snapshot_returns_catalog_and_player_state() {
        let config = eversolo_real_target().expect("set EVERSOLO_REAL_HOST to run this test");
        let snapshot = fetch_snapshot(&config, Duration::from_secs(10))
            .await
            .expect("real Eversolo snapshot fetch should succeed");

        assert!(snapshot.power_attributes.contains_key("state"));
        assert!(snapshot.player_attributes.contains_key("state"));
        assert!(snapshot.player_attributes.contains_key("source"));
        assert!(snapshot.catalog.volume_steps >= 2);
    }

    #[tokio::test]
    #[ignore = "requires a reachable Eversolo device and writes volume before restoring it"]
    async fn destructive_real_device_volume_command_roundtrips() {
        require_eversolo_destructive();
        let config = eversolo_real_target().expect("set EVERSOLO_REAL_HOST to run this test");
        let timeout = Duration::from_secs(10);
        let before = fetch_snapshot(&config, timeout)
            .await
            .expect("initial Eversolo snapshot should succeed");
        let current = before
            .player_attributes
            .get("volume")
            .and_then(Value::as_i64)
            .expect("volume should be present") as i32;
        if current == 0 {
            eprintln!(
                "Skipping Eversolo destructive volume test because current volume is already 0"
            );
            return;
        }
        let target = current - 1;

        execute_operation(&config, EversoloOperation::VolumeSet(target), timeout)
            .await
            .expect("Eversolo volume change should succeed");
        let changed = fetch_snapshot(&config, timeout)
            .await
            .expect("post-volume snapshot should succeed");
        let observed = changed
            .player_attributes
            .get("volume")
            .and_then(Value::as_i64)
            .expect("volume should be present") as i32;
        assert_eq!(observed, target);

        execute_operation(&config, EversoloOperation::VolumeSet(current), timeout)
            .await
            .expect("Eversolo volume restoration should succeed");
    }

    #[tokio::test]
    #[ignore = "requires a reachable Eversolo device and switches source before restoring it"]
    async fn destructive_real_device_source_switch_roundtrips() {
        require_eversolo_destructive();
        let config = eversolo_real_target().expect("set EVERSOLO_REAL_HOST to run this test");
        let timeout = Duration::from_secs(10);
        let before = fetch_snapshot(&config, timeout)
            .await
            .expect("initial Eversolo snapshot should succeed");
        if before
            .player_attributes
            .get("state")
            .and_then(Value::as_str)
            == Some("OFF")
        {
            eprintln!("Skipping Eversolo destructive source test because the player is OFF");
            return;
        }

        let Some(current_source) = before
            .player_attributes
            .get("source")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            eprintln!(
                "Skipping Eversolo destructive source test because the current source is unknown"
            );
            return;
        };

        let Some(target_source) = before
            .catalog
            .source_list
            .iter()
            .find(|source| *source != &current_source)
            .cloned()
        else {
            eprintln!(
                "Skipping Eversolo destructive source test because no alternate source is available"
            );
            return;
        };

        execute_operation(
            &config,
            EversoloOperation::SelectSource(target_source.clone()),
            timeout,
        )
        .await
        .expect("Eversolo source change should succeed");
        let changed = fetch_snapshot(&config, timeout)
            .await
            .expect("post-source snapshot should succeed");
        let observed = changed
            .player_attributes
            .get("source")
            .and_then(Value::as_str)
            .expect("source should be present");
        assert_eq!(observed, target_source);

        execute_operation(
            &config,
            EversoloOperation::SelectSource(current_source),
            timeout,
        )
        .await
        .expect("Eversolo source restoration should succeed");
    }

    #[test]
    fn test_player_attrs() {
        let attrs = player_attrs(
            &sample_state(),
            &sample_routing(),
            EffectivePowerState::Active,
        );
        assert_eq!(attrs["state"], "PLAYING");
        assert_eq!(attrs["volume"], 55);
        assert_eq!(attrs["source"], "USB DAC");
        assert_eq!(attrs["media_title"], "Song");
    }

    #[test]
    fn test_player_attrs_reports_standby_when_screen_is_off_and_playback_is_idle() {
        let mut state = sample_state();
        state.state = 0;

        let attrs = player_attrs(&state, &sample_routing(), EffectivePowerState::Standby);
        assert_eq!(attrs["state"], "STANDBY");
    }

    #[test]
    fn test_power_attrs() {
        assert_eq!(power_attrs(true)["state"], "ON");
        assert_eq!(power_attrs(false)["state"], "OFF");
    }
}
