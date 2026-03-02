use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use homelab::{arcam::ArcamError, eversolo::EversoloError, sony_receiver::SonyError};
use serde::Serialize;
use std::fmt;
use utoipa::ToSchema;

/// Server error type that maps to HTTP status codes
#[derive(Debug)]
pub enum ServerError {
    /// Device not configured (missing env var) - legacy
    DeviceNotConfigured(&'static str),
    /// Device not found by name (404)
    DeviceNotFound(String),
    /// Device already exists (409)
    DeviceExists(String),
    /// Invalid device name (400)
    InvalidDeviceName(String),
    /// Invalid host (400)
    InvalidHost(String),
    /// Sony receiver error
    Sony(SonyError),
    /// Arcam amplifier error
    Arcam(ArcamError),
    /// Eversolo music streamer error
    Eversolo(EversoloError),
    /// Invalid volume level (must be 0-100)
    InvalidVolume(String),
    /// Request timeout
    Timeout,
    /// Configuration I/O error
    ConfigIo(String),
    /// Configuration parse error
    ConfigParse(String),
    /// Invalid parameter value (400)
    InvalidParameter(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotConfigured(device) => {
                write!(f, "{device} not configured")
            }
            Self::DeviceNotFound(name) => {
                write!(f, "Device not found: {name}")
            }
            Self::DeviceExists(name) => {
                write!(f, "Device already exists: {name}")
            }
            Self::InvalidDeviceName(name) => {
                write!(f, "Invalid device name: {name}")
            }
            Self::InvalidHost(msg) => write!(f, "Invalid host: {msg}"),
            Self::Sony(e) => write!(f, "Sony receiver error: {e}"),
            Self::Arcam(e) => write!(f, "Arcam amplifier error: {e}"),
            Self::Eversolo(e) => write!(f, "Eversolo error: {e}"),
            Self::InvalidVolume(msg) => write!(f, "Invalid volume: {msg}"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::ConfigIo(msg) => write!(f, "Configuration I/O error: {msg}"),
            Self::ConfigParse(msg) => write!(f, "Configuration parse error: {msg}"),
            Self::InvalidParameter(msg) => write!(f, "Invalid parameter: {msg}"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sony(e) => Some(e),
            Self::Arcam(e) => Some(e),
            Self::Eversolo(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SonyError> for ServerError {
    fn from(err: SonyError) -> Self {
        Self::Sony(err)
    }
}

impl From<ArcamError> for ServerError {
    fn from(err: ArcamError) -> Self {
        Self::Arcam(err)
    }
}

impl From<EversoloError> for ServerError {
    fn from(err: EversoloError) -> Self {
        Self::Eversolo(err)
    }
}

impl From<crate::config::ConfigError> for ServerError {
    fn from(err: crate::config::ConfigError) -> Self {
        match err {
            crate::config::ConfigError::Io(e) => Self::ConfigIo(e.to_string()),
            crate::config::ConfigError::Parse(e) => Self::ConfigParse(e.to_string()),
            crate::config::ConfigError::InvalidDeviceName(name) => Self::InvalidDeviceName(name),
            crate::config::ConfigError::InvalidHost(msg) => Self::InvalidHost(msg),
        }
    }
}

/// JSON error response body
#[derive(Serialize, ToSchema)]
pub(crate) struct ErrorResponse {
    /// Human-readable error message
    error: String,
    /// Machine-readable error code
    code: &'static str,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ServerError::DeviceNotConfigured(_) => (StatusCode::NOT_FOUND, "DEVICE_NOT_CONFIGURED"),
            ServerError::DeviceNotFound(_) => (StatusCode::NOT_FOUND, "DEVICE_NOT_FOUND"),
            ServerError::DeviceExists(_) => (StatusCode::CONFLICT, "DEVICE_EXISTS"),
            ServerError::InvalidDeviceName(_) => (StatusCode::BAD_REQUEST, "INVALID_DEVICE_NAME"),
            ServerError::InvalidHost(_) => (StatusCode::BAD_REQUEST, "INVALID_HOST"),
            ServerError::Sony(_) => (StatusCode::BAD_GATEWAY, "SONY_ERROR"),
            ServerError::Arcam(_) => (StatusCode::BAD_GATEWAY, "ARCAM_ERROR"),
            ServerError::Eversolo(_) => (StatusCode::BAD_GATEWAY, "EVERSOLO_ERROR"),
            ServerError::InvalidVolume(_) => (StatusCode::BAD_REQUEST, "INVALID_VOLUME"),
            ServerError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "TIMEOUT"),
            ServerError::ConfigIo(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_IO_ERROR"),
            ServerError::ConfigParse(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_PARSE_ERROR")
            }
            ServerError::InvalidParameter(_) => (StatusCode::BAD_REQUEST, "INVALID_PARAMETER"),
        };

        let body = ErrorResponse {
            error: self.to_string(),
            code,
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_not_configured_display() {
        let err = ServerError::DeviceNotConfigured("Sony Receiver");
        assert_eq!(err.to_string(), "Sony Receiver not configured");
    }

    #[test]
    fn test_device_not_found_display() {
        let err = ServerError::DeviceNotFound("living-room".to_string());
        assert_eq!(err.to_string(), "Device not found: living-room");
    }

    #[test]
    fn test_device_exists_display() {
        let err = ServerError::DeviceExists("office".to_string());
        assert_eq!(err.to_string(), "Device already exists: office");
    }

    #[test]
    fn test_invalid_device_name_display() {
        let err = ServerError::InvalidDeviceName("Bad Name!".to_string());
        assert_eq!(err.to_string(), "Invalid device name: Bad Name!");
    }

    #[test]
    fn test_invalid_host_display() {
        let err = ServerError::InvalidHost("empty".to_string());
        assert_eq!(err.to_string(), "Invalid host: empty");
    }

    #[test]
    fn test_invalid_volume_display() {
        let err = ServerError::InvalidVolume("level must be 0-100, got 150".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid volume: level must be 0-100, got 150"
        );
    }

    #[test]
    fn test_timeout_display() {
        let err = ServerError::Timeout;
        assert_eq!(err.to_string(), "Request timed out");
    }

    #[test]
    fn test_config_io_display() {
        let err = ServerError::ConfigIo("permission denied".to_string());
        assert_eq!(
            err.to_string(),
            "Configuration I/O error: permission denied"
        );
    }

    #[test]
    fn test_config_parse_display() {
        let err = ServerError::ConfigParse("invalid json".to_string());
        assert_eq!(err.to_string(), "Configuration parse error: invalid json");
    }

    // Test HTTP status code mappings
    #[test]
    fn test_status_codes() {
        use axum::response::IntoResponse;

        let cases: Vec<(ServerError, StatusCode)> = vec![
            (
                ServerError::DeviceNotConfigured("test"),
                StatusCode::NOT_FOUND,
            ),
            (
                ServerError::DeviceNotFound("test".to_string()),
                StatusCode::NOT_FOUND,
            ),
            (
                ServerError::DeviceExists("test".to_string()),
                StatusCode::CONFLICT,
            ),
            (
                ServerError::InvalidDeviceName("test".to_string()),
                StatusCode::BAD_REQUEST,
            ),
            (
                ServerError::InvalidHost("test".to_string()),
                StatusCode::BAD_REQUEST,
            ),
            (
                ServerError::InvalidVolume("test".to_string()),
                StatusCode::BAD_REQUEST,
            ),
            (ServerError::Timeout, StatusCode::GATEWAY_TIMEOUT),
            (
                ServerError::ConfigIo("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                ServerError::ConfigParse("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (error, expected_status) in cases {
            let response = error.into_response();
            assert_eq!(response.status(), expected_status);
        }
    }
}
