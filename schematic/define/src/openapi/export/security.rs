use indexmap::IndexMap;
use openapiv3::{SecurityRequirement, SecurityScheme};

use crate::auth::AuthStrategy;
use crate::types::RestApi;

/// Maps authentication strategy to OpenAPI security schemes.
pub fn map_security(auth: &AuthStrategy) -> Option<(String, SecurityScheme)> {
    match auth {
        AuthStrategy::None => None,
        AuthStrategy::BearerToken { header } => {
            if let Some(header_name) = header {
                Some((
                    "bearerAuth".to_string(),
                    SecurityScheme::APIKey {
                        location: openapiv3::APIKeyLocation::Header,
                        name: header_name.clone(),
                        description: Some("Bearer token authentication".to_string()),
                        extensions: IndexMap::new(),
                    },
                ))
            } else {
                Some((
                    "bearerAuth".to_string(),
                    SecurityScheme::HTTP {
                        scheme: "bearer".to_string(),
                        bearer_format: Some("JWT".to_string()),
                        description: Some("Bearer token authentication".to_string()),
                        extensions: IndexMap::new(),
                    },
                ))
            }
        }
        AuthStrategy::ApiKey { header, .. } => Some((
            "apiKeyAuth".to_string(),
            SecurityScheme::APIKey {
                location: openapiv3::APIKeyLocation::Header,
                name: header.clone(),
                description: Some("API Key authentication".to_string()),
                extensions: IndexMap::new(),
            },
        )),
        AuthStrategy::Basic => Some((
            "basicAuth".to_string(),
            SecurityScheme::HTTP {
                scheme: "basic".to_string(),
                bearer_format: None,
                description: Some("Basic HTTP authentication".to_string()),
                extensions: IndexMap::new(),
            },
        )),
        AuthStrategy::ApiKeyParam { name, location } => {
            let api_key_location = match location {
                crate::auth::ApiKeyLocation::Query => openapiv3::APIKeyLocation::Query,
                crate::auth::ApiKeyLocation::Cookie => openapiv3::APIKeyLocation::Cookie,
            };
            Some((
                "apiKeyAuth".to_string(),
                SecurityScheme::APIKey {
                    location: api_key_location,
                    name: name.clone(),
                    description: Some("API Key authentication".to_string()),
                    extensions: IndexMap::new(),
                },
            ))
        }
        AuthStrategy::OAuth2(config) => {
            let flows = match config.grant_type {
                crate::oauth::OAuth2GrantType::AuthorizationCodePkce => {
                    let mut scopes = indexmap::IndexMap::new();
                    for scope in &config.default_scopes {
                        scopes.insert(scope.clone(), String::new());
                    }
                    openapiv3::OAuth2Flows {
                        authorization_code: Some(openapiv3::AuthorizationCodeOAuth2Flow {
                            authorization_url: config.authorization_url.clone().unwrap_or_default(),
                            token_url: config.token_url.clone(),
                            refresh_url: None,
                            scopes,
                            extensions: IndexMap::new(),
                        }),
                        ..Default::default()
                    }
                }
                crate::oauth::OAuth2GrantType::ClientCredentials => {
                    let mut scopes = indexmap::IndexMap::new();
                    for scope in &config.default_scopes {
                        scopes.insert(scope.clone(), String::new());
                    }
                    openapiv3::OAuth2Flows {
                        client_credentials: Some(openapiv3::ClientCredentialsOAuth2Flow {
                            token_url: config.token_url.clone(),
                            refresh_url: None,
                            scopes,
                            extensions: IndexMap::new(),
                        }),
                        ..Default::default()
                    }
                }
                _ => return None,
            };
            Some((
                "oauth2Auth".to_string(),
                SecurityScheme::OAuth2 {
                    flows,
                    description: Some("OAuth2 authentication".to_string()),
                    extensions: IndexMap::new(),
                },
            ))
        }
    }
}

/// Maps security requirements for the API.
pub(super) fn map_security_requirements(api: &RestApi) -> Vec<SecurityRequirement> {
    match &api.auth {
        AuthStrategy::None => vec![],
        AuthStrategy::BearerToken { header: _ } => {
            let mut req = IndexMap::new();
            req.insert("bearerAuth".to_string(), vec![]);
            vec![req]
        }
        AuthStrategy::ApiKey { .. } => {
            let mut req = IndexMap::new();
            req.insert("apiKeyAuth".to_string(), vec![]);
            vec![req]
        }
        AuthStrategy::Basic => {
            let mut req = IndexMap::new();
            req.insert("basicAuth".to_string(), vec![]);
            vec![req]
        }
        AuthStrategy::ApiKeyParam { .. } => {
            let mut req = IndexMap::new();
            req.insert("apiKeyAuth".to_string(), vec![]);
            vec![req]
        }
        AuthStrategy::OAuth2(_) => {
            let mut req = IndexMap::new();
            req.insert("oauth2Auth".to_string(), vec![]);
            vec![req]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_security_none_returns_none() {
        let auth = AuthStrategy::None;
        assert!(map_security(&auth).is_none());
    }

    #[test]
    fn map_security_bearer_token_returns_http_bearer() {
        let auth = AuthStrategy::BearerToken { header: None };
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "bearerAuth");
        match scheme {
            SecurityScheme::HTTP { scheme, .. } => {
                assert_eq!(scheme, "bearer");
            }
            _ => panic!("Expected HTTP scheme"),
        }
    }

    #[test]
    fn map_security_bearer_token_with_custom_header_returns_api_key() {
        let auth = AuthStrategy::BearerToken {
            header: Some("X-Auth-Token".to_string()),
        };
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "bearerAuth");
        match scheme {
            SecurityScheme::APIKey { location, name, .. } => {
                assert!(matches!(location, openapiv3::APIKeyLocation::Header));
                assert_eq!(name, "X-Auth-Token");
            }
            _ => panic!("Expected APIKey scheme"),
        }
    }

    #[test]
    fn map_security_api_key_returns_api_key_scheme() {
        let auth = AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
            value_prefix: None,
        };
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "apiKeyAuth");
        match scheme {
            SecurityScheme::APIKey { location, name, .. } => {
                assert!(matches!(location, openapiv3::APIKeyLocation::Header));
                assert_eq!(name, "X-API-Key");
            }
            _ => panic!("Expected APIKey scheme"),
        }
    }

    #[test]
    fn map_security_basic_returns_http_basic() {
        let auth = AuthStrategy::Basic;
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "basicAuth");
        match scheme {
            SecurityScheme::HTTP { scheme, .. } => {
                assert_eq!(scheme, "basic");
            }
            _ => panic!("Expected HTTP scheme"),
        }
    }
}
