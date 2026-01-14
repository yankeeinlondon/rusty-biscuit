//! Authentication strategies for REST APIs.
//!
//! This module defines the supported authentication methods for API clients.
//! Each strategy specifies how credentials are applied to HTTP requests.
//! The actual credential source (environment variables) is configured on
//! the [`RestApi`](crate::RestApi) struct.
//!
//! ## Update Strategy
//!
//! When creating API variants with [`RestApi::variant()`](crate::RestApi::variant),
//! the [`UpdateStrategy`] enum controls how the authentication strategy is handled:
//!
//! - [`UpdateStrategy::NoChange`] - Keep the existing auth strategy
//! - [`UpdateStrategy::ChangeTo`] - Switch to a different auth strategy

use serde::{Deserialize, Serialize};

/// Authentication strategy for an API.
///
/// Defines how authentication credentials are applied to HTTP requests.
/// The credential sources (environment variables) are specified on the
/// [`RestApi`](crate::RestApi) struct via `env_auth` and `env_username` fields.
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
/// // Username from RestApi::env_username, password from RestApi::env_auth[0]
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
    /// Username is read from `RestApi::env_username` and password from
    /// the first element of `RestApi::env_auth` (i.e., `env_auth[0]`).
    Basic,
}

/// Strategy for updating authentication when creating API variants.
///
/// Used with the generated `variant()` method to control how the authentication
/// strategy is handled when creating a new API client instance with different
/// configuration.
///
/// ## Examples
///
/// Keep the existing auth strategy:
///
/// ```
/// use schematic_define::{AuthStrategy, UpdateStrategy};
///
/// let strategy = UpdateStrategy::NoChange;
/// let current = AuthStrategy::BearerToken { header: None };
///
/// // NoChange preserves the current strategy
/// let result = match strategy {
///     UpdateStrategy::NoChange => current.clone(),
///     UpdateStrategy::ChangeTo(new) => new,
/// };
/// assert_eq!(result, current);
/// ```
///
/// Switch to a different auth strategy:
///
/// ```
/// use schematic_define::{AuthStrategy, UpdateStrategy};
///
/// let strategy = UpdateStrategy::ChangeTo(AuthStrategy::ApiKey {
///     header: "X-API-Key".to_string(),
/// });
///
/// let result = match strategy {
///     UpdateStrategy::NoChange => AuthStrategy::None,
///     UpdateStrategy::ChangeTo(new) => new,
/// };
/// assert!(matches!(result, AuthStrategy::ApiKey { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStrategy {
    /// Keep the current authentication strategy unchanged.
    ///
    /// The variant will inherit the auth strategy from the source API instance.
    NoChange,

    /// Change to the specified authentication strategy.
    ///
    /// The variant will use the provided `AuthStrategy` instead of inheriting
    /// from the source API instance.
    ChangeTo(AuthStrategy),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_strategy_no_change_preserves_auth() {
        let current = AuthStrategy::BearerToken { header: None };
        let strategy = UpdateStrategy::NoChange;

        let result = match strategy {
            UpdateStrategy::NoChange => current.clone(),
            UpdateStrategy::ChangeTo(new) => new,
        };

        assert_eq!(result, current);
    }

    #[test]
    fn update_strategy_change_to_replaces_auth() {
        let current = AuthStrategy::BearerToken { header: None };
        let new_auth = AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
        };
        let strategy = UpdateStrategy::ChangeTo(new_auth.clone());

        let result = match strategy {
            UpdateStrategy::NoChange => current,
            UpdateStrategy::ChangeTo(new) => new,
        };

        assert_eq!(result, new_auth);
    }

    #[test]
    fn update_strategy_debug_impl() {
        let no_change = UpdateStrategy::NoChange;
        let change_to = UpdateStrategy::ChangeTo(AuthStrategy::None);

        assert!(format!("{:?}", no_change).contains("NoChange"));
        assert!(format!("{:?}", change_to).contains("ChangeTo"));
    }

    #[test]
    fn update_strategy_clone() {
        let original = UpdateStrategy::ChangeTo(AuthStrategy::Basic);
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn update_strategy_eq() {
        let a = UpdateStrategy::NoChange;
        let b = UpdateStrategy::NoChange;
        let c = UpdateStrategy::ChangeTo(AuthStrategy::None);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
