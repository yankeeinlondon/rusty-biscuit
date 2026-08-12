//! Normalized authentication metadata for export.

use schematic_define::AuthStrategy;

/// Location where an API key is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyLocation {
    /// API key sent in an HTTP header.
    Header,
    /// API key sent as a query parameter.
    Query,
    /// API key sent in a cookie.
    Cookie,
}

/// Normalized authentication for export formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportAuth {
    /// Bearer token authentication.
    Bearer {
        /// Postman variable name (e.g., "bearerToken").
        variable: String,
    },
    /// API key authentication.
    ApiKey {
        /// Parameter / header name (e.g., "X-API-Key").
        header: String,
        /// Postman variable name (e.g., "apiKey").
        variable: String,
        /// Where the key is sent (header, query, cookie).
        location: ApiKeyLocation,
    },
    /// HTTP Basic authentication.
    Basic {
        /// Postman variable for username.
        username_var: String,
        /// Postman variable for password.
        password_var: String,
    },
    /// No authentication.
    None,
}

/// Maps a schematic `AuthStrategy` to an `ExportAuth`.
///
/// ## Examples
///
/// ```
/// use schematic_define::AuthStrategy;
/// use schematic_gen::export::auth::{ExportAuth, map_auth};
///
/// let auth = map_auth(&AuthStrategy::BearerToken { header: None });
/// assert_eq!(auth, ExportAuth::Bearer { variable: "bearerToken".to_string() });
///
/// let auth = map_auth(&AuthStrategy::None);
/// assert_eq!(auth, ExportAuth::None);
/// ```
pub fn map_auth(strategy: &AuthStrategy) -> ExportAuth {
    match strategy {
        AuthStrategy::None => ExportAuth::None,
        AuthStrategy::BearerToken { header } => {
            if header.is_some() {
                // Custom header means it's really an API key style
                ExportAuth::ApiKey {
                    header: header.as_deref().unwrap_or("Authorization").to_string(),
                    variable: "apiKey".to_string(),
                    location: ApiKeyLocation::Header,
                }
            } else {
                ExportAuth::Bearer {
                    variable: "bearerToken".to_string(),
                }
            }
        }
        AuthStrategy::ApiKey { header, .. } => ExportAuth::ApiKey {
            header: header.clone(),
            variable: "apiKey".to_string(),
            location: ApiKeyLocation::Header,
        },
        AuthStrategy::Basic => ExportAuth::Basic {
            username_var: "username".to_string(),
            password_var: "password".to_string(),
        },
        AuthStrategy::ApiKeyParam { name, location } => ExportAuth::ApiKey {
            header: name.clone(),
            variable: "apiKey".to_string(),
            location: match location {
                schematic_define::auth::ApiKeyLocation::Query => ApiKeyLocation::Query,
                schematic_define::auth::ApiKeyLocation::Cookie => ApiKeyLocation::Cookie,
                _ => ApiKeyLocation::Header,
            },
        },
        AuthStrategy::OAuth2(_) => ExportAuth::Bearer {
            variable: "bearerToken".to_string(),
        },
        // Handle any future variants by defaulting to no auth
        _ => ExportAuth::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_auth_none() {
        assert_eq!(map_auth(&AuthStrategy::None), ExportAuth::None);
    }

    #[test]
    fn map_auth_bearer_default_header() {
        assert_eq!(
            map_auth(&AuthStrategy::BearerToken { header: None }),
            ExportAuth::Bearer {
                variable: "bearerToken".to_string()
            }
        );
    }

    #[test]
    fn map_auth_bearer_custom_header() {
        assert_eq!(
            map_auth(&AuthStrategy::BearerToken {
                header: Some("X-Auth-Token".to_string())
            }),
            ExportAuth::ApiKey {
                header: "X-Auth-Token".to_string(),
                variable: "apiKey".to_string(),
                location: ApiKeyLocation::Header,
            }
        );
    }

    #[test]
    fn map_auth_api_key() {
        assert_eq!(
            map_auth(&AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
                value_prefix: None,
            }),
            ExportAuth::ApiKey {
                header: "X-API-Key".to_string(),
                variable: "apiKey".to_string(),
                location: ApiKeyLocation::Header,
            }
        );
    }

    #[test]
    fn map_auth_api_key_param_query() {
        assert_eq!(
            map_auth(&AuthStrategy::ApiKeyParam {
                name: "api_key".to_string(),
                location: schematic_define::auth::ApiKeyLocation::Query,
            }),
            ExportAuth::ApiKey {
                header: "api_key".to_string(),
                variable: "apiKey".to_string(),
                location: ApiKeyLocation::Query,
            }
        );
    }

    #[test]
    fn map_auth_api_key_param_cookie() {
        assert_eq!(
            map_auth(&AuthStrategy::ApiKeyParam {
                name: "api_key".to_string(),
                location: schematic_define::auth::ApiKeyLocation::Cookie,
            }),
            ExportAuth::ApiKey {
                header: "api_key".to_string(),
                variable: "apiKey".to_string(),
                location: ApiKeyLocation::Cookie,
            }
        );
    }

    #[test]
    fn map_auth_basic() {
        assert_eq!(
            map_auth(&AuthStrategy::Basic),
            ExportAuth::Basic {
                username_var: "username".to_string(),
                password_var: "password".to_string(),
            }
        );
    }
}
