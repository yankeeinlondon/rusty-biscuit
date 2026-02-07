//! Error types for the model-citizen library.
//!
//! Provides a unified error type for all model-citizen operations including
//! configuration loading, network requests, and model parsing.

use std::path::PathBuf;

/// Error type for model-citizen operations.
#[derive(Debug, thiserror::Error)]
pub enum ModelCitizenError {
    /// I/O operation failed.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Configuration loading or parsing failed.
    #[error("configuration error: {0}")]
    ConfigError(String),

    /// Network or HTTP request failed.
    #[error("network error: {0}")]
    NetworkError(String),

    /// GGUF or manifest parsing failed.
    #[error("parse error: {0}")]
    ParseError(String),

    /// Model not found at the specified location.
    #[error("model not found: {path}")]
    NotFound {
        /// The path or identifier that was not found.
        path: PathBuf,
    },
}

impl ModelCitizenError {
    /// Creates a new configuration error with the given message.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    /// Creates a new network error with the given message.
    pub fn network(msg: impl Into<String>) -> Self {
        Self::NetworkError(msg.into())
    }

    /// Creates a new parse error with the given message.
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::ParseError(msg.into())
    }

    /// Creates a new not found error for the given path.
    pub fn not_found(path: impl Into<PathBuf>) -> Self {
        Self::NotFound { path: path.into() }
    }
}

impl From<toml::de::Error> for ModelCitizenError {
    fn from(err: toml::de::Error) -> Self {
        Self::ConfigError(err.to_string())
    }
}

impl From<reqwest::Error> for ModelCitizenError {
    fn from(err: reqwest::Error) -> Self {
        Self::NetworkError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = ModelCitizenError::from(io_err);
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn config_error_display() {
        let err = ModelCitizenError::config("invalid toml");
        assert_eq!(err.to_string(), "configuration error: invalid toml");
    }

    #[test]
    fn network_error_display() {
        let err = ModelCitizenError::network("connection refused");
        assert_eq!(err.to_string(), "network error: connection refused");
    }

    #[test]
    fn parse_error_display() {
        let err = ModelCitizenError::parse("invalid GGUF magic");
        assert_eq!(err.to_string(), "parse error: invalid GGUF magic");
    }

    #[test]
    fn not_found_error_display() {
        let err = ModelCitizenError::not_found("/models/missing.gguf");
        assert_eq!(err.to_string(), "model not found: /models/missing.gguf");
    }

    #[test]
    fn error_implements_std_error() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<ModelCitizenError>();
    }

    #[test]
    fn error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ModelCitizenError>();
    }
}
