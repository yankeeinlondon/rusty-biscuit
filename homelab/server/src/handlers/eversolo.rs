//! Eversolo DMP-A8 control endpoints.

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use homelab::eversolo::Eversolo;
use serde::Deserialize;
use tokio::time::timeout;
use tracing::warn;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Eversolo control routes with device name parameter
pub fn routes_with_name() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_info_by_name))
        .routes(routes!(get_state_by_name))
        .routes(routes!(play_pause_by_name))
        .routes(routes!(next_by_name))
        .routes(routes!(previous_by_name))
        .routes(routes!(get_volume_by_name))
        .routes(routes!(set_volume_by_name))
        .routes(routes!(set_mute_by_name))
        .routes(routes!(get_routing_by_name))
        .routes(routes!(set_input_by_name))
        .routes(routes!(set_output_by_name))
        .routes(routes!(get_power_options_by_name))
        .routes(routes!(set_power_by_name))
        .routes(routes!(get_brightness_by_name))
        .routes(routes!(set_brightness_by_name))
        .routes(routes!(get_display_modes_by_name))
        .routes(routes!(set_display_mode_by_name))
        .routes(routes!(wake_by_name))
}

// --- Request DTOs ---

#[derive(Deserialize, ToSchema)]
pub struct VolumeRequest {
    /// Volume level (0 to device max)
    pub level: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct MuteRequest {
    /// Whether to mute
    pub muted: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct TagRequest {
    /// Tag identifier
    pub tag: String,
}

#[derive(Deserialize, ToSchema)]
pub struct BrightnessSetRequest {
    /// Target: "screen" or "knob"
    pub target: String,
    /// Brightness index
    pub index: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct DisplayModeSetRequest {
    /// Target: "vu" or "spectrum"
    pub target: String,
    /// Mode index
    pub index: i64,
}

// --- Handlers ---

#[utoipa::path(
    get,
    path = "/{name}/info",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Device model and firmware info"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_info_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let info = with_timeout(state.request_timeout, eversolo.get_model()).await?;
    Ok(Json(serde_json::to_value(info).unwrap()))
}

#[utoipa::path(
    get,
    path = "/{name}/state",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Current playback state, track info, and volume"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_state_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let playback_state = with_timeout(state.request_timeout, eversolo.get_state()).await?;
    Ok(Json(serde_json::to_value(playback_state).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/play-pause",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Toggled play/pause"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn play_pause_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.play_or_pause()).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/next",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Skipped to next track"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn next_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.play_next()).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/previous",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Skipped to previous track"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn previous_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.play_previous()).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    get,
    path = "/{name}/volume",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Current volume and mute state"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_volume_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let playback_state = with_timeout(state.request_timeout, eversolo.get_state()).await?;
    Ok(Json(
        serde_json::to_value(playback_state.volume_data).unwrap(),
    ))
}

#[utoipa::path(
    put,
    path = "/{name}/volume",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    request_body = VolumeRequest,
    responses(
        (status = 200, description = "Volume set"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_volume_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<VolumeRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.set_volume(req.level)).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/mute",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    request_body = MuteRequest,
    responses(
        (status = 200, description = "Mute state set"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_mute_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<MuteRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.set_mute(req.muted)).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    get,
    path = "/{name}/routing",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Available audio inputs and outputs"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_routing_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let io = with_timeout(state.request_timeout, eversolo.get_inputs_outputs()).await?;
    Ok(Json(serde_json::to_value(io).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/input",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    request_body = TagRequest,
    responses(
        (status = 200, description = "Audio input set"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_input_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<TagRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.set_input(&req.tag)).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/output",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    request_body = TagRequest,
    responses(
        (status = 200, description = "Audio output set"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_output_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<TagRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.set_output(&req.tag)).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    get,
    path = "/{name}/power-options",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Available power options"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_power_options_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let opts = with_timeout(state.request_timeout, eversolo.get_power_options()).await?;
    Ok(Json(serde_json::to_value(opts).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/power",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    request_body = TagRequest,
    responses(
        (status = 200, description = "Power action executed"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_power_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<TagRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = with_timeout(state.request_timeout, eversolo.set_power_option(&req.tag)).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    get,
    path = "/{name}/brightness",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Screen and knob brightness levels"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_brightness_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let screen = with_timeout(state.request_timeout, eversolo.get_screen_brightness()).await?;
    let knob = with_timeout(state.request_timeout, eversolo.get_knob_brightness()).await?;
    Ok(Json(serde_json::json!({
        "screen": serde_json::to_value(screen).unwrap(),
        "knob": serde_json::to_value(knob).unwrap(),
    })))
}

#[utoipa::path(
    put,
    path = "/{name}/brightness",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    request_body = BrightnessSetRequest,
    responses(
        (status = 200, description = "Brightness set"),
        (status = 400, description = "Invalid target", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_brightness_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<BrightnessSetRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = match req.target.as_str() {
        "screen" => {
            with_timeout(
                state.request_timeout,
                eversolo.set_screen_brightness(req.index),
            )
            .await?
        }
        "knob" => {
            with_timeout(
                state.request_timeout,
                eversolo.set_knob_brightness(req.index),
            )
            .await?
        }
        _ => {
            return Err(ServerError::InvalidParameter(format!(
                "target must be 'screen' or 'knob', got '{}'",
                req.target
            )));
        }
    };
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    get,
    path = "/{name}/display-modes",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "VU and spectrum display modes"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_display_modes_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let vu = with_timeout(state.request_timeout, eversolo.get_vu_modes()).await?;
    let spectrum = with_timeout(state.request_timeout, eversolo.get_spectrum_modes()).await?;
    Ok(Json(serde_json::json!({
        "vu": serde_json::to_value(vu).unwrap(),
        "spectrum": serde_json::to_value(spectrum).unwrap(),
    })))
}

#[utoipa::path(
    put,
    path = "/{name}/display-mode",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    request_body = DisplayModeSetRequest,
    responses(
        (status = 200, description = "Display mode set"),
        (status = 400, description = "Invalid target", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Eversolo API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_display_mode_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<DisplayModeSetRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let eversolo = create_eversolo_by_name(&state, &name).await?;
    let resp = match req.target.as_str() {
        "vu" => with_timeout(state.request_timeout, eversolo.set_vu_mode(req.index)).await?,
        "spectrum" => {
            with_timeout(state.request_timeout, eversolo.set_spectrum_mode(req.index)).await?
        }
        _ => {
            return Err(ServerError::InvalidParameter(format!(
                "target must be 'vu' or 'spectrum', got '{}'",
                req.target
            )));
        }
    };
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[utoipa::path(
    put,
    path = "/{name}/wake",
    tag = "eversolo",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Wake-on-LAN packet sent"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 400, description = "No MAC address configured", body = ErrorResponse),
    )
)]
pub(crate) async fn wake_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let mac = {
        let devices = state.eversolo_devices.read().await;
        let service = devices
            .get(&name)
            .ok_or_else(|| ServerError::DeviceNotFound(name.clone()))?;
        service
            .mac_address
            .clone()
            .ok_or_else(|| ServerError::InvalidParameter("no MAC address configured".to_string()))?
    };

    // Eversolo uses port 9517 for WoL
    homelab::wol::send_magic_packet(&mac, "255.255.255.255", 9517)
        .map_err(|e| ServerError::InvalidParameter(e.to_string()))?;

    Ok(Json(serde_json::json!({"wol_sent": true, "mac": mac})))
}

// --- Helpers ---

async fn create_eversolo_by_name(state: &AppState, name: &str) -> Result<Eversolo, ServerError> {
    let (host, port) = state
        .get_eversolo(name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.to_string()))?;
    Ok(Eversolo::new(host, port))
}

async fn with_timeout<T, E>(
    duration: std::time::Duration,
    future: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, ServerError>
where
    E: Into<ServerError>,
{
    match timeout(duration, future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => {
            let err: ServerError = e.into();
            warn!(error = %err, "Eversolo API error");
            Err(err)
        }
        Err(_) => {
            warn!("Eversolo API timeout after {:?}", duration);
            Err(ServerError::Timeout)
        }
    }
}
