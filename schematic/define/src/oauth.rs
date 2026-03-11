//! OAuth2 configuration types for REST APIs.
//!
//! This module defines declarative OAuth2 metadata for API definitions.
//! These types describe OAuth2 endpoints, grant types, and security requirements.
//! Actual token lifecycle management is handled by the `schematic-oauth` runtime crate.

use serde::{Deserialize, Serialize};

/// OAuth2 provider configuration for an API.
///
/// Describes the OAuth2 endpoints, grant type, and security requirements
/// for a provider. This is declarative metadata — actual token lifecycle
/// management is handled by the `schematic-oauth` runtime crate.
///
/// ## Examples
///
/// ```
/// use schematic_define::{OAuth2Config, OAuth2GrantType, PkceRequirement, OAuth2ClientAuthMethod};
///
/// let config = OAuth2Config {
///     grant_type: OAuth2GrantType::AuthorizationCodePkce,
///     authorization_url: Some("https://github.com/login/oauth/authorize".into()),
///     token_url: "https://github.com/login/oauth/access_token".into(),
///     revocation_url: None,
///     device_authorization_url: None,
///     default_scopes: vec!["repo".into(), "read:user".into()],
///     pkce: PkceRequirement::Required,
///     client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2Config {
    /// The OAuth2 grant type used by this provider.
    pub grant_type: OAuth2GrantType,
    /// Authorization endpoint URL (required for authorization code flows).
    pub authorization_url: Option<String>,
    /// Token endpoint URL.
    pub token_url: String,
    /// Token revocation endpoint URL (optional).
    pub revocation_url: Option<String>,
    /// Device authorization endpoint URL (for device code flow).
    pub device_authorization_url: Option<String>,
    /// Default OAuth2 scopes requested by this API.
    pub default_scopes: Vec<String>,
    /// PKCE requirement for this provider.
    pub pkce: PkceRequirement,
    /// How client credentials are sent to the token endpoint.
    pub client_auth: OAuth2ClientAuthMethod,
}

/// OAuth2 grant type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OAuth2GrantType {
    /// Authorization Code with PKCE (recommended for most use cases).
    AuthorizationCodePkce,
    /// Client Credentials (service-to-service).
    ClientCredentials,
    /// Device Code (for input-constrained devices).
    DeviceCode,
}

/// PKCE (Proof Key for Code Exchange) requirement.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PkceRequirement {
    /// PKCE is required by the provider.
    #[default]
    Required,
    /// PKCE is supported but not required.
    Supported,
    /// PKCE is not used (e.g., client credentials flow).
    NotUsed,
}

/// How client credentials are sent to the OAuth2 token endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OAuth2ClientAuthMethod {
    /// HTTP Basic authentication with client_id:client_secret.
    #[default]
    ClientSecretBasic,
    /// Client credentials in the POST body.
    ClientSecretPost,
    /// No client authentication (public clients).
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;

    #[test]
    fn oauth2_config_serde_roundtrip() {
        let config = OAuth2Config {
            grant_type: OAuth2GrantType::AuthorizationCodePkce,
            authorization_url: Some("https://example.com/authorize".into()),
            token_url: "https://example.com/token".into(),
            revocation_url: Some("https://example.com/revoke".into()),
            device_authorization_url: None,
            default_scopes: vec!["read".into(), "write".into()],
            pkce: PkceRequirement::Required,
            client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: OAuth2Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn oauth2_grant_type_serde_roundtrip() {
        for variant in [
            OAuth2GrantType::AuthorizationCodePkce,
            OAuth2GrantType::ClientCredentials,
            OAuth2GrantType::DeviceCode,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: OAuth2GrantType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn pkce_requirement_default_is_required() {
        assert_eq!(PkceRequirement::default(), PkceRequirement::Required);
    }

    #[test]
    fn pkce_requirement_serde_roundtrip() {
        for variant in [
            PkceRequirement::Required,
            PkceRequirement::Supported,
            PkceRequirement::NotUsed,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: PkceRequirement = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn client_auth_method_default_is_basic() {
        assert_eq!(
            OAuth2ClientAuthMethod::default(),
            OAuth2ClientAuthMethod::ClientSecretBasic
        );
    }

    #[test]
    fn client_auth_method_serde_roundtrip() {
        for variant in [
            OAuth2ClientAuthMethod::ClientSecretBasic,
            OAuth2ClientAuthMethod::ClientSecretPost,
            OAuth2ClientAuthMethod::None,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: OAuth2ClientAuthMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn auth_strategy_oauth2_serde_roundtrip() {
        let strategy = AuthStrategy::OAuth2(OAuth2Config {
            grant_type: OAuth2GrantType::ClientCredentials,
            authorization_url: None,
            token_url: "https://auth.example.com/token".into(),
            revocation_url: None,
            device_authorization_url: None,
            default_scopes: vec!["api".into()],
            pkce: PkceRequirement::NotUsed,
            client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
        });

        let json = serde_json::to_string(&strategy).unwrap();
        let parsed: AuthStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, parsed);
    }

    #[test]
    fn env_mapping_with_oauth_fields_serde_roundtrip() {
        use crate::headers::{EnvList, EnvMapping};

        let mapping = EnvMapping {
            bearer_token: None,
            basic_user: None,
            basic_pass: None,
            api_key: None,
            oauth_client_id: Some(EnvList::single("GITHUB_CLIENT_ID")),
            oauth_client_secret: Some(EnvList::single("GITHUB_CLIENT_SECRET")),
            oauth_redirect_uri: Some(EnvList::single("GITHUB_REDIRECT_URI")),
        };

        let json = serde_json::to_string(&mapping).unwrap();
        let parsed: EnvMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(mapping, parsed);
    }

    #[test]
    fn env_mapping_backward_compat_without_oauth_fields() {
        use crate::headers::EnvMapping;

        // JSON without OAuth fields should deserialize correctly
        let json = r#"{"bearer_token":{"names":["API_KEY"]}}"#;
        let parsed: EnvMapping = serde_json::from_str(json).unwrap();
        assert!(parsed.bearer_token.is_some());
        assert!(parsed.oauth_client_id.is_none());
        assert!(parsed.oauth_client_secret.is_none());
        assert!(parsed.oauth_redirect_uri.is_none());
    }

    #[test]
    fn endpoint_with_oauth_scopes_serde() {
        use crate::response::ApiResponse;
        use crate::types::{Endpoint, RestMethod};

        let endpoint = Endpoint {
            id: "ListRepos".into(),
            method: RestMethod::Get,
            path: "/repos".into(),
            description: "List repositories".into(),
            request: None,
            response: ApiResponse::json_type("ListReposResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: Some(vec!["repo".into(), "read:org".into()]),
        };

        // Verify the field is accessible
        assert_eq!(
            endpoint.oauth_scopes.as_ref().unwrap(),
            &["repo", "read:org"]
        );
    }

    #[test]
    fn endpoint_without_oauth_scopes() {
        use crate::response::ApiResponse;
        use crate::types::{Endpoint, RestMethod};

        let endpoint = Endpoint {
            id: "Health".into(),
            method: RestMethod::Get,
            path: "/health".into(),
            description: "Health check".into(),
            request: None,
            response: ApiResponse::json_type("HealthResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        };

        assert!(endpoint.oauth_scopes.is_none());
    }
}
