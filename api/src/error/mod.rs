//! Layered error types for the API crate.
//!
//! The error hierarchy is structured for actionable diagnostics:
//! - [`ApiError`] - Top-level error type for all API operations
//! - [`ClientError`] - HTTP client and network errors
//! - [`ValidationError`] - Response parsing and validation errors
//! - [`AuthError`] - Authentication and authorization errors
//! - [`ConfigError`] - Endpoint and API configuration errors

mod api_error;
mod auth_error;
mod client_error;
mod config_error;
mod validation_error;

pub use api_error::ApiError;
pub use auth_error::AuthError;
pub use client_error::ClientError;
pub use config_error::ConfigError;
pub use validation_error::ValidationError;
