//! CRUD endpoints for device configuration management.
//!
//! These endpoints allow listing, creating, updating, and deleting
//! Sony receiver and Arcam amplifier configurations via REST.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::config::{is_valid_device_name, ArcamAmpService, SonyReceiverService};
use crate::error::{ErrorResponse, ServerError};
use crate::state::AppState;

/// Create all Sony receiver CRUD routes
pub fn sony_receiver_crud_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_sony_receivers, create_sony_receiver))
        .routes(routes!(update_sony_receiver, delete_sony_receiver))
}

/// Create all Arcam amplifier CRUD routes
pub fn arcam_amp_crud_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_arcam_amps, create_arcam_amp))
        .routes(routes!(update_arcam_amp, delete_arcam_amp))
}

// --- Request/Response DTOs ---

/// Request body for creating/updating a Sony receiver
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SonyReceiverRequest {
    /// Hostname or IP address
    pub host: String,
    /// Port number (default: 10000)
    #[serde(default = "default_sony_port")]
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
    /// Port number (default: 50000)
    #[serde(default = "default_arcam_port")]
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
pub(crate) async fn list_sony_receivers(
    State(state): State<AppState>,
) -> impl IntoResponse {
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

// --- Arcam Amplifier CRUD ---

#[utoipa::path(
    get,
    path = "/",
    tag = "arcam_amp",
    responses(
        (status = 200, description = "List of Arcam amplifiers", body = Vec<ArcamAmpResponse>),
    )
)]
pub(crate) async fn list_arcam_amps(
    State(state): State<AppState>,
) -> impl IntoResponse {
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
