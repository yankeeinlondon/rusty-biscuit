use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Sony receiver routes (legacy - single device)
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_power, set_power))
        .routes(routes!(get_volume, set_volume))
        .routes(routes!(get_mute, set_mute))
        .routes(routes!(list_inputs))
        .routes(routes!(get_current_input))
        .routes(routes!(set_input))
        .routes(routes!(get_system_info))
}

/// Create all Sony receiver routes with device name parameter
pub fn routes_with_name() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_power_by_name, set_power_by_name))
        .routes(routes!(get_volume_by_name, set_volume_by_name))
        .routes(routes!(get_mute_by_name, set_mute_by_name))
        .routes(routes!(list_inputs_by_name))
        .routes(routes!(get_current_input_by_name))
        .routes(routes!(set_input_by_name))
        .routes(routes!(get_system_info_by_name))
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

// --- Legacy Handlers (single device via ENV) ---

#[utoipa::path(
    get,
    path = "/power",
    tag = "sony",
    responses(
        (status = 200, description = "Current power status", body = PowerResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_power(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    let status = with_timeout(state.request_timeout, sony.get_power_status()).await?;

    Ok(Json(PowerResponse { status }))
}

#[utoipa::path(
    post,
    path = "/power",
    tag = "sony",
    request_body = PowerRequest,
    responses(
        (status = 200, description = "Power state updated", body = PowerResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_power(
    State(state): State<AppState>,
    Json(req): Json<PowerRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    with_timeout(state.request_timeout, sony.set_power(req.active)).await?;

    Ok(Json(PowerResponse {
        status: if req.active {
            "active".to_string()
        } else {
            "standby".to_string()
        },
    }))
}

#[utoipa::path(
    get,
    path = "/volume",
    tag = "sony",
    responses(
        (status = 200, description = "Current volume info", body = VolumeResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_volume(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

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
    path = "/volume",
    tag = "sony",
    request_body = VolumeRequest,
    responses(
        (status = 200, description = "Volume updated", body = inline(serde_json::Value)),
        (status = 400, description = "Invalid volume level", body = ErrorResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_volume(
    State(state): State<AppState>,
    Json(req): Json<VolumeRequest>,
) -> Result<impl IntoResponse, ServerError> {
    // Validate volume range
    if req.level > 100 {
        return Err(ServerError::InvalidVolume(format!(
            "level must be 0-100, got {}",
            req.level
        )));
    }

    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    with_timeout(state.request_timeout, sony.set_volume(req.level)).await?;

    Ok(Json(serde_json::json!({ "volume": req.level })))
}

#[utoipa::path(
    get,
    path = "/mute",
    tag = "sony",
    responses(
        (status = 200, description = "Current mute status", body = MuteResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_mute(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    let muted = with_timeout(state.request_timeout, sony.get_mute_status()).await?;

    Ok(Json(MuteResponse { muted }))
}

#[utoipa::path(
    post,
    path = "/mute",
    tag = "sony",
    request_body = MuteRequest,
    responses(
        (status = 200, description = "Mute state updated", body = MuteResponse),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_mute(
    State(state): State<AppState>,
    Json(req): Json<MuteRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    with_timeout(state.request_timeout, sony.set_mute(req.mute)).await?;

    Ok(Json(MuteResponse { muted: req.mute }))
}

#[utoipa::path(
    get,
    path = "/inputs",
    tag = "sony",
    responses(
        (status = 200, description = "List of available inputs", body = inline(Vec<serde_json::Value>)),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn list_inputs(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    let inputs = with_timeout(state.request_timeout, sony.list_inputs()).await?;

    Ok(Json(inputs))
}

#[utoipa::path(
    get,
    path = "/input/current",
    tag = "sony",
    responses(
        (status = 200, description = "Currently selected input", body = inline(serde_json::Value)),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_current_input(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    let info = with_timeout(state.request_timeout, sony.get_current_input()).await?;

    Ok(Json(info))
}

#[utoipa::path(
    post,
    path = "/input",
    tag = "sony",
    request_body = InputRequest,
    responses(
        (status = 200, description = "Input changed", body = inline(serde_json::Value)),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_input(
    State(state): State<AppState>,
    Json(req): Json<InputRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    with_timeout(state.request_timeout, sony.set_input(&req.uri)).await?;

    Ok(Json(serde_json::json!({ "uri": req.uri })))
}

#[utoipa::path(
    get,
    path = "/system/info",
    tag = "sony",
    responses(
        (status = 200, description = "System information", body = inline(serde_json::Value)),
        (status = 404, description = "Device not configured", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_system_info(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    let sony = state
        .sony
        .as_ref()
        .ok_or(ServerError::DeviceNotConfigured("Sony Receiver"))?;

    let info = with_timeout(state.request_timeout, sony.get_system_information()).await?;

    Ok(Json(info))
}

// --- Named Device Handlers (multi-device via config) ---

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
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    let status = with_timeout(state.request_timeout, sony.get_power_status()).await?;

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
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

    with_timeout(state.request_timeout, sony.set_power(req.active)).await?;

    Ok(Json(PowerResponse {
        status: if req.active {
            "active".to_string()
        } else {
            "standby".to_string()
        },
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
    let sony = state
        .get_sony(&name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name))?;

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
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(ServerError::Timeout),
    }
}
