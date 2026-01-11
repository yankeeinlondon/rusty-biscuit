//! HTTP client and network errors.

use thiserror::Error;

/// Errors from the HTTP client layer.
///
/// These errors represent network-level failures, HTTP status errors,
/// and connection issues that occur during request execution.
#[derive(Debug, Error)]
pub enum ClientError {
    /// HTTP request failed due to network or protocol error.
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// Server returned a non-success HTTP status code.
    #[error("HTTP {status}: {message}")]
    HttpStatus {
        /// The HTTP status code returned.
        status: u16,
        /// Error message from the response body.
        message: String,
    },

    /// Request exceeded the configured timeout.
    #[error("Request timeout after {duration_ms}ms")]
    Timeout {
        /// The timeout duration in milliseconds.
        duration_ms: u64,
    },

    /// Failed to establish connection to the server.
    #[error("Connection failed: {0}")]
    Connection(String),
}

impl ClientError {
    /// Returns `true` if this error is retryable.
    ///
    /// Timeout and connection errors are typically retryable,
    /// while HTTP status errors depend on the status code.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } | Self::Connection(_) => true,
            Self::HttpStatus { status, .. } => {
                // 5xx errors and 429 (rate limit) are retryable
                *status >= 500 || *status == 429
            }
            Self::Request(e) => e.is_timeout() || e.is_connect(),
        }
    }

    /// Returns the HTTP status code if this is an HTTP status error.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::HttpStatus { status, .. } => Some(*status),
            Self::Request(e) => e.status().map(|s| s.as_u16()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_is_retryable() {
        let err = ClientError::Timeout { duration_ms: 5000 };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_connection_is_retryable() {
        let err = ClientError::Connection("connection refused".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_500_is_retryable() {
        let err = ClientError::HttpStatus {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_429_is_retryable() {
        let err = ClientError::HttpStatus {
            status: 429,
            message: "Too Many Requests".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_400_not_retryable() {
        let err = ClientError::HttpStatus {
            status: 400,
            message: "Bad Request".to_string(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_status_code_extraction() {
        let err = ClientError::HttpStatus {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(err.status_code(), Some(404));

        let timeout = ClientError::Timeout { duration_ms: 1000 };
        assert_eq!(timeout.status_code(), None);
    }
}
