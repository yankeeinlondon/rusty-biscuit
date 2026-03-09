//! Eversolo HTTP and Wake-on-LAN command execution.

use std::collections::HashMap;
use std::time::Duration;

use homelab::eversolo::{
    DisplayModeListResponse, EffectivePowerState, Eversolo, GetModelResponse,
    GetStateResponse, InputOutputListResponse, PowerOptionsResponse, infer_effective_power_state,
    is_actively_playing,
};
use homelab::network::parse_host;
use homelab::wol::send_magic_packet;
use schematic_schema::eversolo::VolumeData;
use serde_json::{Value, json};

use crate::error::EversoloIntegrationError;
use crate::types::{
    BrightnessCommand, DEFAULT_VOLUME_STEPS, DeviceConfig, DeviceIdentity, EntityCatalog,
    EversoloOperation, NamedOption, SelectionCommand,
};

/// Full device snapshot used to populate UC entity state.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceSnapshot {
    pub updates: Vec<EntityUpdate>,
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
    let (
        model_result,
        state_result,
        routing_result,
        power_options_result,
        screen_brightness_result,
        knob_brightness_result,
        vu_modes_result,
        spectrum_modes_result,
    ) = tokio::join!(
        run_with_timeout(timeout, client.get_model()),
        run_with_timeout(timeout, client.get_state()),
        run_with_timeout(timeout, client.get_inputs_outputs()),
        run_with_timeout(timeout, client.get_power_options()),
        run_with_timeout(timeout, client.get_screen_brightness()),
        run_with_timeout(timeout, client.get_knob_brightness()),
        run_with_timeout(timeout, client.get_vu_modes()),
        run_with_timeout(timeout, client.get_spectrum_modes()),
    );

    let mut first_error = None;

    let model = optional_domain(model_result, |response| ensure_status(response.status), &mut first_error);
    let state =
        optional_domain(state_result, |response| ensure_optional_status(response.status), &mut first_error);
    let routing = optional_domain(routing_result, |response| ensure_status(response.status), &mut first_error);
    let power_options =
        optional_domain(power_options_result, |response| ensure_status(response.status), &mut first_error);
    let screen_brightness = optional_domain(
        screen_brightness_result,
        |response| ensure_optional_status(response.status),
        &mut first_error,
    );
    let knob_brightness = optional_domain(
        knob_brightness_result,
        |response| ensure_optional_status(response.status),
        &mut first_error,
    );
    let vu_modes = optional_domain(vu_modes_result, |response| ensure_status(response.status), &mut first_error);
    let spectrum_modes =
        optional_domain(spectrum_modes_result, |response| ensure_status(response.status), &mut first_error);

    let effective_power_state = state.as_ref().map(|playback_state| {
        infer_effective_power_state(
            playback_state.state,
            screen_brightness.as_ref().map(|response| response.current_value),
        )
    });

    let catalog = EntityCatalog {
        source_list: routing
            .as_ref()
            .map(available_source_names)
            .unwrap_or_default(),
        output_list: routing
            .as_ref()
            .map(available_output_names)
            .unwrap_or_default(),
        power_options: power_options
            .as_ref()
            .map(power_option_list)
            .unwrap_or_default(),
        vu_modes: vu_modes.as_ref().map(mode_titles).unwrap_or_default(),
        spectrum_modes: spectrum_modes
            .as_ref()
            .map(mode_titles)
            .unwrap_or_default(),
        volume_steps: state
            .as_ref()
            .map(volume_steps_from_state)
            .unwrap_or(DEFAULT_VOLUME_STEPS),
        screen_brightness_max: screen_brightness.as_ref().and_then(|response| response.max),
        knob_brightness_max: knob_brightness.as_ref().and_then(|response| response.max),
        identity: model
            .as_ref()
            .map(identity_from_model)
            .unwrap_or_default(),
    };

    let mut updates = Vec::new();

    if let (Some(state), Some(effective_power_state)) = (state.as_ref(), effective_power_state) {
        updates.push(EntityUpdate {
            entity_kind: "power",
            entity_type: "switch",
            attributes: effective_power_attrs(effective_power_state),
        });
        updates.push(EntityUpdate {
            entity_kind: "player",
            entity_type: "media_player",
            attributes: player_attrs(state, routing.as_ref(), effective_power_state),
        });
    }

    if let Some(routing) = routing.as_ref() {
        updates.push(EntityUpdate {
            entity_kind: "input_select",
            entity_type: "select",
            attributes: select_attrs(
                current_input_name(routing).unwrap_or_default(),
                available_source_names(routing),
            ),
        });
        updates.push(EntityUpdate {
            entity_kind: "output_select",
            entity_type: "select",
            attributes: select_attrs(
                current_output_name(routing).unwrap_or_default(),
                available_output_names(routing),
            ),
        });
    }

    if let Some(screen_brightness) = screen_brightness.as_ref() {
        updates.push(EntityUpdate {
            entity_kind: "screen_brightness",
            entity_type: "light",
            attributes: light_attrs(screen_brightness.current_value, screen_brightness.max),
        });
    }

    if let Some(knob_brightness) = knob_brightness.as_ref() {
        updates.push(EntityUpdate {
            entity_kind: "knob_brightness",
            entity_type: "light",
            attributes: light_attrs(knob_brightness.current_value, knob_brightness.max),
        });
    }

    if let Some(vu_modes) = vu_modes.as_ref() {
        updates.push(EntityUpdate {
            entity_kind: "vu_mode_select",
            entity_type: "select",
            attributes: select_attrs(
                current_mode_title(vu_modes).unwrap_or_default(),
                mode_titles(vu_modes),
            ),
        });
    }

    if let Some(spectrum_modes) = spectrum_modes.as_ref() {
        updates.push(EntityUpdate {
            entity_kind: "spectrum_mode_select",
            entity_type: "select",
            attributes: select_attrs(
                current_mode_title(spectrum_modes).unwrap_or_default(),
                mode_titles(spectrum_modes),
            ),
        });
    }

    if let Some(model) = model.as_ref() {
        updates.extend(identity_updates(model));
    }

    if updates.is_empty() {
        return Err(first_error.unwrap_or(EversoloIntegrationError::Timeout));
    }

    Ok(DeviceSnapshot { updates, catalog })
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
        EversoloOperation::Seek(position_seconds) => {
            let client = build_client(config)?;
            let position_ms = position_seconds.saturating_mul(1000);
            let resp = run_with_timeout(timeout, client.seek_to(position_ms)).await?;
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
        EversoloOperation::SelectInput(selection) => {
            select_input(config, selection, timeout).await
        }
        EversoloOperation::SelectOutput(selection) => {
            select_output(config, selection, timeout).await
        }
        EversoloOperation::SelectVuMode(selection) => {
            select_display_mode(config, selection, DisplayModeTarget::Vu, timeout).await
        }
        EversoloOperation::SelectSpectrumMode(selection) => {
            select_display_mode(config, selection, DisplayModeTarget::Spectrum, timeout).await
        }
        EversoloOperation::PowerAction(tag) => {
            let client = build_client(config)?;
            let resp = run_with_timeout(timeout, client.set_power_option(&tag)).await?;
            ensure_status(resp.status)?;
            snapshot_updates(config, timeout).await
        }
        EversoloOperation::ScreenBrightness(command) => {
            set_brightness(config, BrightnessTarget::Screen, command, timeout).await
        }
        EversoloOperation::KnobBrightness(command) => {
            set_brightness(config, BrightnessTarget::Knob, command, timeout).await
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
    Ok(snapshot.updates)
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
    routing: Option<&InputOutputListResponse>,
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

    if let Some(source) = routing.and_then(current_input_name) {
        attrs.insert("source".to_string(), json!(source));
    }

    if let Some(position) = state.position {
        attrs.insert("media_position".to_string(), json!(position / 1000));
    }

    if let Some(duration) = state.duration {
        attrs.insert("media_duration".to_string(), json!(duration / 1000));
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
            0 => "STOPPED",
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

fn current_output_name(routing: &InputOutputListResponse) -> Option<String> {
    let index = usize::try_from(routing.output_index?).ok()?;
    routing.output_data.get(index).map(|item| item.name.clone())
}

fn available_output_names(routing: &InputOutputListResponse) -> Vec<String> {
    let selected_name = current_output_name(routing);
    routing
        .output_data
        .iter()
        .filter(|item| item.enable || selected_name.as_deref() == Some(item.name.as_str()))
        .map(|item| item.name.clone())
        .collect()
}

fn select_attrs(current_option: String, options: Vec<String>) -> HashMap<String, Value> {
    HashMap::from([
        ("current_option".to_string(), json!(current_option)),
        ("options".to_string(), json!(options)),
    ])
}

fn light_attrs(current_value: i32, max: Option<i32>) -> HashMap<String, Value> {
    HashMap::from([
        (
            "state".to_string(),
            json!(if current_value > 0 { "ON" } else { "OFF" }),
        ),
        (
            "brightness".to_string(),
            json!(uc_brightness_from_device(current_value, max)),
        ),
    ])
}

fn power_option_list(response: &PowerOptionsResponse) -> Vec<NamedOption> {
    response
        .data
        .iter()
        .map(|option| NamedOption {
            label: option.name.clone(),
            value: option.tag.clone(),
        })
        .collect()
}

fn mode_titles(response: &DisplayModeListResponse) -> Vec<String> {
    response.data.iter().map(|mode| mode.title.clone()).collect()
}

fn current_mode_title(response: &DisplayModeListResponse) -> Option<String> {
    let index = usize::try_from(response.current_index?).ok()?;
    response.data.get(index).map(|mode| mode.title.clone())
}

fn identity_from_model(model: &GetModelResponse) -> DeviceIdentity {
    DeviceIdentity {
        model: Some(model.model.clone()),
        firmware: Some(model.firmware.clone()),
        ip: Some(model.ip.clone()),
        net_mac: Some(model.net_mac.clone()),
        able_remote_boot: model.able_remote_boot,
    }
}

fn identity_updates(model: &GetModelResponse) -> Vec<EntityUpdate> {
    vec![
        EntityUpdate {
            entity_kind: "model_sensor",
            entity_type: "sensor",
            attributes: sensor_attrs(model.model.clone()),
        },
        EntityUpdate {
            entity_kind: "firmware_sensor",
            entity_type: "sensor",
            attributes: sensor_attrs(model.firmware.clone()),
        },
        EntityUpdate {
            entity_kind: "network_address_sensor",
            entity_type: "sensor",
            attributes: sensor_attrs(model.ip.clone()),
        },
        EntityUpdate {
            entity_kind: "mac_sensor",
            entity_type: "sensor",
            attributes: sensor_attrs(model.net_mac.clone()),
        },
        EntityUpdate {
            entity_kind: "remote_boot_sensor",
            entity_type: "sensor",
            attributes: sensor_attrs(if model.able_remote_boot.unwrap_or(false) {
                "true".to_string()
            } else {
                "false".to_string()
            }),
        },
    ]
}

fn sensor_attrs(value: String) -> HashMap<String, Value> {
    HashMap::from([("value".to_string(), json!(value))])
}

#[allow(dead_code)]
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

fn resolve_output_tag(routing: &InputOutputListResponse, output: &str) -> Option<String> {
    routing
        .output_data
        .iter()
        .filter(|item| item.enable || current_output_name(routing).as_deref() == Some(item.name.as_str()))
        .find(|item| item.name.eq_ignore_ascii_case(output) || item.tag.eq_ignore_ascii_case(output))
        .map(|item| item.tag.clone())
}

fn resolve_mode_index(response: &DisplayModeListResponse, mode: &str) -> Option<i32> {
    response
        .data
        .iter()
        .find(|entry| {
            entry.title.eq_ignore_ascii_case(mode)
                || entry
                    .tag
                    .as_deref()
                    .is_some_and(|tag| tag.eq_ignore_ascii_case(mode))
        })
        .and_then(|entry| entry.index)
}

fn apply_selection_command(
    options: &[String],
    current: Option<String>,
    command: &SelectionCommand,
) -> Option<String> {
    match command {
        SelectionCommand::Option(value) => options
            .iter()
            .find(|option| option.eq_ignore_ascii_case(value))
            .cloned(),
        SelectionCommand::First => options.first().cloned(),
        SelectionCommand::Last => options.last().cloned(),
        SelectionCommand::Next => {
            let current = current?;
            let index = options
                .iter()
                .position(|option| option.eq_ignore_ascii_case(&current))?;
            options.get((index + 1).min(options.len().saturating_sub(1))).cloned()
        }
        SelectionCommand::Previous => {
            let current = current?;
            let index = options
                .iter()
                .position(|option| option.eq_ignore_ascii_case(&current))?;
            options.get(index.saturating_sub(1)).cloned()
        }
    }
}

fn optional_domain<T>(
    result: Result<T, EversoloIntegrationError>,
    validate: impl FnOnce(&T) -> Result<(), EversoloIntegrationError>,
    first_error: &mut Option<EversoloIntegrationError>,
) -> Option<T> {
    match result {
        Ok(value) => match validate(&value) {
            Ok(()) => Some(value),
            Err(error) => {
                if first_error.is_none() {
                    *first_error = Some(error);
                }
                None
            }
        },
        Err(error) => {
            if first_error.is_none() {
                *first_error = Some(error);
            }
            None
        }
    }
}

fn uc_brightness_from_device(current_value: i32, max: Option<i32>) -> i32 {
    let max = max.unwrap_or(255).max(1);
    ((current_value.clamp(0, max) as f64 / f64::from(max)) * 255.0).round() as i32
}

fn device_brightness_from_uc(level: u8, max: i32) -> i32 {
    ((f64::from(level) / 255.0) * f64::from(max.max(1))).round() as i32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrightnessTarget {
    Screen,
    Knob,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisplayModeTarget {
    Vu,
    Spectrum,
}

async fn select_input(
    config: &DeviceConfig,
    selection: SelectionCommand,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let client = build_client(config)?;
    let routing = run_with_timeout(timeout, client.get_inputs_outputs()).await?;
    ensure_status(routing.status)?;

    let selected = apply_selection_command(
        &available_source_names(&routing),
        current_input_name(&routing),
        &selection,
    )
    .ok_or_else(|| {
        EversoloIntegrationError::InvalidParameter("unable to resolve input selection".to_string())
    })?;
    let tag = resolve_input_tag(&routing, &selected).ok_or_else(|| {
        EversoloIntegrationError::InvalidParameter(format!(
            "unknown input source \"{selected}\""
        ))
    })?;
    let resp = run_with_timeout(timeout, client.set_input(&tag)).await?;
    ensure_status(resp.status)?;
    snapshot_updates(config, timeout).await
}

async fn select_output(
    config: &DeviceConfig,
    selection: SelectionCommand,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let client = build_client(config)?;
    let routing = run_with_timeout(timeout, client.get_inputs_outputs()).await?;
    ensure_status(routing.status)?;

    let selected = apply_selection_command(
        &available_output_names(&routing),
        current_output_name(&routing),
        &selection,
    )
    .ok_or_else(|| {
        EversoloIntegrationError::InvalidParameter("unable to resolve output selection".to_string())
    })?;
    let tag = resolve_output_tag(&routing, &selected).ok_or_else(|| {
        EversoloIntegrationError::InvalidParameter(format!(
            "unknown output route \"{selected}\""
        ))
    })?;
    let resp = run_with_timeout(timeout, client.set_output(&tag)).await?;
    ensure_status(resp.status)?;
    snapshot_updates(config, timeout).await
}

async fn select_display_mode(
    config: &DeviceConfig,
    selection: SelectionCommand,
    target: DisplayModeTarget,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let client = build_client(config)?;
    let response = match target {
        DisplayModeTarget::Vu => run_with_timeout(timeout, client.get_vu_modes()).await?,
        DisplayModeTarget::Spectrum => run_with_timeout(timeout, client.get_spectrum_modes()).await?,
    };
    ensure_status(response.status)?;

    let selected = apply_selection_command(
        &mode_titles(&response),
        current_mode_title(&response),
        &selection,
    )
    .ok_or_else(|| {
        EversoloIntegrationError::InvalidParameter("unable to resolve display mode".to_string())
    })?;
    let index = resolve_mode_index(&response, &selected).ok_or_else(|| {
        EversoloIntegrationError::InvalidParameter(format!(
            "unknown display mode \"{selected}\""
        ))
    })?;
    let resp = match target {
        DisplayModeTarget::Vu => run_with_timeout(timeout, client.set_vu_mode(i64::from(index))).await?,
        DisplayModeTarget::Spectrum => {
            run_with_timeout(timeout, client.set_spectrum_mode(i64::from(index))).await?
        }
    };
    ensure_status(resp.status)?;
    snapshot_updates(config, timeout).await
}

async fn set_brightness(
    config: &DeviceConfig,
    target: BrightnessTarget,
    command: BrightnessCommand,
    timeout: Duration,
) -> Result<Vec<EntityUpdate>, EversoloIntegrationError> {
    let client = build_client(config)?;
    let response = match target {
        BrightnessTarget::Screen => run_with_timeout(timeout, client.get_screen_brightness()).await?,
        BrightnessTarget::Knob => run_with_timeout(timeout, client.get_knob_brightness()).await?,
    };
    ensure_optional_status(response.status)?;

    let fallback_max = match target {
        BrightnessTarget::Screen => config.screen_brightness_max,
        BrightnessTarget::Knob => config.knob_brightness_max,
    };
    let max = response.max.or(fallback_max).unwrap_or(255).max(1);
    let target_index = match command {
        BrightnessCommand::On(Some(level)) => device_brightness_from_uc(level, max),
        BrightnessCommand::On(None) => {
            if response.current_value > 0 {
                response.current_value
            } else {
                max
            }
        }
        BrightnessCommand::Off => 0,
        BrightnessCommand::Toggle => {
            if response.current_value > 0 {
                0
            } else {
                max
            }
        }
    };

    let resp = match target {
        BrightnessTarget::Screen => {
            run_with_timeout(timeout, client.set_screen_brightness(i64::from(target_index))).await?
        }
        BrightnessTarget::Knob => {
            run_with_timeout(timeout, client.set_knob_brightness(i64::from(target_index))).await?
        }
    };
    ensure_status(resp.status)?;
    snapshot_updates(config, timeout).await
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

    fn update_attrs<'a>(snapshot: &'a DeviceSnapshot, kind: &str) -> &'a HashMap<String, Value> {
        &snapshot
            .updates
            .iter()
            .find(|update| update.entity_kind == kind)
            .unwrap_or_else(|| panic!("missing update for {kind}"))
            .attributes
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
        assert_eq!(playback_state_label(0, EffectivePowerState::Active), "STOPPED");
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
            screen_brightness_max: None,
            knob_brightness_max: None,
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

        assert!(update_attrs(&snapshot, "power").contains_key("state"));
        assert!(update_attrs(&snapshot, "player").contains_key("state"));
        assert!(update_attrs(&snapshot, "player").contains_key("source"));
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
            .updates
            .iter()
            .find(|update| update.entity_kind == "player")
            .expect("player update should be present")
            .attributes
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
            .updates
            .iter()
            .find(|update| update.entity_kind == "player")
            .expect("player update should be present")
            .attributes
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
            .updates
            .iter()
            .find(|update| update.entity_kind == "player")
            .expect("player update should be present")
            .attributes
            .get("state")
            .and_then(Value::as_str)
            == Some("OFF")
        {
            eprintln!("Skipping Eversolo destructive source test because the player is OFF");
            return;
        }

        let Some(current_source) = before
            .updates
            .iter()
            .find(|update| update.entity_kind == "player")
            .expect("player update should be present")
            .attributes
            .get("source")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
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
            .updates
            .iter()
            .find(|update| update.entity_kind == "player")
            .expect("player update should be present")
            .attributes
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
            Some(&sample_routing()),
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

        let attrs = player_attrs(&state, Some(&sample_routing()), EffectivePowerState::Standby);
        assert_eq!(attrs["state"], "STANDBY");
    }

    #[test]
    fn test_power_attrs() {
        assert_eq!(power_attrs(true)["state"], "ON");
        assert_eq!(power_attrs(false)["state"], "OFF");
    }
}
