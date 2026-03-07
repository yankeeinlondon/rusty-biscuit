//! Error types for the Eversolo UC integration.

use homelab::{eversolo::EversoloError, wol::WolError};

#[derive(Debug, thiserror::Error)]
pub enum EversoloIntegrationError {
    #[error("Eversolo device error: {0}")]
    Eversolo(#[from] EversoloError),

    #[error("Wake-on-LAN error: {0}")]
    Wol(#[from] WolError),

    #[error("Invalid host: {0}")]
    InvalidHost(String),

    #[error("Operation timed out")]
    Timeout,

    #[error("Unknown entity: {0}")]
    UnknownEntity(String),

    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    #[error("Power on requires a configured MAC address")]
    MissingMacAddress,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Device returned unexpected status code: {0}")]
    UnexpectedStatus(i32),
}

impl EversoloIntegrationError {
    /// Map to a UC error code for `result` responses.
    pub fn uc_error_code(&self) -> u16 {
        match self {
            Self::UnknownEntity(_) => 404,
            Self::UnknownCommand(_) | Self::MissingMacAddress | Self::InvalidParameter(_) => 400,
            Self::Timeout => 503,
            Self::Eversolo(_) | Self::Wol(_) | Self::InvalidHost(_) | Self::UnexpectedStatus(_) => {
                503
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_uc_codes() {
        assert_eq!(
            EversoloIntegrationError::UnknownEntity("x".into()).uc_error_code(),
            404
        );
        assert_eq!(
            EversoloIntegrationError::UnknownCommand("x".into()).uc_error_code(),
            400
        );
        assert_eq!(
            EversoloIntegrationError::MissingMacAddress.uc_error_code(),
            400
        );
        assert_eq!(
            EversoloIntegrationError::InvalidParameter("x".into()).uc_error_code(),
            400
        );
        assert_eq!(EversoloIntegrationError::Timeout.uc_error_code(), 503);
    }
}
