//! Normalized authentication metadata for export.

use schematic_define::{AuthMethod, AuthPolicy, AuthStrategy, Endpoint, EndpointAuth, EnvAuthStrategy, OAuth2Config};

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
    /// OAuth2 with the definition's grant, URLs, and scopes.
    OAuth2 {
        /// The grant and its provider endpoints.
        config: OAuth2Config,
        /// Postman variable for the OAuth2 client id.
        client_id_var: String,
        /// Postman variable for the OAuth2 client secret.
        client_secret_var: String,
    },
    /// No authentication.
    None,
}

impl ExportAuth {
    /// An OAuth2 auth using the default client variables.
    fn oauth2(config: &OAuth2Config) -> Self {
        Self::OAuth2 {
            config: config.clone(),
            client_id_var: "oauth2ClientId".to_string(),
            client_secret_var: "oauth2ClientSecret".to_string(),
        }
    }
}

/// Maps a schematic `AuthStrategy` to an `ExportAuth`.
///
/// ## Examples
///
/// ```
/// use schematic_define::{AuthMethod, AuthPolicy, AuthStrategy, Endpoint, EndpointAuth, EnvAuthStrategy};
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
        AuthStrategy::Basic { .. } => ExportAuth::Basic {
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
        AuthStrategy::OAuth2(config) => ExportAuth::oauth2(config),
        // Handle any future variants by defaulting to no auth
        _ => ExportAuth::None,
    }
}

/// Maps one accepted method of an [`AuthPolicy`] the way [`map_auth`] maps
/// the matching strategy.
fn map_method(method: &AuthMethod) -> ExportAuth {
    match method {
        AuthMethod::BearerToken { header } => map_auth(&AuthStrategy::BearerToken { header: header.clone() }),
        AuthMethod::ApiKey { header, value_prefix } => map_auth(&AuthStrategy::ApiKey {
            header: header.clone(),
            value_prefix: value_prefix.clone(),
        }),
        AuthMethod::ApiKeyParam { name, location } => map_auth(&AuthStrategy::ApiKeyParam {
            name: name.clone(),
            location: *location,
        }),
        AuthMethod::Basic => ExportAuth::Basic {
            username_var: "username".to_string(),
            password_var: "password".to_string(),
        },
        AuthMethod::OAuth2(config) => ExportAuth::oauth2(config),
        _ => ExportAuth::None,
    }
}

fn map_fallback(fallback: &EnvAuthStrategy) -> ExportAuth {
    match fallback {
        EnvAuthStrategy::BearerToken { header } => map_method(&AuthMethod::BearerToken { header: header.clone() }),
        EnvAuthStrategy::ApiKey { header, value_prefix } => map_method(&AuthMethod::ApiKey {
            header: header.clone(),
            value_prefix: value_prefix.clone(),
        }),
        EnvAuthStrategy::ApiKeyParam { name, location } => map_method(&AuthMethod::ApiKeyParam {
            name: name.clone(),
            location: *location,
        }),
        EnvAuthStrategy::Basic => map_method(&AuthMethod::Basic),
        _ => ExportAuth::None,
    }
}

/// The distinct auths `policy` accepts as alternatives, in order: its
/// explicit methods, then its environment fallback.
pub fn policy_alternatives(policy: &AuthPolicy) -> Vec<ExportAuth> {
    let mut alternatives: Vec<ExportAuth> = Vec::new();
    let mapped = policy.explicit.iter().map(map_method).chain(policy.env_fallback.iter().map(map_fallback));
    for auth in mapped {
        if auth != ExportAuth::None && !alternatives.contains(&auth) {
            alternatives.push(auth);
        }
    }
    alternatives
}

/// The auth a request uses in place of its API's, or `None` when the
/// endpoint inherits.
///
/// A public endpoint is [`ExportAuth::None`]. A replacement policy is its
/// first alternative ([`policy_alternatives`]): one auth per request is all a
/// Postman request holds. When the replacement declares its own environment
/// mapping it names a credential distinct from the API's, so its variables
/// carry the endpoint id (`bearerToken_GetAdmin`) rather than sharing the
/// API's.
///
/// ## Examples
///
/// ```
/// use schematic_define::Endpoint;
/// use schematic_gen::export::auth::{ExportAuth, endpoint_auth_override};
///
/// let public = Endpoint::get("Health", "/health").describes("Health").returns_empty().public().build().unwrap();
/// assert_eq!(endpoint_auth_override(&public), Some(ExportAuth::None));
/// ```
pub fn endpoint_auth_override(endpoint: &Endpoint) -> Option<ExportAuth> {
    match endpoint.auth.as_ref()? {
        EndpointAuth::Public => Some(ExportAuth::None),
        EndpointAuth::Policy { policy, env_mapping } => {
            let auth = policy_alternatives(policy).into_iter().next()?;
            Some(if env_mapping.is_some() { with_variable_suffix(auth, &endpoint.id) } else { auth })
        }
        _ => None,
    }
}

pub(crate) fn with_variable_suffix(auth: ExportAuth, id: &str) -> ExportAuth {
    let suffixed = |name: String| format!("{name}_{id}");
    match auth {
        ExportAuth::Bearer { variable } => ExportAuth::Bearer { variable: suffixed(variable) },
        ExportAuth::ApiKey { header, variable, location } => ExportAuth::ApiKey { header, variable: suffixed(variable), location },
        ExportAuth::Basic { username_var, password_var } => ExportAuth::Basic {
            username_var: suffixed(username_var),
            password_var: suffixed(password_var),
        },
        ExportAuth::OAuth2 { config, client_id_var, client_secret_var } => ExportAuth::OAuth2 {
            config,
            client_id_var: suffixed(client_id_var),
            client_secret_var: suffixed(client_secret_var),
        },
        ExportAuth::None => ExportAuth::None,
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
            map_auth(&AuthStrategy::Basic {
                username_env: schematic_define::EnvList::single("USER"),
                password_env: schematic_define::EnvList::single("PASS"),
            }),
            ExportAuth::Basic {
                username_var: "username".to_string(),
                password_var: "password".to_string(),
            }
        );
    }

    #[test]
    fn policy_alternatives_count_an_env_fallback_naming_an_explicit_method_once() {
        let bearer = AuthPolicy::from_auth_strategy(&AuthStrategy::BearerToken { header: None });
        assert_eq!(policy_alternatives(&bearer), vec![ExportAuth::Bearer { variable: "bearerToken".to_string() }]);

        let two = AuthPolicy {
            explicit: vec![AuthMethod::Basic],
            env_fallback: Some(EnvAuthStrategy::BearerToken { header: None }),
        };
        assert_eq!(policy_alternatives(&two).len(), 2);
        assert!(matches!(policy_alternatives(&two)[0], ExportAuth::Basic { .. }));
    }
}
