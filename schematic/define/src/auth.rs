//! Authentication strategies for REST APIs.
//!
//! This module defines the supported authentication methods for API clients.
//! Each strategy specifies how credentials are applied to HTTP requests.
//! The actual credential source (environment variables) is configured on
//! the [`RestApi`](crate::RestApi) struct.

use serde::{Deserialize, Serialize};

/// Authentication strategy for an API.
///
/// Defines how authentication credentials are applied to HTTP requests.
/// The credential sources (environment variables) are specified on the
/// [`RestApi`](crate::RestApi) struct via `env_auth`, `env_username`,
/// and `env_password` fields.
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
///     header: None, // Uses default "Authorization" header
/// };
/// // Credential env vars are set on RestApi::env_auth
/// ```
///
/// API key in custom header:
///
/// ```
/// use schematic_define::AuthStrategy;
///
/// let auth = AuthStrategy::ApiKey {
///     header: "X-API-Key".to_string(),
/// };
/// // Credential env vars are set on RestApi::env_auth
/// ```
///
/// Basic authentication:
///
/// ```
/// use schematic_define::AuthStrategy;
///
/// let auth = AuthStrategy::Basic;
/// // Credential env vars are set on RestApi::env_username and env_password
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    ///
    /// The token is read from environment variables specified in
    /// `RestApi::env_auth`. Multiple env vars can be specified as a fallback chain.
    BearerToken {
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
    ///
    /// The key is read from environment variables specified in
    /// `RestApi::env_auth`. Multiple env vars can be specified as a fallback chain.
    ApiKey {
        /// Header name (e.g., "X-API-Key", "Api-Key").
        header: String,
    },

    /// Basic authentication (username:password).
    ///
    /// Uses HTTP Basic Authentication with base64-encoded credentials.
    /// Generates: `Authorization: Basic <base64(username:password)>`
    ///
    /// Username and password are read from environment variables specified in
    /// `RestApi::env_username` and `RestApi::env_password`.
    Basic,
}
