//! Authentication strategies for REST APIs.
//!
//! This module defines the supported authentication methods for API clients.
//! Each strategy specifies how credentials are applied to HTTP requests.
//! Token and key credential sources (environment variables) are configured
//! on the [`RestApi`](crate::RestApi) struct; [`AuthStrategy::Basic`] carries
//! its own username and password lists.
//!
//! ## Update Strategy
//!
//! [`UpdateStrategy`] describes how an API variant's authentication strategy
//! is handled:
//!
//! - [`UpdateStrategy::NoChange`] - Keep the existing auth strategy
//! - [`UpdateStrategy::ChangeTo`] - Switch to a different auth strategy
//!
//! Generated clients no longer have a `variant()` method, so no generated code
//! reads it; a client is reconfigured through its chained settings.

use serde::{Deserialize, Serialize};

use crate::headers::{EnvList, EnvMapping};
use crate::oauth::OAuth2Config;

/// Authentication strategy for an API.
///
/// Defines how authentication credentials are applied to HTTP requests.
/// Token and key credential sources (environment variables) are specified on
/// the [`RestApi`](crate::RestApi) struct via its `env_auth` field; Basic
/// carries its own username and password environment lists.
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
///     value_prefix: None,
/// };
/// // Credential env vars are set on RestApi::env_auth
/// ```
///
/// Basic authentication:
///
/// ```
/// use schematic_define::{AuthStrategy, EnvList};
///
/// let auth = AuthStrategy::Basic {
///     username_env: EnvList::single("API_USERNAME"),
///     password_env: EnvList::from_strs(&["API_PASSWORD", "API_SECRET"]),
/// };
/// ```
#[non_exhaustive]
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

        /// Optional prefix prepended to the credential value before it is sent.
        ///
        /// Lets a definition require only the bare secret in the environment
        /// while still emitting a scheme-qualified header value. Gitea sets
        /// this to `"token "` so `Authorization: token <pat>` is produced from
        /// a bare `GITEA_TOKEN`.
        value_prefix: Option<String>,
    },

    /// Basic authentication (username:password).
    ///
    /// Uses HTTP Basic Authentication with base64-encoded credentials.
    /// Generates: `Authorization: Basic <base64(username:password)>`
    ///
    /// Each list is an ordered fallback chain; every name is kept, not just
    /// the first. An empty list means that half has no environment source (it
    /// never supplies an empty credential); an explicit
    /// [`RestApi::env_mapping`](crate::RestApi::env_mapping) still replaces
    /// the mapping derived from these lists wholesale. Both fields are
    /// required when deserializing.
    Basic {
        /// Environment variables naming the username, in priority order.
        username_env: EnvList,
        /// Environment variables naming the password, in priority order.
        password_env: EnvList,
    },

    /// API key in a non-header location (query parameter or cookie).
    ///
    /// Used when the API key is passed in the query string or as a cookie
    /// rather than in a header. Imported from OpenAPI `apiKey` security
    /// schemes with `in: query` or `in: cookie`.
    ///
    /// The key is read from environment variables specified in
    /// `RestApi::env_auth`. Multiple env vars can be specified as a fallback chain.
    ///
    /// ## Examples
    ///
    /// Query parameter API key:
    ///
    /// ```
    /// use schematic_define::auth::{AuthStrategy, ApiKeyLocation};
    ///
    /// let auth = AuthStrategy::ApiKeyParam {
    ///     name: "api_key".to_string(),
    ///     location: ApiKeyLocation::Query,
    /// };
    /// ```
    ///
    /// Cookie-based API key:
    ///
    /// ```
    /// use schematic_define::auth::{AuthStrategy, ApiKeyLocation};
    ///
    /// let auth = AuthStrategy::ApiKeyParam {
    ///     name: "session".to_string(),
    ///     location: ApiKeyLocation::Cookie,
    /// };
    /// ```
    ApiKeyParam {
        /// Parameter name (e.g., "api_key", "token").
        name: String,
        /// Where the API key is sent.
        location: ApiKeyLocation,
    },

    /// OAuth2 authentication.
    ///
    /// The configuration describes the OAuth2 provider endpoints, grant type,
    /// and security requirements. Token lifecycle management is handled by the
    /// `schematic-oauth` runtime crate.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{
    ///     AuthStrategy, AuthorizationCodeGrant, OAuth2ClientAuthMethod, OAuth2Config, PkceRequirement,
    /// };
    ///
    /// let auth = AuthStrategy::OAuth2(OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant {
    ///     authorization_url: "https://github.com/login/oauth/authorize".into(),
    ///     token_url: "https://github.com/login/oauth/access_token".into(),
    ///     revocation_url: None,
    ///     default_scopes: vec!["repo".into()],
    ///     pkce: PkceRequirement::Required,
    ///     client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
    /// }));
    /// ```
    OAuth2(OAuth2Config),
}

/// Explicit authentication methods accepted by a generated REST client.
///
/// Unlike [`AuthStrategy`], this type models what callers may provide
/// programmatically at runtime. A client may accept several explicit methods
/// while only falling back to one environment-based strategy.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Explicit bearer token, typically sent as `Authorization: Bearer <token>`.
    BearerToken {
        /// Optional header name override.
        header: Option<String>,
    },
    /// Explicit API key sent in a custom header.
    ApiKey {
        /// Header name (e.g. `X-API-Key`).
        header: String,
        /// Optional prefix prepended to the credential value before it is sent.
        value_prefix: Option<String>,
    },
    /// Explicit API key sent in a query parameter or cookie.
    ApiKeyParam {
        /// Parameter name (e.g. `api_key`).
        name: String,
        /// Where the API key is sent.
        location: ApiKeyLocation,
    },
    /// Explicit HTTP basic auth credentials.
    Basic,
    /// Explicit OAuth2 bearer token obtained out-of-band.
    OAuth2(OAuth2Config),
}

/// Environment-backed authentication strategy used as a fallback.
///
/// This is intentionally separate from [`AuthMethod`] because OAuth2 metadata
/// may be accepted explicitly without implying that OAuth credentials can be
/// loaded from environment variables.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvAuthStrategy {
    /// Bearer token fallback from [`crate::EnvMapping::bearer_token`].
    BearerToken {
        /// Optional header name override.
        header: Option<String>,
    },
    /// API key fallback from [`crate::EnvMapping::api_key`].
    ApiKey {
        /// Header name (e.g. `X-API-Key`).
        header: String,
        /// Optional prefix prepended to the credential value before it is sent.
        value_prefix: Option<String>,
    },
    /// Parameter-based API key fallback (query string or cookie).
    ApiKeyParam {
        /// Parameter name (e.g. `api_key`).
        name: String,
        /// Where the API key is sent.
        location: ApiKeyLocation,
    },
    /// Basic auth fallback from [`crate::EnvMapping::basic_user`] and
    /// [`crate::EnvMapping::basic_pass`].
    Basic,
}

/// Authentication policy for generated REST clients.
///
/// This decouples the explicit auth methods accepted by the client from the
/// environment-based fallback used when the caller does not inject credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthPolicy {
    /// Explicit auth methods that callers may inject programmatically.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explicit: Vec<AuthMethod>,
    /// Environment-based fallback strategy, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_fallback: Option<EnvAuthStrategy>,
}

impl AuthPolicy {
    /// Builds a backward-compatible auth policy from a legacy [`AuthStrategy`].
    #[must_use]
    pub fn from_auth_strategy(auth: &AuthStrategy) -> Self {
        match auth {
            AuthStrategy::None => Self::default(),
            AuthStrategy::BearerToken { header } => Self {
                explicit: vec![AuthMethod::BearerToken {
                    header: header.clone(),
                }],
                env_fallback: Some(EnvAuthStrategy::BearerToken {
                    header: header.clone(),
                }),
            },
            AuthStrategy::ApiKey {
                header,
                value_prefix,
            } => Self {
                explicit: vec![AuthMethod::ApiKey {
                    header: header.clone(),
                    value_prefix: value_prefix.clone(),
                }],
                env_fallback: Some(EnvAuthStrategy::ApiKey {
                    header: header.clone(),
                    value_prefix: value_prefix.clone(),
                }),
            },
            AuthStrategy::Basic { .. } => Self {
                explicit: vec![AuthMethod::Basic],
                env_fallback: Some(EnvAuthStrategy::Basic),
            },
            AuthStrategy::OAuth2(config) => Self {
                explicit: vec![AuthMethod::OAuth2(config.clone())],
                env_fallback: None,
            },
            AuthStrategy::ApiKeyParam { name, location } => Self {
                explicit: vec![AuthMethod::ApiKeyParam {
                    name: name.clone(),
                    location: *location,
                }],
                env_fallback: Some(EnvAuthStrategy::ApiKeyParam {
                    name: name.clone(),
                    location: *location,
                }),
            },
        }
    }

    /// Returns the first configured OAuth2 provider metadata, if any.
    #[must_use]
    pub fn oauth2(&self) -> Option<&OAuth2Config> {
        self.explicit.iter().find_map(|method| match method {
            AuthMethod::OAuth2(config) => Some(config),
            _ => None,
        })
    }
}

/// One endpoint's authentication, overriding its API's; see
/// [`RestApi::endpoint_auth`](crate::RestApi::endpoint_auth) for resolution.
///
/// Serialized as `"Public"` or `{"Policy":{"policy":{..},"env_mapping":{..}}}`;
/// an absent `env_mapping` reuses the API's effective mapping, and an explicit
/// null is rejected.
///
/// ## Notes
///
/// API build rejects a policy accepting no method (use `Public`) and a policy
/// whose `env_fallback` has no compatible source in the effective mapping.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// One value per endpoint definition; boxing would only complicate literals.
#[allow(clippy::large_enum_variant)]
pub enum EndpointAuth {
    /// No authentication: no credential is looked up or sent.
    Public,
    /// Replaces the API policy; never merged with it.
    Policy {
        /// Methods this endpoint accepts.
        policy: AuthPolicy,
        /// Replaces the API's effective mapping whole when present.
        #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
        env_mapping: Option<EnvMapping>,
    },
}

/// Location for API key authentication.
///
/// Specifies where an API key is sent when using the `ApiKeyParam` auth strategy.
/// This is typically imported from OpenAPI `apiKey` security schemes.
///
/// ## Examples
///
/// ```
/// use schematic_define::auth::ApiKeyLocation;
///
/// let query = ApiKeyLocation::Query;
/// let cookie = ApiKeyLocation::Cookie;
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiKeyLocation {
    /// API key in query string: `?api_key=xxx`
    Query,
    /// API key in cookie: `Cookie: api_key=xxx`
    Cookie,
}

/// Strategy for updating authentication when creating API variants.
///
/// No generated code reads it: the generated `variant()` method it served was
/// removed, and a client is reconfigured through its chained settings.
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
///     _ => unreachable!(),
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
///     value_prefix: None,
/// });
///
/// let result = match strategy {
///     UpdateStrategy::NoChange => AuthStrategy::None,
///     UpdateStrategy::ChangeTo(new) => new,
///     _ => unreachable!(),
/// };
/// assert!(matches!(result, AuthStrategy::ApiKey { .. }));
/// ```
#[non_exhaustive]
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
    fn auth_policy_from_bearer_strategy_supports_explicit_and_env_fallback() {
        let policy = AuthPolicy::from_auth_strategy(&AuthStrategy::BearerToken { header: None });
        assert_eq!(
            policy.explicit,
            vec![AuthMethod::BearerToken { header: None }]
        );
        assert_eq!(
            policy.env_fallback,
            Some(EnvAuthStrategy::BearerToken { header: None })
        );
    }

    #[test]
    fn auth_policy_from_api_key_strategy_supports_explicit_and_env_fallback() {
        let policy = AuthPolicy::from_auth_strategy(&AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
            value_prefix: None,
        });
        assert_eq!(
            policy.explicit,
            vec![AuthMethod::ApiKey {
                header: "X-API-Key".to_string(),
                value_prefix: None,
            }]
        );
        assert_eq!(
            policy.env_fallback,
            Some(EnvAuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
                value_prefix: None,
            })
        );
    }

    #[test]
    fn auth_policy_from_api_key_strategy_carries_value_prefix() {
        let policy = AuthPolicy::from_auth_strategy(&AuthStrategy::ApiKey {
            header: "Authorization".to_string(),
            value_prefix: Some("token ".to_string()),
        });
        assert_eq!(
            policy.explicit,
            vec![AuthMethod::ApiKey {
                header: "Authorization".to_string(),
                value_prefix: Some("token ".to_string()),
            }]
        );
        assert_eq!(
            policy.env_fallback,
            Some(EnvAuthStrategy::ApiKey {
                header: "Authorization".to_string(),
                value_prefix: Some("token ".to_string()),
            })
        );
    }

    #[test]
    fn auth_policy_from_api_key_param_strategy_is_non_empty() {
        let policy = AuthPolicy::from_auth_strategy(&AuthStrategy::ApiKeyParam {
            name: "api_key".to_string(),
            location: ApiKeyLocation::Query,
        });
        assert_eq!(
            policy.explicit,
            vec![AuthMethod::ApiKeyParam {
                name: "api_key".to_string(),
                location: ApiKeyLocation::Query,
            }]
        );
        assert_eq!(
            policy.env_fallback,
            Some(EnvAuthStrategy::ApiKeyParam {
                name: "api_key".to_string(),
                location: ApiKeyLocation::Query,
            })
        );
    }

    #[test]
    fn auth_policy_from_oauth_strategy_is_explicit_only() {
        let config = OAuth2Config::AuthorizationCodePkce(crate::oauth::AuthorizationCodeGrant {
            authorization_url: "https://example.com/authorize".to_string(),
            token_url: "https://example.com/token".to_string(),
            revocation_url: None,
            default_scopes: vec!["read".to_string()],
            pkce: crate::oauth::PkceRequirement::Required,
            client_auth: crate::oauth::OAuth2ClientAuthMethod::ClientSecretPost,
        });

        let policy = AuthPolicy::from_auth_strategy(&AuthStrategy::OAuth2(config.clone()));
        assert_eq!(policy.explicit, vec![AuthMethod::OAuth2(config.clone())]);
        assert!(policy.env_fallback.is_none());
        assert_eq!(policy.oauth2(), Some(&config));
    }

    // ========== ApiKeyLocation Tests ==========

    #[test]
    fn api_key_location_debug_clone_eq() {
        let location = ApiKeyLocation::Query;
        // Explicitly exercise the Clone impl on a Copy type.
        #[allow(clippy::clone_on_copy)]
        let cloned = location.clone();
        assert_eq!(location, cloned);
        assert!(format!("{:?}", location).contains("Query"));
    }

    #[test]
    fn api_key_location_copy() {
        let original = ApiKeyLocation::Cookie;
        let copied = original; // Copy, not move
        assert_eq!(original, copied);
    }

    #[test]
    fn api_key_location_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ApiKeyLocation::Query);
        set.insert(ApiKeyLocation::Cookie);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn api_key_location_serde_roundtrip() {
        let location = ApiKeyLocation::Query;
        let serialized = serde_json::to_string(&location).unwrap();
        let deserialized: ApiKeyLocation = serde_json::from_str(&serialized).unwrap();
        assert_eq!(location, deserialized);
    }

    // ========== AuthStrategy::ApiKeyParam Tests ==========

    #[test]
    fn auth_strategy_api_key_param_query() {
        let auth = AuthStrategy::ApiKeyParam {
            name: "api_key".to_string(),
            location: ApiKeyLocation::Query,
        };

        match auth {
            AuthStrategy::ApiKeyParam { name, location } => {
                assert_eq!(name, "api_key");
                assert_eq!(location, ApiKeyLocation::Query);
            }
            _ => panic!("Expected ApiKeyParam"),
        }
    }

    #[test]
    fn auth_strategy_api_key_param_cookie() {
        let auth = AuthStrategy::ApiKeyParam {
            name: "session".to_string(),
            location: ApiKeyLocation::Cookie,
        };

        match auth {
            AuthStrategy::ApiKeyParam { name, location } => {
                assert_eq!(name, "session");
                assert_eq!(location, ApiKeyLocation::Cookie);
            }
            _ => panic!("Expected ApiKeyParam"),
        }
    }

    #[test]
    fn auth_strategy_api_key_param_clone_eq() {
        let auth = AuthStrategy::ApiKeyParam {
            name: "token".to_string(),
            location: ApiKeyLocation::Query,
        };
        let cloned = auth.clone();
        assert_eq!(auth, cloned);
    }

    #[test]
    fn auth_strategy_api_key_param_serde_roundtrip() {
        let auth = AuthStrategy::ApiKeyParam {
            name: "api_key".to_string(),
            location: ApiKeyLocation::Query,
        };
        let serialized = serde_json::to_string(&auth).unwrap();
        let deserialized: AuthStrategy = serde_json::from_str(&serialized).unwrap();
        assert_eq!(auth, deserialized);
    }

    // ========== UpdateStrategy Tests ==========

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
            value_prefix: None,
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
        let original = UpdateStrategy::ChangeTo(AuthStrategy::Basic {
            username_env: EnvList::single("USER"),
            password_env: EnvList::single("PASS"),
        });
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

    fn basic(username: &[&str], password: &[&str]) -> AuthStrategy {
        AuthStrategy::Basic {
            username_env: EnvList::from_strs(username),
            password_env: EnvList::from_strs(password),
        }
    }

    #[test]
    fn auth_policy_from_basic_strategy_stays_source_neutral() {
        let policy = AuthPolicy::from_auth_strategy(&basic(&["U1", "U2"], &["P1", "P2"]));
        assert_eq!(policy.explicit, vec![AuthMethod::Basic]);
        assert_eq!(policy.env_fallback, Some(EnvAuthStrategy::Basic));
    }

    #[test]
    fn basic_serde_round_trip_is_byte_stable_and_ordered() {
        let auth = basic(&["U1", "U2"], &["P1", "P2", "P3"]);
        let first = serde_json::to_string(&auth).unwrap();
        assert_eq!(
            first,
            r#"{"Basic":{"username_env":{"names":["U1","U2"]},"password_env":{"names":["P1","P2","P3"]}}}"#
        );
        let read: AuthStrategy = serde_json::from_str(&first).unwrap();
        assert_eq!(read, auth);
        assert_eq!(serde_json::to_string(&read).unwrap(), first);
    }

    #[test]
    fn basic_serde_accepts_empty_lists_as_no_environment_source() {
        let auth: AuthStrategy = serde_json::from_str(
            r#"{"Basic":{"username_env":{"names":[]},"password_env":{"names":[]}}}"#,
        )
        .unwrap();
        assert_eq!(auth, basic(&[], &[]));
    }

    #[test]
    fn basic_serde_rejects_malformed_lists_for_each_field() {
        let valid = r#"{"names":["A"]}"#;
        let malformed = [
            None,
            Some("null"),
            Some(r#""A""#),
            Some(r#"["A"]"#),
            Some(r#"{"names":null}"#),
            Some(r#"{"names":"A"}"#),
            Some(r#"{"names":["A",1]}"#),
            Some(r#"{"names":[1,true]}"#),
        ];
        for field in ["username_env", "password_env"] {
            let other = if field == "username_env" { "password_env" } else { "username_env" };
            for value in malformed {
                let json = match value {
                    None => format!(r#"{{"Basic":{{"{other}":{valid}}}}}"#),
                    Some(value) => format!(r#"{{"Basic":{{"{field}":{value},"{other}":{valid}}}}}"#),
                };
                assert!(serde_json::from_str::<AuthStrategy>(&json).is_err(), "accepted {json}");
            }
        }
    }

    #[test]
    fn basic_serde_rejects_unit_tag_duplicate_keys_and_trailing_content() {
        for json in [
            r#""Basic""#,
            r#"{"Basic":null}"#,
            r#"{"Basic":{"username_env":{"names":["A"]},"username_env":{"names":["B"]},"password_env":{"names":["P"]}}}"#,
            r#"{"Basic":{"username_env":{"names":["A"]},"password_env":{"names":["P"],"names":["Q"]}}}"#,
            r#"{"Basic":{"username_env":{"names":["A"]},"password_env":{"names":["P"]}}} x"#,
        ] {
            assert!(serde_json::from_str::<AuthStrategy>(json).is_err(), "accepted {json}");
        }
    }
}
