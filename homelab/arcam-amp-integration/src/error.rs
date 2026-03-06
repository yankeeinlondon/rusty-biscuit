//! Error types for the Arcam UC integration.

use homelab::arcam::ArcamError;

#[derive(Debug, thiserror::Error)]
pub enum ArcamIntegrationError {
    #[error("Arcam device error: {0}")]
    Arcam(#[from] ArcamError),

    #[error("Invalid host: {0}")]
    InvalidHost(String),

    #[error("Operation timed out")]
    Timeout,

    #[error("Unknown entity: {0}")]
    UnknownEntity(String),

    #[error("Unknown command: {0}")]
    UnknownCommand(String),
}

impl ArcamIntegrationError {
    /// Map to a UC error code for `result` responses.
    pub fn uc_error_code(&self) -> u16 {
        match self {
            Self::UnknownEntity(_) => 404,
            Self::UnknownCommand(_) => 400,
            Self::Timeout => 503,
            Self::Arcam(_) | Self::InvalidHost(_) => 503,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_uc_codes() {
        assert_eq!(ArcamIntegrationError::UnknownEntity("x".into()).uc_error_code(), 404);
        assert_eq!(ArcamIntegrationError::UnknownCommand("x".into()).uc_error_code(), 400);
        assert_eq!(ArcamIntegrationError::Timeout.uc_error_code(), 503);
        assert_eq!(ArcamIntegrationError::InvalidHost("x".into()).uc_error_code(), 503);
    }
}
