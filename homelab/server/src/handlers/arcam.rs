use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use homelab::arcam::{Arcam, ArcamSystemStatus, ArcamError};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{warn, debug};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Arcam amplifier routes with device name parameter
pub fn routes_with_name() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_status_by_name))
        .routes(routes!(get_power_by_name))
        .routes(routes!(power_on_by_name))
        .routes(routes!(power_off_by_name))
        .routes(routes!(get_mute_by_name))
        .routes(routes!(mute_on_by_name))
        .routes(routes!(mute_off_by_name))
        .routes(routes!(get_mode_by_name))
        .routes(routes!(get_temperature_by_name))
        .routes(routes!(get_timeout_by_name))
        .routes(routes!(get_name_by_name))
        .routes(routes!(set_name_by_name))
        .routes(routes!(get_auto_shutdown_by_name))
        .routes(routes!(set_auto_shutdown_by_name))
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

#[derive(Serialize, ToSchema)]
pub struct SystemStatusResponse {
    /// Power state
    pub power: bool,
    /// Mute state
    pub muted: bool,
    /// System model name (e.g., "PA240")
    pub model: Option<String>,
    /// Software version
    pub software_version: Option<String>,
    /// Amplifier mode
    pub amplifier_mode: Option<String>,
    /// Auto shutdown setting
    pub auto_shutdown: Option<String>,
    /// Seconds until auto-standby
    pub timeout_counter: Option<u16>,
    /// Input signal detected
    pub input_detect: Option<bool>,
    /// Friendly name
    pub friendly_name: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
}

impl From<ArcamSystemStatus> for SystemStatusResponse {
    fn from(s: ArcamSystemStatus) -> Self {
        Self {
            power: s.power,
            muted: s.muted,
            model: s.model,
            software_version: s.software_version,
            amplifier_mode: s.amplifier_mode.map(|m| match m {
                1 => "Stereo",
                2 => "Bridged",
                3 => "DualMono",
                _ => "Unknown",
            }.to_string()),
            auto_shutdown: s.auto_shutdown.map(|v| homelab::arcam::auto_shutdown_label(v).to_string()),
            timeout_counter: s.timeout_counter,
            input_detect: s.input_detect,
            friendly_name: s.friendly_name,
            ip_address: s.ip_address,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct TemperatureResponse {
    /// Sensor type (lifter or output)
    pub sensor: String,
    /// Temperature in Celsius
    pub celsius: u8,
}

#[derive(Serialize, ToSchema)]
pub struct TimeoutResponse {
    /// Seconds remaining until auto-standby
    pub seconds_remaining: u16,
    /// Human-readable formatted string
    pub formatted: String,
}

#[derive(Serialize, ToSchema)]
pub struct NameResponse {
    /// Friendly name
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SetNameRequest {
    /// New friendly name (max 10 characters, A-Z, 0-9, space)
    pub name: String,
}

// --- Named Device Handlers (multi-device via config) ---

#[utoipa::path(
    get,
    path = "/{name}/status",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Complete system status", body = SystemStatusResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_status_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    debug!(device = %name, "Getting Arcam status");
    let arcam = create_arcam_by_name(&state, &name).await?;

    let status = with_timeout(state.request_timeout, arcam.get_system_status()).await?;

    Ok(Json(SystemStatusResponse::from(status)))
}

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

    let result = with_timeout(state.request_timeout, arcam.get_amplifier_mode()).await;

    let mode = match result {
        Ok(mode_byte) => match mode_byte {
            1 => "Stereo",
            2 => "Bridged",
            3 => "DualMono",
            _ => "Unknown",
        },
        Err(e) => {
            let err: ServerError = e;
            match err {
                ServerError::Arcam(ArcamError::ParameterNotRecognised(_)) => "N/A",
                _ => return Err(err),
            }
        }
    };

    Ok(Json(ModeResponse { mode }))
}

#[utoipa::path(
    get,
    path = "/{name}/temperature/{sensor_type}",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name"),
        ("sensor_type" = String, Path, description = "Sensor type: lifter or output")
    ),
    responses(
        (status = 200, description = "Temperature reading", body = TemperatureResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_temperature_by_name(
    State(state): State<AppState>,
    Path((name, sensor_type)): Path<(String, String)>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    let sensor = 0xF0; // Default to sensor 1

    let (sensor_name, temp) = match sensor_type.as_str() {
        "lifter" => {
            let t = with_timeout(state.request_timeout, arcam.get_lifter_temperature(sensor)).await?;
            ("lifter".to_string(), t)
        }
        "output" => {
            let t = with_timeout(state.request_timeout, arcam.get_output_temperature(sensor)).await?;
            ("output".to_string(), t)
        }
        _ => return Err(ServerError::InvalidParameter("sensor_type must be lifter or output".into())),
    };

    Ok(Json(TemperatureResponse {
        sensor: sensor_name,
        celsius: temp,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/timeout",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Timeout counter", body = TimeoutResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_timeout_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    let seconds = with_timeout(state.request_timeout, arcam.get_timeout_counter()).await?;

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let formatted = if hours > 0 {
        format!("{} hours {} minutes", hours, minutes)
    } else if minutes > 0 {
        format!("{} minutes", minutes)
    } else if seconds > 0 {
        format!("{} seconds", seconds)
    } else {
        "Ready to power off".to_string()
    };

    Ok(Json(TimeoutResponse {
        seconds_remaining: seconds,
        formatted,
    }))
}

#[utoipa::path(
    get,
    path = "/{name}/name",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Friendly name", body = NameResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_name_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    let name_result = with_timeout(state.request_timeout, arcam.get_friendly_name()).await?;

    Ok(Json(NameResponse { name: name_result }))
}

#[utoipa::path(
    put,
    path = "/{name}/name",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = SetNameRequest,
    responses(
        (status = 200, description = "Friendly name set", body = NameResponse),
        (status = 400, description = "Invalid name", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_name_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SetNameRequest>,
) -> Result<impl IntoResponse, ServerError> {
    if req.name.len() > 10 {
        return Err(ServerError::InvalidParameter("name must be 10 characters or less".into()));
    }

    let arcam = create_arcam_by_name(&state, &name).await?;

    let name_result = with_timeout(state.request_timeout, arcam.set_friendly_name(&req.name)).await?;

    Ok(Json(NameResponse { name: name_result }))
}

#[derive(Deserialize, ToSchema)]
pub struct SetAutoShutdownRequest {
    /// Auto shutdown value: 0=off, 1=20min, 2=30min, 3=1hr, 4=2hr
    pub value: u8,
}

#[derive(Serialize, ToSchema)]
pub struct AutoShutdownResponse {
    /// Current auto shutdown value
    pub value: u8,
    /// Human-readable label
    pub label: String,
}

#[utoipa::path(
    get,
    path = "/{name}/auto-shutdown",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 200, description = "Auto shutdown setting", body = AutoShutdownResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_auto_shutdown_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let arcam = create_arcam_by_name(&state, &name).await?;

    let value = with_timeout(state.request_timeout, arcam.get_auto_shutdown()).await?;

    let label = homelab::arcam::auto_shutdown_label(value).to_string();

    Ok(Json(AutoShutdownResponse { value, label }))
}

#[utoipa::path(
    put,
    path = "/{name}/auto-shutdown",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = SetAutoShutdownRequest,
    responses(
        (status = 200, description = "Auto shutdown set", body = AutoShutdownResponse),
        (status = 400, description = "Invalid value", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_auto_shutdown_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SetAutoShutdownRequest>,
) -> Result<impl IntoResponse, ServerError> {
    if req.value > 4 {
        return Err(ServerError::InvalidParameter(
            "value must be 0-4 (0=off, 1=20min, 2=30min, 3=1hr, 4=2hr)".into(),
        ));
    }

    let arcam = create_arcam_by_name(&state, &name).await?;

    let value = with_timeout(state.request_timeout, arcam.set_auto_shutdown(req.value)).await?;

    let label = homelab::arcam::auto_shutdown_label(value).to_string();

    Ok(Json(AutoShutdownResponse { value, label }))
}

// --- Helpers ---

async fn create_arcam_by_name(state: &AppState, name: &str) -> Result<Arcam, ServerError> {
    let (host_str, _port) = state
        .get_arcam_host(name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.to_string()))?;

    // Note: Arcam port is hardcoded to 50000 in the library
    let host = homelab::network::parse_host(&host_str)
        .map_err(|e| ServerError::InvalidHost(e.0))?;
    Ok(Arcam::new(host))
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
        Ok(Err(e)) => {
            let err: ServerError = e.into();
            warn!(error = %err, "Arcam API error");
            Err(err)
        }
        Err(_) => {
            warn!("Arcam API timeout after {:?}", duration);
            Err(ServerError::Timeout)
        }
    }
}