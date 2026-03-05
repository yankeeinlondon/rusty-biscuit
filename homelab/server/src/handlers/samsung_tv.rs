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
        .routes(routes!(wake_by_name))
        .routes(routes!(get_app_status_by_name))
        .routes(routes!(close_app_by_name))
        .routes(routes!(install_app_by_name))
        .routes(routes!(launch_app_ws_by_name))
        .routes(routes!(list_installed_apps_by_name))
        .routes(routes!(get_art_status_by_name))
        .routes(routes!(set_art_status_by_name))
        .routes(routes!(get_art_current_by_name))
        .routes(routes!(get_art_list_by_name))
        .routes(routes!(select_art_by_name))
        .routes(routes!(get_art_brightness_by_name))
        .routes(routes!(set_art_brightness_by_name))
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

#[derive(Deserialize, ToSchema)]
pub struct WsAppLaunchRequest {
    /// App ID to launch
    pub app_id: String,
    /// Optional deep-link metadata tag (URL)
    #[serde(default)]
    pub meta_tag: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ArtStatusRequest {
    /// Enable (true) or disable (false) Art Mode
    pub on: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct ArtSelectRequest {
    /// Content ID of the artwork to display
    pub content_id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ArtBrightnessRequest {
    /// Brightness level (0-100)
    pub level: u8,
}

// --- Existing Handlers ---

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

#[utoipa::path(
    put,
    path = "/{name}/wake",
    tag = "samsung_tv",
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
        let tvs = state.samsung_tvs.read().await;
        let service = tvs
            .get(&name)
            .ok_or_else(|| ServerError::DeviceNotFound(name.clone()))?;
        service
            .mac_address
            .clone()
            .ok_or_else(|| ServerError::InvalidParameter("no MAC address configured".to_string()))?
    };

    // Standard WoL port 9
    homelab::wol::send_magic_packet(&mac, "255.255.255.255", 9)
        .map_err(|e| ServerError::InvalidParameter(e.to_string()))?;

    Ok(Json(serde_json::json!({"wol_sent": true, "mac": mac})))
}

// --- App Lifecycle Handlers ---

#[utoipa::path(
    get,
    path = "/{name}/app/{app_id}",
    tag = "samsung_tv",
    params(
        ("name" = String, Path, description = "Device name"),
        ("app_id" = String, Path, description = "Application ID"),
    ),
    responses(
        (status = 200, description = "App status"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_app_status_by_name(
    State(state): State<AppState>,
    Path((name, app_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    let status = with_timeout(state.request_timeout, tv.get_app_status(&app_id)).await?;
    Ok(Json(serde_json::to_value(status).unwrap()))
}

#[utoipa::path(
    delete,
    path = "/{name}/app/{app_id}",
    tag = "samsung_tv",
    params(
        ("name" = String, Path, description = "Device name"),
        ("app_id" = String, Path, description = "Application ID"),
    ),
    responses(
        (status = 200, description = "App closed"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn close_app_by_name(
    State(state): State<AppState>,
    Path((name, app_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.close_app(&app_id)).await?;
    Ok(Json(serde_json::json!({"closed": true, "app_id": app_id})))
}

#[utoipa::path(
    put,
    path = "/{name}/app/{app_id}/install",
    tag = "samsung_tv",
    params(
        ("name" = String, Path, description = "Device name"),
        ("app_id" = String, Path, description = "Application ID"),
    ),
    responses(
        (status = 200, description = "App install initiated"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn install_app_by_name(
    State(state): State<AppState>,
    Path((name, app_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.install_app(&app_id)).await?;
    Ok(Json(serde_json::json!({"install_initiated": true, "app_id": app_id})))
}

#[utoipa::path(
    post,
    path = "/{name}/app/ws",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    request_body = WsAppLaunchRequest,
    responses(
        (status = 200, description = "App launched via WebSocket"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn launch_app_ws_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<WsAppLaunchRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(
        state.request_timeout,
        tv.launch_app_ws(&req.app_id, req.meta_tag.as_deref()),
    )
    .await?;
    Ok(Json(serde_json::json!({"launched": true, "app_id": req.app_id})))
}

#[utoipa::path(
    get,
    path = "/{name}/app/list",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Installed apps request sent"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn list_installed_apps_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.request_installed_apps()).await?;
    Ok(Json(serde_json::json!({"request_sent": true})))
}

// --- Art Mode Handlers ---

#[utoipa::path(
    get,
    path = "/{name}/art/status",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Art Mode status request sent"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_art_status_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.get_art_mode_status()).await?;
    Ok(Json(serde_json::json!({"request_sent": true})))
}

#[utoipa::path(
    put,
    path = "/{name}/art/status",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    request_body = ArtStatusRequest,
    responses(
        (status = 200, description = "Art Mode status updated"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_art_status_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<ArtStatusRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.set_art_mode(req.on)).await?;
    Ok(Json(serde_json::json!({"art_mode": req.on})))
}

#[utoipa::path(
    get,
    path = "/{name}/art/current",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Current artwork request sent"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_art_current_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.get_current_artwork()).await?;
    Ok(Json(serde_json::json!({"request_sent": true})))
}

#[utoipa::path(
    get,
    path = "/{name}/art/list",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Artwork list request sent"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_art_list_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.get_artwork_list()).await?;
    Ok(Json(serde_json::json!({"request_sent": true})))
}

#[utoipa::path(
    put,
    path = "/{name}/art/select",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    request_body = ArtSelectRequest,
    responses(
        (status = 200, description = "Artwork selected"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn select_art_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<ArtSelectRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.select_artwork(&req.content_id)).await?;
    Ok(Json(serde_json::json!({"selected": true, "content_id": req.content_id})))
}

#[utoipa::path(
    get,
    path = "/{name}/art/brightness",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    responses(
        (status = 200, description = "Brightness request sent"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn get_art_brightness_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.get_art_brightness()).await?;
    Ok(Json(serde_json::json!({"request_sent": true})))
}

#[utoipa::path(
    put,
    path = "/{name}/art/brightness",
    tag = "samsung_tv",
    params(("name" = String, Path, description = "Device name")),
    request_body = ArtBrightnessRequest,
    responses(
        (status = 200, description = "Brightness updated"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 502, description = "Samsung TV API error", body = ErrorResponse),
        (status = 504, description = "Request timeout", body = ErrorResponse),
    )
)]
pub(crate) async fn set_art_brightness_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<ArtBrightnessRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let tv = create_samsung_by_name(&state, &name).await?;
    with_timeout(state.request_timeout, tv.set_art_brightness(req.level)).await?;
    Ok(Json(serde_json::json!({"brightness": req.level})))
}

// --- Helpers ---

async fn create_samsung_by_name(
    state: &AppState,
    name: &str,
) -> Result<SamsungTv, ServerError> {
    let (host, rest_port, ws_port, use_https) = state
        .get_samsung_tv(name)
        .await
        .ok_or_else(|| ServerError::DeviceNotFound(name.to_string()))?;
    Ok(SamsungTv::with_https(host, rest_port, ws_port, use_https))
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
