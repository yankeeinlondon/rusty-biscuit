//! Samsung Smart TV control endpoints.

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use homelab::samsung_tv::SamsungTv;
use serde::Deserialize;
use tokio::time::timeout;
use tracing::warn;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Samsung TV control routes with device name parameter
pub fn routes_with_name() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_info_by_name))
        .routes(routes!(get_logs_by_name))
        .routes(routes!(launch_app_by_name))
        .routes(routes!(send_key_by_name))
}

// --- Request DTOs ---

#[derive(Deserialize, ToSchema)]
pub struct AppLaunchRequest {
    /// App ID to launch
    #[serde(default)]
    pub id: Option<String>,
    /// App name to launch
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct RemoteKeyRequest {
    /// Remote key name (e.g., KEY_VOLUP, KEY_POWER, KEY_HOME)
    pub key: String,
}

// --- Handlers ---

#[utoipa::path(
    get,
    path = "/{name}/info",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Device information"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_info_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    let info = with_timeout(state.request_timeout, tv.get_device_info()).await?;
    Ok(Json(serde_json::to_value(info).unwrap()))
}

#[utoipa::path(
    get,
    path = "/{name}/logs",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Server logs"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_logs_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    let logs = with_timeout(state.request_timeout, tv.get_server_logs()).await?;
    Ok(logs)
}

#[utoipa::path(
    post,
    path = "/{name}/app",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    request_body = AppLaunchRequest,
    responses(
        (status = 200, description = "App launched"),
        (status = 400, description = "Invalid parameters", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn launch_app_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<AppLaunchRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    match (req.id, req.name) {
        (Some(id), _) => {
            with_timeout(state.request_timeout, tv.launch_app_by_id(&id)).await?;
            Ok(Json(serde_json::json!({"launched_by": "id", "app_id": id})))
        }
        (None, Some(app_name)) => {
            with_timeout(state.request_timeout, tv.launch_app_by_name(&app_name)).await?;
            Ok(Json(serde_json::json!({"launched_by": "name", "app_name": app_name})))
        }
        (None, None) => Err(ServerError::InvalidParameter(
            "either 'id' or 'name' is required".to_string(),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/{name}/remote/key",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    request_body = RemoteKeyRequest,
    responses(
        (status = 200, description = "Key sent"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn send_key_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<RemoteKeyRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.send_key(&req.key)).await?;
    Ok(Json(serde_json::json!({"key": req.key})))
}

// --- Helpers ---

async fn create_samsung_by_name(
    state: &AppState,
    name: &str,
) -> Result<SamsungTv, ServerError> {
    let (host, rest_port, ws_port) = state
        .get_samsung_tv(name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.to_string()))?;
    Ok(SamsungTv::new(host, rest_port, ws_port))
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
            warn!(error = %err, "Samsung TV API error");
            Err(err)
        }
        Err(_) => {
            warn!("Samsung TV API timeout after {:?}", duration);
            Err(ServerError::Timeout)
        }
    }
}
