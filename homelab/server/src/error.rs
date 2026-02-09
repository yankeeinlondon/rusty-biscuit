use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use homelab::{arcam::ArcamError, sony_receiver::SonyError};
use serde::Serialize;
use std::fmt;
use utoipa::ToSchema;

/// Server error type that maps to HTTP status codes
#[derive(Debug)]
pub enum ServerError {
    /// Device not configured (missing env var)
    DeviceNotConfigured(&'static str),
    /// Sony receiver error
    Sony(SonyError),
    /// Arcam amplifier error
    Arcam(ArcamError),
    /// Invalid volume level (must be 0-100)
    InvalidVolume(String),
    /// Request timeout
    Timeout,
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotConfigured(device) => {
                write!(f, "{} not configured", device)
            }
            Self::Sony(e) => write!(f, "Sony receiver error: {}", e),
            Self::Arcam(e) => write!(f, "Arcam amplifier error: {}", e),
            Self::InvalidVolume(msg) => write!(f, "Invalid volume: {}", msg),
            Self::Timeout => write!(f, "Request timed out"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sony(e) => Some(e),
            Self::Arcam(e) => Some(e),
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
            ServerError::Sony(_) => (StatusCode::BAD_GATEWAY, "SONY_ERROR"),
            ServerError::Arcam(_) => (StatusCode::BAD_GATEWAY, "ARCAM_ERROR"),
            ServerError::InvalidVolume(_) => (StatusCode::BAD_REQUEST, "INVALID_VOLUME"),
            ServerError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "TIMEOUT"),
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
}
