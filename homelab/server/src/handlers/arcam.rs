use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use homelab::{arcam::Arcam, network::Host};
use serde::Serialize;
use std::{net::Ipv4Addr, time::Duration};
use tokio::time::timeout;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Arcam amplifier routes (legacy - single device)
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_power))
        .routes(routes!(power_on))
        .routes(routes!(power_off))
        .routes(routes!(get_mute))
        .routes(routes!(mute_on))
        .routes(routes!(mute_off))
        .routes(routes!(get_mode))
}

/// Create all Arcam amplifier routes with device name parameter
pub fn routes_with_name() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_power_by_name))
        .routes(routes!(power_on_by_name))
        .routes(routes!(power_off_by_name))
        .routes(routes!(get_mute_by_name))
        .routes(routes!(mute_on_by_name))
        .routes(routes!(mute_off_by_name))
        .routes(routes!(get_mode_by_name))
}

// --- Response DTOs ---

#[derive(Serialize, ToSchema)]
pub struct PowerResponse {
    /// Whether the amplifier is powered on
    on: bool,
}

#[derive(Serialize, ToSchema)]
pub struct MuteResponse {
    /// Whether the amplifier is muted
    muted: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ModeResponse {
    /// Amplifier mode (Stereo, Bridged, DualMono, or Unknown)
    mode: &'static str,
}

// --- Legacy Handlers (single device via ENV) ---

#[utoipa::path(
    get,
    path = "/power",
    tag = "arcam",
    responses(
        (status = 200, description = "Current power state", body = PowerResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_power(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam(&state)?;

    let on = with_timeout(state.request_timeout, arcam.request_power_state()).await?;

    Ok(Json(PowerResponse { on }))
}

#[utoipa::path(
    post,
    path = "/power/on",
    tag = "arcam",
    responses(
        (status = 200, description = "Power turned on", body = PowerResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn power_on(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam(&state)?;

    with_timeout(state.request_timeout, arcam.power_on()).await?;

    Ok(Json(PowerResponse { on: true }))
}

#[utoipa::path(
    post,
    path = "/power/off",
    tag = "arcam",
    responses(
        (status = 200, description = "Power turned off", body = PowerResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn power_off(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam(&state)?;

    with_timeout(state.request_timeout, arcam.power_off()).await?;

    Ok(Json(PowerResponse { on: false }))
}

#[utoipa::path(
    get,
    path = "/mute",
    tag = "arcam",
    responses(
        (status = 200, description = "Current mute state", body = MuteResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_mute(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam(&state)?;

    let muted = with_timeout(state.request_timeout, arcam.get_mute_status()).await?;

    Ok(Json(MuteResponse { muted }))
}

#[utoipa::path(
    post,
    path = "/mute/on",
    tag = "arcam",
    responses(
        (status = 200, description = "Mute enabled", body = MuteResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn mute_on(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam(&state)?;

    with_timeout(state.request_timeout, arcam.mute_on()).await?;

    Ok(Json(MuteResponse { muted: true }))
}

#[utoipa::path(
    post,
    path = "/mute/off",
    tag = "arcam",
    responses(
        (status = 200, description = "Mute disabled", body = MuteResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn mute_off(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam(&state)?;

    with_timeout(state.request_timeout, arcam.mute_off()).await?;

    Ok(Json(MuteResponse { muted: false }))
}

#[utoipa::path(
    get,
    path = "/mode",
    tag = "arcam",
    responses(
        (status = 200, description = "Current amplifier mode", body = ModeResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_mode(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam(&state)?;

    let mode_byte = with_timeout(state.request_timeout, arcam.get_amplifier_mode()).await?;

    let mode = match mode_byte {
        0 => "Stereo",
        1 => "Bridged",
        2 => "DualMono",
        _ => "Unknown",
    };

    Ok(Json(ModeResponse { mode }))
}

// --- Named Device Handlers (multi-device via config) ---

#[utoipa::path(
    get,
    path = "/{name}/power",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Current power state", body = PowerResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_power_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    let on = with_timeout(state.request_timeout, arcam.request_power_state()).await?;

    Ok(Json(PowerResponse { on }))
}

#[utoipa::path(
    post,
    path = "/{name}/power/on",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Power turned on", body = PowerResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn power_on_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    with_timeout(state.request_timeout, arcam.power_on()).await?;

    Ok(Json(PowerResponse { on: true }))
}

#[utoipa::path(
    post,
    path = "/{name}/power/off",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Power turned off", body = PowerResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn power_off_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    with_timeout(state.request_timeout, arcam.power_off()).await?;

    Ok(Json(PowerResponse { on: false }))
}

#[utoipa::path(
    get,
    path = "/{name}/mute",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Current mute state", body = MuteResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_mute_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    let muted = with_timeout(state.request_timeout, arcam.get_mute_status()).await?;

    Ok(Json(MuteResponse { muted }))
}

#[utoipa::path(
    post,
    path = "/{name}/mute/on",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Mute enabled", body = MuteResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn mute_on_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    with_timeout(state.request_timeout, arcam.mute_on()).await?;

    Ok(Json(MuteResponse { muted: true }))
}

#[utoipa::path(
    post,
    path = "/{name}/mute/off",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Mute disabled", body = MuteResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn mute_off_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    with_timeout(state.request_timeout, arcam.mute_off()).await?;

    Ok(Json(MuteResponse { muted: false }))
}

#[utoipa::path(
    get,
    path = "/{name}/mode",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Current amplifier mode", body = ModeResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_mode_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    let mode_byte = with_timeout(state.request_timeout, arcam.get_amplifier_mode()).await?;

    let mode = match mode_byte {
        0 => "Stereo",
        1 => "Bridged",
        2 => "DualMono",
        _ => "Unknown",
    };

    Ok(Json(ModeResponse { mode }))
}

// --- Helpers ---

fn create_arcam(state: &AppState) -> Result<Arcam, ServerError> {
    let host_str = state
        .arcam_host
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Arcam Amplifier"))?;

    let host = parse_host(host_str);
    Ok(Arcam::new(host))
}

async fn create_arcam_by_name(state: &AppState, name: &str) -> Result<Arcam, ServerError> {
    let (host_str, _port) = state
        .get_arcam_host(name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.to_string()))?;

    // Note: Arcam port is hardcoded to 50000 in the library
    let host = parse_host(&host_str);
    Ok(Arcam::new(host))
}

fn parse_host(host_str: &str) -> Host {
    // Try IPv4 first
    if let Ok(ipv4) = host_str.parse::<Ipv4Addr>() {
        return Host::V4(ipv4);
    }

    // Try IPv6
    if let Ok(ipv6) = host_str.parse() {
        return Host::V6(ipv6);
    }

    // Must be DNS
    Host::Dns(host_str.to_string())
}

async fn with_timeout<T, E>(
    duration: Duration,
    future: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, ServerError>
where
    E: Into<ServerError>,
{
    match timeout(duration, future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(ServerError::Timeout),
    }
}
