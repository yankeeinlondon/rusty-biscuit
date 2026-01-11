//! Endpoint and API configuration errors.

use thiserror::Error;

use crate::endpoint_id::EndpointIdError;

/// Errors in API or endpoint configuration.
///
/// These errors occur during API setup, typically indicating
/// programmer errors or invalid configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// URL parsing failed.
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// A required configuration field is missing.
    #[error("Missing required field: {field}")]
    MissingField {
        /// The name of the missing field.
        field: &'static str,
    },

    /// Endpoint ID validation failed.
    #[error("Invalid endpoint ID: {0}")]
    InvalidEndpointId(#[from] EndpointIdError),

    /// Path template contains invalid parameter syntax.
    #[error("Invalid path template: {message}")]
    InvalidPathTemplate {
        /// Description of the path template error.
        message: String,
    },

    /// Duplicate endpoint ID detected.
    #[error("Duplicate endpoint ID: {id}")]
    DuplicateEndpoint {
        /// The duplicate endpoint ID.
        id: String,
    },
}

impl ConfigError {
    /// Creates a missing field error.
    pub fn missing_field(field: &'static str) -> Self {
        Self::MissingField { field }
    }

    /// Creates an invalid path template error.
    pub fn invalid_path(message: impl Into<String>) -> Self {
        Self::InvalidPathTemplate {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_field() {
        let err = ConfigError::missing_field("base_url");
        assert_eq!(err.to_string(), "Missing required field: base_url");
    }

    #[test]
    fn test_invalid_url() {
        let url_err = url::Url::parse("not-a-url").unwrap_err();
        let err = ConfigError::InvalidUrl(url_err);
        assert!(err.to_string().contains("Invalid URL"));
    }

    #[test]
    fn test_invalid_path_template() {
        let err = ConfigError::invalid_path("unclosed brace in /users/{id");
        assert!(err.to_string().contains("Invalid path template"));
    }

    #[test]
    fn test_duplicate_endpoint() {
        let err = ConfigError::DuplicateEndpoint {
            id: "get_users".to_string(),
        };
        assert_eq!(err.to_string(), "Duplicate endpoint ID: get_users");
    }
}
