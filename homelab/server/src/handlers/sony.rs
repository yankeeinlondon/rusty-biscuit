use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Sony receiver routes with device name parameter
pub fn routes_with_name() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_power_by_name, set_power_by_name))
        .routes(routes!(get_volume_by_name, set_volume_by_name))
        .routes(routes!(get_mute_by_name, set_mute_by_name))
        .routes(routes!(list_inputs_by_name))
        .routes(routes!(list_sources_by_name))
        .routes(routes!(get_current_input_by_name))
        .routes(routes!(set_input_by_name))
        .routes(routes!(set_source_by_name))
        .routes(routes!(get_system_info_by_name))
        .routes(routes!(get_main_zone_by_name))
        .routes(routes!(get_zone2_by_name))
        .routes(routes!(get_zone3_by_name))
        .routes(routes!(get_system_settings_by_name))
        .routes(routes!(get_audio_settings_by_name))
        .routes(routes!(get_imax_config_by_name))
        .routes(routes!(get_network_config_by_name))
        .routes(routes!(get_hdmi_config_by_name))
}

// --- Request/Response DTOs ---

#[derive(Deserialize, ToSchema)]
pub struct PowerRequest {
    /// Whether to set power active (true) or standby (false)
    active: bool,
}

#[derive(Serialize, ToSchema)]
pub struct PowerResponse {
    /// Current power status (e.g. "active", "standby")
    status: String,
}

#[derive(Deserialize, ToSchema)]
pub struct VolumeRequest {
    /// Volume level (0-100)
    #[schema(minimum = 0, maximum = 100)]
    level: u32,
}

#[derive(Serialize, ToSchema)]
pub struct VolumeResponse {
    /// Current volume level
    volume: u32,
    /// Mute status string
    mute: String,
    /// Minimum supported volume
    min_volume: u32,
    /// Maximum supported volume
    max_volume: u32,
}

#[derive(Deserialize, ToSchema)]
pub struct MuteRequest {
    /// Whether to mute (true) or unmute (false)
    mute: bool,
}

#[derive(Serialize, ToSchema)]
pub struct MuteResponse {
    /// Whether the receiver is currently muted
    muted: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct InputRequest {
    /// Input URI (e.g. "extInput:hdmi?port=1")
    uri: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SourceRequest {
    /// Source category (e.g. "GAME", "STB", "BD", "SAT", "VIDEO", "AUX", "TV", "CD")
    category: String,
}

#[derive(Serialize, ToSchema)]
pub struct SourceResponse {
    /// Source category (e.g. "GAME", "STB", "BD")
    category: String,
    /// User-defined display name
    name: String,
    /// HDMI port assignment (e.g. "HDMI 1", "HDMI 3")
    hdmi_assign: String,
    /// Icon identifier used in input URIs (e.g. "game", "mediaBox")
    icon: String,
    /// Whether the source is visible in the receiver UI
    visible: bool,
    /// Sound field preset (e.g. "A.F.D.", "2ch Stereo")
    sound_field: String,
}

#[derive(Serialize, ToSchema)]
pub struct NetworkConfigResponse {
    /// Connection type ("wired" or "wireless")
    connection_type: String,
    /// IPv4 DHCP enabled
    ipv4_dhcp: bool,
    /// IPv4 address
    ipv4_address: String,
    /// IPv4 subnet mask
    ipv4_subnet: String,
    /// IPv4 gateway
    ipv4_gateway: String,
    /// Primary DNS server
    dns1: String,
    /// Secondary DNS server (null if unavailable)
    dns2: Option<String>,
    /// IPv6 enabled
    ipv6_enabled: bool,
    /// WiFi SSID (null if wired)
    wifi_ssid: Option<String>,
    /// WiFi auth type (null if wired)
    wifi_auth: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct HdmiConfigResponse {
    /// 4K/8K upscaling mode
    scaling_4k8k: String,
    /// HDMI CEC control enabled
    cec: bool,
    /// Standby link mode
    standby_link: String,
    /// HDMI passthrough mode
    passthrough: String,
    /// Audio return channel mode (e.g. "earc")
    audio_return_channel: String,
    /// Audio output target (e.g. "amp")
    audio_out: String,
    /// Zone 2 audio output
    zone2_audio_out: String,
    /// Subwoofer level mode
    subwoofer_level: String,
    /// HDMI output 2 mode (e.g. "zone2")
    out2: String,
    /// Supported video formats on output A
    video_format_a: String,
    /// Supported HDR formats on output A
    hdr_format_a: String,
    /// Other features on output A
    other_features_a: String,
    /// Supported video formats on output B
    video_format_b: String,
    /// Supported HDR formats on output B
    hdr_format_b: String,
    /// Other features on output B
    other_features_b: String,
    /// Fast View mode enabled
    fast_view: bool,
    /// Per-port signal format settings
    port_signal_formats: Vec<HdmiPortSignalFormatResponse>,
    /// Per-source HDMI assignments
    source_assignments: Vec<HdmiSourceAssignmentResponse>,
    /// Picture-in-picture output 4 enabled
    out4_pip: bool,
}

#[derive(Serialize, ToSchema)]
pub struct HdmiPortSignalFormatResponse {
    /// Port number (1-7)
    port: u8,
    /// Signal format (e.g. "enhancedformat4k120_8k")
    signal_format: String,
}

#[derive(Serialize, ToSchema)]
pub struct HdmiSourceAssignmentResponse {
    /// Source name (e.g. "game", "mediabox")
    source: String,
    /// Signal format for this source
    signal_format: String,
}

#[derive(Serialize, ToSchema)]
pub struct ImaxConfigResponse {
    /// Audio upmixer mode (e.g. "auto", "off")
    upmixer: String,
    /// Virtualizer enabled
    virtualizer: bool,
    /// IMAX mode (e.g. "auto", "off")
    mode: String,
    /// HPF crossover frequencies per speaker position
    crossovers: Vec<ImaxCrossoverResponse>,
    /// Subwoofer low-pass filter frequency (null if unavailable)
    lpf_subwoofer: Option<String>,
    /// Subwoofer volume level (null if unavailable)
    subwoofer_volume: Option<String>,
    /// Subwoofer bass redirect enabled
    subwoofer_redirect: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ImaxCrossoverResponse {
    /// Speaker position (e.g. "Front", "Center", "Height 1")
    position: String,
    /// Crossover frequency value (null if position not configured)
    value: Option<String>,
}

// --- Named Device Handlers (multi-device via config) ---

#[derive(Serialize, ToSchema)]
pub struct ZoneResponse {
    /// Zone number (2 or 3)
    zone: u8,
    /// Power status ("on" or "off")
    power: String,
    /// Volume level
    volume: String,
    /// Current input
    input: String,
}

#[derive(Serialize, ToSchema)]
pub struct MainZoneResponse {
    /// Power status ("on" or "off")
    power: String,
    /// Volume level
    volume: String,
    /// Mute status ("on" or "off")
    mute: String,
    /// Current input
    input: String,
}

#[derive(Serialize, ToSchema)]
pub struct SystemSettingsResponse {
    /// Volume display units ("dB" or "linear")
    volume_display: Option<String>,
    /// Display dimmer state
    dimmer: Option<String>,
    /// Device name
    device_name: Option<String>,
    /// Wired network status
    wired: Option<String>,
    /// Wireless network status
    wireless: Option<String>,
    /// Internet connectivity status
    internet: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AudioSettingsResponse {
    /// Headphone insertion state
    headphones_inserted: bool,
    /// Current sound field (e.g. "dolby_mode", "2ch_stereo")
    sound_field: String,
    /// 360 Spatial Sound Mapping enabled
    spatial_sound_360: bool,
    /// Speaker relocation enabled
    speaker_relocation: bool,
    /// DSD Native playback enabled
    dsd_native: bool,
    /// Pure Direct mode enabled
    pure_direct: bool,
    /// Subwoofer low-pass filter enabled
    subwoofer_lpf: bool,
    /// A/V sync delay (ms)
    av_sync: String,
    /// Dual mono mode (e.g. "main", "sub", "main/sub")
    dual_mono: String,
    /// Dynamic range compression enabled
    dynamic_range_compression: bool,
    /// Bluetooth mode (e.g. "rx", "tx", "off")
    bluetooth_mode: String,
}

#[utoipa::path(
    get,
    path = "/{name}/power",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Current power status", body = PowerResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_power_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    debug!(device = %name, "Getting power status");
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.clone()))?;

    let status = with_timeout(state.request_timeout, sony.get_power_status()).await?;
    info!(device = %name, status = %status, "Power status retrieved");

    Ok(Json(PowerResponse { status }))
}

#[utoipa::path(
    post,
    path = "/{name}/power",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = PowerRequest,
    responses(
        (status = 200, description = "Power state updated", body = PowerResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_power_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<PowerRequest>,
) -> Result<impl IntoResponse, ServerError> {
    info!(device = %name, active = req.active, "Setting power");
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.clone()))?;

    with_timeout(state.request_timeout, sony.set_power(req.active)).await?;

    let new_status = if req.active { "active" } else { "standby" };
    info!(device = %name, status = %new_status, "Power set successfully");

    Ok(Json(PowerResponse {
        status: new_status.to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/volume",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Current volume info", body = VolumeResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_volume_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    debug!(device = %name, "Getting volume");
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.clone()))?;

    let info = with_timeout(state.request_timeout, sony.get_volume()).await?;

    Ok(Json(VolumeResponse {
        volume: info.volume,
        mute: info.mute,
        min_volume: info.min_volume,
        max_volume: info.max_volume,
    }))
}

#[utoipa::path(
    post,
    path = "/{name}/volume",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = VolumeRequest,
    responses(
        (status = 200, description = "Volume updated", body = inline(serde_json::Value)),
        (status = 400, description = "Invalid volume level", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_volume_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<VolumeRequest>,
) -> Result<impl IntoResponse, ServerError> {
    if req.level > 100 {
        return Err(ServerError::InvalidVolume(format!(
            "level must be 0-100, got {}",
            req.level
        )));
    }

    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    with_timeout(state.request_timeout, sony.set_volume(req.level)).await?;

    Ok(Json(serde_json::json!({ "volume": req.level })))
}

#[utoipa::path(
    get,
    path = "/{name}/mute",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Current mute status", body = MuteResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_mute_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let muted = with_timeout(state.request_timeout, sony.get_mute_status()).await?;

    Ok(Json(MuteResponse { muted }))
}

#[utoipa::path(
    post,
    path = "/{name}/mute",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = MuteRequest,
    responses(
        (status = 200, description = "Mute state updated", body = MuteResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_mute_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<MuteRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    with_timeout(state.request_timeout, sony.set_mute(req.mute)).await?;

    Ok(Json(MuteResponse { muted: req.mute }))
}

#[utoipa::path(
    get,
    path = "/{name}/inputs",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "List of available inputs", body = inline(Vec<serde_json::Value>)),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn list_inputs_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let inputs = with_timeout(state.request_timeout, sony.list_inputs()).await?;

    Ok(Json(inputs))
}

#[utoipa::path(
    get,
    path = "/{name}/sources",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Audio sources with user-defined names and assignments", body = Vec<SourceResponse>),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn list_sources_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let inputs = with_timeout(state.request_timeout, sony.get_native_inputs()).await?;

    let response: Vec<SourceResponse> = inputs
        .into_iter()
        .map(|i| SourceResponse {
            category: i.category,
            name: i.name,
            hdmi_assign: i.hdmi_assign,
            icon: i.icon,
            visible: i.visible,
            sound_field: i.sound_field,
        })
        .collect();

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/{name}/input/current",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Currently selected input", body = inline(serde_json::Value)),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_current_input_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let info = with_timeout(state.request_timeout, sony.get_current_input()).await?;

    Ok(Json(info))
}

#[utoipa::path(
    post,
    path = "/{name}/input",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = InputRequest,
    responses(
        (status = 200, description = "Input changed", body = inline(serde_json::Value)),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_input_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<InputRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    with_timeout(state.request_timeout, sony.set_input(&req.uri)).await?;

    Ok(Json(serde_json::json!({ "uri": req.uri })))
}

/// Resolves a source category to the actual input URI by querying the receiver.
///
/// Fetches both native inputs (category-to-HDMI mapping) and the input list
/// (real URIs), then finds the URI whose HDMI port matches the category's
/// HDMI assignment.
async fn resolve_category_to_uri(
    sony: &homelab::sony_receiver::SonyReceiver,
    category: &str,
    timeout_duration: Duration,
) -> Result<String, ServerError> {
    let (native, inputs) = tokio::join!(
        with_timeout(timeout_duration, sony.get_native_inputs()),
        with_timeout(timeout_duration, sony.list_inputs()),
    );
    let native = native?;
    let inputs = inputs?;

    // Find this category's HDMI port from native config
    let native_input = native
        .iter()
        .find(|n| n.category == category)
        .ok_or_else(|| {
            ServerError::InvalidParameter(format!(
                "unknown source category \"{category}\", valid: {}",
                native
                    .iter()
                    .map(|n| n.category.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

    // Match against real input URIs: find the HDMI input whose port matches
    let hdmi_port = crate::extract_hdmi_port(&native_input.hdmi_assign);
    if let Some(port) = hdmi_port {
        let target = format!("extInput:hdmi?port={port}");
        if let Some(input) = inputs.iter().find(|i| i.uri == target) {
            return Ok(input.uri.clone());
        }
    }

    // Fallback: try matching by icon/title against input URIs
    // (handles non-HDMI inputs like TV via eARC)
    for input in &inputs {
        let matched = crate::match_uri_to_category(&input.uri, Some(&native));
        if matched.as_deref() == Some(category) {
            return Ok(input.uri.clone());
        }
    }

    Err(ServerError::InvalidParameter(format!(
        "could not resolve category \"{category}\" to an input URI"
    )))
}

#[utoipa::path(
    post,
    path = "/{name}/source",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = SourceRequest,
    responses(
        (status = 200, description = "Source changed", body = inline(serde_json::Value)),
        (status = 400, description = "Invalid source category", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_source_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SourceRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let category = req.category.to_uppercase();

    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let uri = resolve_category_to_uri(&sony, &category, state.request_timeout).await?;

    with_timeout(state.request_timeout, sony.set_input(&uri)).await?;

    Ok(Json(
        serde_json::json!({ "category": category, "uri": uri }),
    ))
}

#[utoipa::path(
    get,
    path = "/{name}/system/info",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "System information", body = inline(serde_json::Value)),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_system_info_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let info = with_timeout(state.request_timeout, sony.get_system_information()).await?;

    Ok(Json(info))
}

#[utoipa::path(
    get,
    path = "/{name}/zone",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Main zone status", body = MainZoneResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_main_zone_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let status = with_timeout(state.request_timeout, sony.get_main_zone_status()).await?;

    Ok(Json(MainZoneResponse {
        power: status.power,
        volume: status.volume,
        mute: status.mute,
        input: status.input,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/zone2",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Zone 2 status", body = ZoneResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_zone2_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let status = with_timeout(state.request_timeout, sony.get_zone2_status()).await?;

    Ok(Json(ZoneResponse {
        zone: status.zone,
        power: status.power,
        volume: status.volume,
        input: status.input,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/zone3",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Zone 3 status", body = ZoneResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_zone3_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let status = with_timeout(state.request_timeout, sony.get_zone3_status()).await?;

    Ok(Json(ZoneResponse {
        zone: status.zone,
        power: status.power,
        volume: status.volume,
        input: status.input,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/system/settings",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "System settings", body = SystemSettingsResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_system_settings_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let settings = with_timeout(state.request_timeout, sony.get_system_settings()).await?;

    Ok(Json(SystemSettingsResponse {
        volume_display: settings.volume_display,
        dimmer: settings.dimmer,
        device_name: settings.device_name,
        wired: settings.network.wired,
        wireless: settings.network.wireless,
        internet: settings.network.internet,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/audio/settings",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Audio settings", body = AudioSettingsResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_audio_settings_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let settings = with_timeout(state.request_timeout, sony.get_audio_settings()).await?;

    Ok(Json(AudioSettingsResponse {
        headphones_inserted: settings.headphones_inserted,
        sound_field: settings.sound_field,
        spatial_sound_360: settings.spatial_sound_360,
        speaker_relocation: settings.speaker_relocation,
        dsd_native: settings.dsd_native,
        pure_direct: settings.pure_direct,
        subwoofer_lpf: settings.subwoofer_lpf,
        av_sync: settings.av_sync,
        dual_mono: settings.dual_mono,
        dynamic_range_compression: settings.dynamic_range_compression,
        bluetooth_mode: settings.bluetooth_mode,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/audio/imax",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "IMAX Enhanced audio configuration", body = ImaxConfigResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_imax_config_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let config = with_timeout(state.request_timeout, sony.get_imax_config()).await?;

    Ok(Json(ImaxConfigResponse {
        upmixer: config.upmixer,
        virtualizer: config.virtualizer,
        mode: config.mode,
        crossovers: config
            .crossovers
            .into_iter()
            .map(|c| ImaxCrossoverResponse {
                position: c.position,
                value: c.value,
            })
            .collect(),
        lpf_subwoofer: config.lpf_subwoofer,
        subwoofer_volume: config.subwoofer_volume,
        subwoofer_redirect: config.subwoofer_redirect,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/network",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Network configuration", body = NetworkConfigResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_network_config_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let config = with_timeout(state.request_timeout, sony.get_network_config()).await?;

    Ok(Json(NetworkConfigResponse {
        connection_type: config.connection_type,
        ipv4_dhcp: config.ipv4_dhcp,
        ipv4_address: config.ipv4_address,
        ipv4_subnet: config.ipv4_subnet,
        ipv4_gateway: config.ipv4_gateway,
        dns1: config.dns1,
        dns2: config.dns2,
        ipv6_enabled: config.ipv6_enabled,
        wifi_ssid: config.wifi_ssid,
        wifi_auth: config.wifi_auth,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/hdmi",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "HDMI configuration", body = HdmiConfigResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_hdmi_config_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let config = with_timeout(state.request_timeout, sony.get_hdmi_config()).await?;

    Ok(Json(HdmiConfigResponse {
        scaling_4k8k: config.scaling_4k8k,
        cec: config.cec,
        standby_link: config.standby_link,
        passthrough: config.passthrough,
        audio_return_channel: config.audio_return_channel,
        audio_out: config.audio_out,
        zone2_audio_out: config.zone2_audio_out,
        subwoofer_level: config.subwoofer_level,
        out2: config.out2,
        video_format_a: config.video_format_a,
        hdr_format_a: config.hdr_format_a,
        other_features_a: config.other_features_a,
        video_format_b: config.video_format_b,
        hdr_format_b: config.hdr_format_b,
        other_features_b: config.other_features_b,
        fast_view: config.fast_view,
        port_signal_formats: config
            .port_signal_formats
            .into_iter()
            .map(|p| HdmiPortSignalFormatResponse {
                port: p.port,
                signal_format: p.signal_format,
            })
            .collect(),
        source_assignments: config
            .source_assignments
            .into_iter()
            .map(|s| HdmiSourceAssignmentResponse {
                source: s.source,
                signal_format: s.signal_format,
            })
            .collect(),
        out4_pip: config.out4_pip,
    }))
}

// --- Helpers ---

async fn with_timeout<T, E>(
    duration: Duration,
    future: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, ServerError>
where
    E: Into<ServerError>,
{
    match timeout(duration, future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => {
            let err: ServerError = e.into();
            warn!(error = %err, "Sony API error");
            Err(err)
        }
        Err(_) => {
            warn!("Sony API timeout after {:?}", duration);
            Err(ServerError::Timeout)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{extract_hdmi_port, match_uri_to_category};
    use homelab::sony_receiver::NativeInputConfig;

    fn make_source(category: &str, name: &str, hdmi: &str, icon: &str) -> NativeInputConfig {
        NativeInputConfig {
            category: category.to_string(),
            name: name.to_string(),
            hdmi_assign: hdmi.to_string(),
            icon: icon.to_string(),
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
        }
    }

    #[test]
    fn resolve_hdmi_port_from_category() {
        // Verify native config -> HDMI port extraction works for resolving categories
        let inputs = vec![
            make_source("GAME", "", "in1", ""),
            make_source("STB", "", "in2", ""),
            make_source("TV", "", "eARC/OUT A", ""),
        ];
        assert_eq!(extract_hdmi_port(&inputs[0].hdmi_assign), Some(1));
        assert_eq!(extract_hdmi_port(&inputs[1].hdmi_assign), Some(2));
        assert_eq!(extract_hdmi_port(&inputs[2].hdmi_assign), None); // eARC has no port

        // match_uri_to_category reverse-matches HDMI URIs back to categories
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=1", Some(&inputs)),
            Some("GAME".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=2", Some(&inputs)),
            Some("STB".to_string())
        );
    }
}
