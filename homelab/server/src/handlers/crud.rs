//! CRUD endpoints for device configuration management.
//!
//! These endpoints allow listing, creating, updating, and deleting
//! Sony receiver and Arcam amplifier configurations via REST.

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::config::{ArcamAmpService, EversoloService, SonyReceiverService, is_valid_device_name};
use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Sony receiver CRUD routes
pub fn sony_receiver_crud_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_sony_receivers, create_sony_receiver))
        .routes(routes!(update_sony_receiver, delete_sony_receiver))
        .routes(routes!(rename_sony_receiver))
}

/// Create all Arcam amplifier CRUD routes
pub fn arcam_amp_crud_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_arcam_amps, create_arcam_amp))
        .routes(routes!(update_arcam_amp, delete_arcam_amp))
        .routes(routes!(rename_arcam_amp))
}

// --- Request/Response DTOs ---

/// Request body for creating/updating a Sony receiver
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SonyReceiverRequest {
    /// Hostname or IP address
    pub host: String,
    /// Port number
    #[serde(default = "default_sony_port")]
    #[schema(default = 10000)]
    pub port: u16,
}

fn default_sony_port() -> u16 {
    10000
}

/// Request body for creating/updating an Arcam amplifier
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ArcamAmpRequest {
    /// Hostname or IP address
    pub host: String,
    /// Port number
    #[serde(default = "default_arcam_port")]
    #[schema(default = 50000)]
    pub port: u16,
}

fn default_arcam_port() -> u16 {
    50000
}

/// Response for a Sony receiver
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SonyReceiverResponse {
    /// Device name
    pub name: String,
    /// Hostname or IP address
    pub host: String,
    /// Port number
    pub port: u16,
}

/// Response for an Arcam amplifier
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ArcamAmpResponse {
    /// Device name
    pub name: String,
    /// Hostname or IP address
    pub host: String,
    /// Port number
    pub port: u16,
}

// --- Sony Receiver CRUD ---

#[utoipa::path(
    get,
    path = "/",
    tag = "sony_receiver",
    responses(
        (status = 200, description = "List of Sony receivers", body = Vec<SonyReceiverResponse>),
    )
)]
pub(crate) async fn list_sony_receivers(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.read().await;
    let receivers: Vec<SonyReceiverResponse> = config
        .sony_receivers
        .iter()
        .map(|(name, service)| SonyReceiverResponse {
            name: name.clone(),
            host: service.host.clone(),
            port: service.port,
        })
        .collect();

    Json(receivers)
}

#[utoipa::path(
    post,
    path = "/",
    tag = "sony_receiver",
    request_body = inline(CreateDeviceRequest),
    responses(
        (status = 201, description = "Sony receiver created", body = SonyReceiverResponse),
        (status = 400, description = "Invalid device name or host", body = ErrorResponse),
        (status = 409, description = "Device already exists", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn create_sony_receiver(
    State(state): State<AppState>,
    Json(req): Json<CreateDeviceRequest>,
) -> Result<impl IntoResponse, ServerError> {
    // Validate device name
    if !is_valid_device_name(&req.name) {
        return Err(ServerError::InvalidDeviceName(req.name));
    }

    // Check if device already exists
    {
        let config = state.config.read().await;
        if config.sony_receivers.contains_key(&req.name) {
            return Err(ServerError::DeviceExists(req.name));
        }
    }

    // Create service config
    let service = SonyReceiverService {
        host: req.host.clone(),
        port: req.port.unwrap_or(10000),
    };

    // Add to state (this also saves to disk)
    state.add_sony(req.name.clone(), service.clone()).await?;

    Ok((
        StatusCode::CREATED,
        Json(SonyReceiverResponse {
            name: req.name,
            host: service.host,
            port: service.port,
        }),
    ))
}

#[utoipa::path(
    put,
    path = "/{name}",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = SonyReceiverRequest,
    responses(
        (status = 200, description = "Sony receiver updated", body = SonyReceiverResponse),
        (status = 400, description = "Invalid host", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn update_sony_receiver(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SonyReceiverRequest>,
) -> Result<impl IntoResponse, ServerError> {
    // Check if device exists
    {
        let config = state.config.read().await;
        if !config.sony_receivers.contains_key(&name) {
            return Err(ServerError::DeviceNotFound(name));
        }
    }

    // Create updated service config
    let service = SonyReceiverService {
        host: req.host.clone(),
        port: req.port,
    };

    // Update in state (this also saves to disk)
    state.add_sony(name.clone(), service.clone()).await?;

    Ok(Json(SonyReceiverResponse {
        name,
        host: service.host,
        port: service.port,
    }))
}

#[utoipa::path(
    delete,
    path = "/{name}",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 204, description = "Sony receiver deleted"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn delete_sony_receiver(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let removed = state.remove_sony(&name).await?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServerError::DeviceNotFound(name))
    }
}

#[utoipa::path(
    patch,
    path = "/{name}",
    tag = "sony_receiver",
    params(
        ("name" = String, Path, description = "Current device name")
    ),
    request_body(content = String, description = "New device name (plain text)", content_type = "text/plain"),
    responses(
        (status = 200, description = "Sony receiver renamed", body = SonyReceiverResponse),
        (status = 400, description = "Invalid new device name", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 409, description = "New device name already exists", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn rename_sony_receiver(
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, ServerError> {
    // Parse new name from plain text body
    let new_name = String::from_utf8_lossy(&body).trim().to_string();

    // Validate new device name
    if !crate::config::is_valid_device_name(&new_name) {
        return Err(ServerError::InvalidDeviceName(new_name));
    }

    // Check if new name already exists
    {
        let config = state.config.read().await;
        if config.sony_receivers.contains_key(&new_name) {
            return Err(ServerError::DeviceExists(new_name));
        }
    }

    // Check if old device exists
    let service = {
        let config = state.config.read().await;
        match config.sony_receivers.get(&name) {
            Some(s) => s.clone(),
            None => return Err(ServerError::DeviceNotFound(name)),
        }
    };

    // Rename in state (this also saves to disk)
    state.rename_sony(&name, new_name.clone()).await?;

    Ok(Json(SonyReceiverResponse {
        name: new_name,
        host: service.host,
        port: service.port,
    }))
}

// --- Arcam Amplifier CRUD ---

#[utoipa::path(
    get,
    path = "/",
    tag = "arcam_amp",
    responses(
        (status = 200, description = "List of Arcam amplifiers", body = Vec<ArcamAmpResponse>),
    )
)]
pub(crate) async fn list_arcam_amps(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.read().await;
    let amps: Vec<ArcamAmpResponse> = config
        .arcam_amps
        .iter()
        .map(|(name, service)| ArcamAmpResponse {
            name: name.clone(),
            host: service.host.clone(),
            port: service.port,
        })
        .collect();

    Json(amps)
}

#[utoipa::path(
    post,
    path = "/",
    tag = "arcam_amp",
    request_body = inline(CreateDeviceRequest),
    responses(
        (status = 201, description = "Arcam amplifier created", body = ArcamAmpResponse),
        (status = 400, description = "Invalid device name or host", body = ErrorResponse),
        (status = 409, description = "Device already exists", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn create_arcam_amp(
    State(state): State<AppState>,
    Json(req): Json<CreateDeviceRequest>,
) -> Result<impl IntoResponse, ServerError> {
    // Validate device name
    if !is_valid_device_name(&req.name) {
        return Err(ServerError::InvalidDeviceName(req.name));
    }

    // Check if device already exists
    {
        let config = state.config.read().await;
        if config.arcam_amps.contains_key(&req.name) {
            return Err(ServerError::DeviceExists(req.name));
        }
    }

    // Validate host is not empty
    if req.host.is_empty() {
        return Err(ServerError::InvalidHost("empty host".to_string()));
    }

    // Create service config
    let service = ArcamAmpService {
        host: req.host.clone(),
        port: req.port.unwrap_or(50000),
    };

    // Add to state (this also saves to disk)
    state.add_arcam(req.name.clone(), service.clone()).await?;

    Ok((
        StatusCode::CREATED,
        Json(ArcamAmpResponse {
            name: req.name,
            host: service.host,
            port: service.port,
        }),
    ))
}

#[utoipa::path(
    put,
    path = "/{name}",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = ArcamAmpRequest,
    responses(
        (status = 200, description = "Arcam amplifier updated", body = ArcamAmpResponse),
        (status = 400, description = "Invalid host", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn update_arcam_amp(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<ArcamAmpRequest>,
) -> Result<impl IntoResponse, ServerError> {
    // Check if device exists
    {
        let config = state.config.read().await;
        if !config.arcam_amps.contains_key(&name) {
            return Err(ServerError::DeviceNotFound(name));
        }
    }

    // Validate host is not empty
    if req.host.is_empty() {
        return Err(ServerError::InvalidHost("empty host".to_string()));
    }

    // Create updated service config
    let service = ArcamAmpService {
        host: req.host.clone(),
        port: req.port,
    };

    // Update in state (this also saves to disk)
    state.add_arcam(name.clone(), service.clone()).await?;

    Ok(Json(ArcamAmpResponse {
        name,
        host: service.host,
        port: service.port,
    }))
}

#[utoipa::path(
    delete,
    path = "/{name}",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 204, description = "Arcam amplifier deleted"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn delete_arcam_amp(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let removed = state.remove_arcam(&name).await?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServerError::DeviceNotFound(name))
    }
}

#[utoipa::path(
    patch,
    path = "/{name}",
    tag = "arcam_amp",
    params(
        ("name" = String, Path, description = "Current device name")
    ),
    request_body(content = String, description = "New device name (plain text)", content_type = "text/plain"),
    responses(
        (status = 200, description = "Arcam amplifier renamed", body = ArcamAmpResponse),
        (status = 400, description = "Invalid new device name", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 409, description = "New device name already exists", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn rename_arcam_amp(
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, ServerError> {
    // Parse new name from plain text body
    let new_name = String::from_utf8_lossy(&body).trim().to_string();

    // Validate new device name
    if !crate::config::is_valid_device_name(&new_name) {
        return Err(ServerError::InvalidDeviceName(new_name));
    }

    // Check if new name already exists
    {
        let config = state.config.read().await;
        if config.arcam_amps.contains_key(&new_name) {
            return Err(ServerError::DeviceExists(new_name));
        }
    }

    // Check if old device exists
    let service = {
        let config = state.config.read().await;
        match config.arcam_amps.get(&name) {
            Some(s) => s.clone(),
            None => return Err(ServerError::DeviceNotFound(name)),
        }
    };

    // Rename in state (this also saves to disk)
    state.rename_arcam(&name, new_name.clone()).await?;

    Ok(Json(ArcamAmpResponse {
        name: new_name,
        host: service.host,
        port: service.port,
    }))
}

/// Request body for creating a new device
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDeviceRequest {
    /// Device name (lowercase alphanumeric with underscores and hyphens)
    pub name: String,
    /// Hostname or IP address
    pub host: String,
    /// Port number (optional, defaults to device-specific port)
    pub port: Option<u16>,
}

// --- Eversolo CRUD ---

/// Request body for creating/updating an Eversolo device
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EversoloRequest {
    /// Hostname or IP address
    pub host: String,
    /// Port number
    #[serde(default = "default_eversolo_port")]
    #[schema(default = 9529)]
    pub port: u16,
}

fn default_eversolo_port() -> u16 {
    9529
}

/// Response for an Eversolo device
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EversoloResponse {
    /// Device name
    pub name: String,
    /// Hostname or IP address
    pub host: String,
    /// Port number
    pub port: u16,
}

/// Create all Eversolo device CRUD routes
pub fn eversolo_crud_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_eversolo_devices, create_eversolo_device))
        .routes(routes!(update_eversolo_device, delete_eversolo_device))
        .routes(routes!(rename_eversolo_device))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "eversolo",
    responses(
        (status = 200, description = "List of Eversolo devices", body = Vec<EversoloResponse>),
    )
)]
pub(crate) async fn list_eversolo_devices(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.read().await;
    let devices: Vec<EversoloResponse> = config
        .eversolo_devices
        .iter()
        .map(|(name, service)| EversoloResponse {
            name: name.clone(),
            host: service.host.clone(),
            port: service.port,
        })
        .collect();

    Json(devices)
}

#[utoipa::path(
    post,
    path = "/",
    tag = "eversolo",
    request_body = inline(CreateDeviceRequest),
    responses(
        (status = 201, description = "Eversolo device created", body = EversoloResponse),
        (status = 400, description = "Invalid device name or host", body = ErrorResponse),
        (status = 409, description = "Device already exists", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn create_eversolo_device(
    State(state): State<AppState>,
    Json(req): Json<CreateDeviceRequest>,
) -> Result<impl IntoResponse, ServerError> {
    if !is_valid_device_name(&req.name) {
        return Err(ServerError::InvalidDeviceName(req.name));
    }

    {
        let config = state.config.read().await;
        if config.eversolo_devices.contains_key(&req.name) {
            return Err(ServerError::DeviceExists(req.name));
        }
    }

    if req.host.is_empty() {
        return Err(ServerError::InvalidHost("empty host".to_string()));
    }

    let service = EversoloService {
        host: req.host.clone(),
        port: req.port.unwrap_or(9529),
    };

    state
        .add_eversolo(req.name.clone(), service.clone())
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(EversoloResponse {
            name: req.name,
            host: service.host,
            port: service.port,
        }),
    ))
}

#[utoipa::path(
    put,
    path = "/{name}",
    tag = "eversolo",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    request_body = EversoloRequest,
    responses(
        (status = 200, description = "Eversolo device updated", body = EversoloResponse),
        (status = 400, description = "Invalid host", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn update_eversolo_device(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<EversoloRequest>,
) -> Result<impl IntoResponse, ServerError> {
    {
        let config = state.config.read().await;
        if !config.eversolo_devices.contains_key(&name) {
            return Err(ServerError::DeviceNotFound(name));
        }
    }

    if req.host.is_empty() {
        return Err(ServerError::InvalidHost("empty host".to_string()));
    }

    let service = EversoloService {
        host: req.host.clone(),
        port: req.port,
    };

    state.add_eversolo(name.clone(), service.clone()).await?;

    Ok(Json(EversoloResponse {
        name,
        host: service.host,
        port: service.port,
    }))
}

#[utoipa::path(
    delete,
    path = "/{name}",
    tag = "eversolo",
    params(
        ("name" = String, Path, description = "Device name")
    ),
    responses(
        (status = 204, description = "Eversolo device deleted"),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn delete_eversolo_device(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let removed = state.remove_eversolo(&name).await?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServerError::DeviceNotFound(name))
    }
}

#[utoipa::path(
    patch,
    path = "/{name}",
    tag = "eversolo",
    params(
        ("name" = String, Path, description = "Current device name")
    ),
    request_body(content = String, description = "New device name (plain text)", content_type = "text/plain"),
    responses(
        (status = 200, description = "Eversolo device renamed", body = EversoloResponse),
        (status = 400, description = "Invalid new device name", body = ErrorResponse),
        (status = 404, description = "Device not found", body = ErrorResponse),
        (status = 409, description = "New device name already exists", body = ErrorResponse),
        (status = 500, description = "Configuration error", body = ErrorResponse),
    )
)]
pub(crate) async fn rename_eversolo_device(
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, ServerError> {
    let new_name = String::from_utf8_lossy(&body).trim().to_string();

    if !crate::config::is_valid_device_name(&new_name) {
        return Err(ServerError::InvalidDeviceName(new_name));
    }

    {
        let config = state.config.read().await;
        if config.eversolo_devices.contains_key(&new_name) {
            return Err(ServerError::DeviceExists(new_name));
        }
    }

    let service = {
        let config = state.config.read().await;
        match config.eversolo_devices.get(&name) {
            Some(s) => s.clone(),
            None => return Err(ServerError::DeviceNotFound(name)),
        }
    };

    state.rename_eversolo(&name, new_name.clone()).await?;

    Ok(Json(EversoloResponse {
        name: new_name,
        host: service.host,
        port: service.port,
    }))
}
