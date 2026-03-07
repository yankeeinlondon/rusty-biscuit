//! Error types for the Sony receiver UC integration.

use homelab::sony_receiver::SonyError;

#[derive(Debug, thiserror::Error)]
pub enum SonyIntegrationError {
    #[error("Sony receiver error: {0}")]
    Sony(#[from] SonyError),

    #[error("Invalid host: {0}")]
    InvalidHost(String),

    #[error("Operation timed out")]
    Timeout,

    #[error("Unknown entity: {0}")]
    UnknownEntity(String),

    #[error("Unknown command: {0}")]
    UnknownCommand(String),
}

impl SonyIntegrationError {
    /// Map to a UC error code for `result` responses.
    pub fn uc_error_code(&self) -> u16 {
        match self {
            Self::UnknownEntity(_) => 404,
            Self::UnknownCommand(_) => 400,
            Self::Timeout => 503,
            Self::Sony(_) | Self::InvalidHost(_) => 503,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_uc_codes() {
        assert_eq!(
            SonyIntegrationError::UnknownEntity("x".into()).uc_error_code(),
            404
        );
        assert_eq!(
            SonyIntegrationError::UnknownCommand("x".into()).uc_error_code(),
            400
        );
        assert_eq!(SonyIntegrationError::Timeout.uc_error_code(), 503);
        assert_eq!(
            SonyIntegrationError::InvalidHost("x".into()).uc_error_code(),
            503
        );
    }
}
