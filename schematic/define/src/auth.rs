//! Authentication strategies for REST APIs.
//!
//! This module defines the supported authentication methods for API clients.
//! Each strategy specifies how credentials are obtained (via environment
//! variables) and how they're applied to HTTP requests.

use serde::{Deserialize, Serialize};

/// Authentication strategy for an API.
///
/// Defines how authentication credentials are obtained and applied to
/// HTTP requests. All credentials are read from environment variables
/// to avoid hardcoding secrets in code.
///
/// ## Examples
///
/// No authentication:
///
/// ```
/// use schematic_define::AuthStrategy;
///
/// let auth = AuthStrategy::None;
/// ```
///
/// Bearer token (most common for modern APIs):
///
/// ```
/// use schematic_define::AuthStrategy;
///
/// let auth = AuthStrategy::BearerToken {
///     env_var: "OPENAI_API_KEY".to_string(),
///     header: None, // Uses default "Authorization" header
/// };
/// ```
///
/// API key in custom header:
///
/// ```
/// use schematic_define::AuthStrategy;
///
/// let auth = AuthStrategy::ApiKey {
///     env_var: "MY_API_KEY".to_string(),
///     header: "X-API-Key".to_string(),
/// };
/// ```
///
/// Basic authentication:
///
/// ```
/// use schematic_define::AuthStrategy;
///
/// let auth = AuthStrategy::Basic {
///     username_env: "SERVICE_USER".to_string(),
///     password_env: "SERVICE_PASSWORD".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum AuthStrategy {
    /// No authentication required.
    ///
    /// Use for public APIs that don't require any credentials.
    #[default]
    None,

    /// Bearer token in Authorization header.
    ///
    /// The most common authentication method for modern REST APIs.
    /// Generates: `Authorization: Bearer <token>`
    BearerToken {
        /// Environment variable name containing the token (e.g., "OPENAI_API_KEY").
        env_var: String,
        /// Optional header name override.
        ///
        /// Default is "Authorization". Some APIs use custom headers like
        /// "X-Auth-Token" for bearer tokens.
        header: Option<String>,
    },

    /// API key in a custom header.
    ///
    /// Common for APIs that use a simple key without the "Bearer" prefix.
    /// Generates: `<header>: <key>`
    ApiKey {
        /// Environment variable name containing the key.
        env_var: String,
        /// Header name (e.g., "X-API-Key", "Api-Key").
        header: String,
    },

    /// Basic authentication (username:password).
    ///
    /// Uses HTTP Basic Authentication with base64-encoded credentials.
    /// Generates: `Authorization: Basic <base64(username:password)>`
    Basic {
        /// Environment variable for username.
        username_env: String,
        /// Environment variable for password.
        password_env: String,
    },
}
