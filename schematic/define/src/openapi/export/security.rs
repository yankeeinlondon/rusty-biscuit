use indexmap::IndexMap;
use openapiv3::{ReferenceOr, SecurityRequirement, SecurityScheme};

use crate::auth::{AuthMethod, AuthPolicy, AuthStrategy, EndpointAuth, EnvAuthStrategy};
use crate::types::{Endpoint, RestApi};

/// Maps authentication strategy to OpenAPI security schemes.
///
/// `None` for [`AuthStrategy::None`] and for an OAuth2 device-code grant,
/// which OpenAPI 3.0 has no flow for.
pub fn map_security(auth: &AuthStrategy) -> Option<(String, SecurityScheme)> {
    AuthPolicy::from_auth_strategy(auth)
        .explicit
        .first()
        .and_then(method_scheme)
}

/// The base component name and scheme of one accepted method; `None` for an
/// OAuth2 device-code grant.
fn method_scheme(method: &AuthMethod) -> Option<(String, SecurityScheme)> {
    match method {
        AuthMethod::BearerToken { header } => {
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
        AuthMethod::ApiKey { header, .. } => Some((
            "apiKeyAuth".to_string(),
            SecurityScheme::APIKey {
                location: openapiv3::APIKeyLocation::Header,
                name: header.clone(),
                description: Some("API Key authentication".to_string()),
                extensions: IndexMap::new(),
            },
        )),
        AuthMethod::Basic => Some((
            "basicAuth".to_string(),
            SecurityScheme::HTTP {
                scheme: "basic".to_string(),
                bearer_format: None,
                description: Some("Basic HTTP authentication".to_string()),
                extensions: IndexMap::new(),
            },
        )),
        AuthMethod::ApiKeyParam { name, location } => {
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
        AuthMethod::OAuth2(config) => {
            let flows = match config {
                crate::oauth::OAuth2Config::AuthorizationCodePkce(grant) => {
                    let mut scopes = indexmap::IndexMap::new();
                    for scope in &grant.default_scopes {
                        scopes.insert(scope.clone(), String::new());
                    }
                    openapiv3::OAuth2Flows {
                        authorization_code: Some(openapiv3::AuthorizationCodeOAuth2Flow {
                            authorization_url: grant.authorization_url.clone(),
                            token_url: grant.token_url.clone(),
                            refresh_url: None,
                            scopes,
                            extensions: IndexMap::new(),
                        }),
                        ..Default::default()
                    }
                }
                crate::oauth::OAuth2Config::ClientCredentials(grant) => {
                    let mut scopes = indexmap::IndexMap::new();
                    for scope in &grant.default_scopes {
                        scopes.insert(scope.clone(), String::new());
                    }
                    openapiv3::OAuth2Flows {
                        client_credentials: Some(openapiv3::ClientCredentialsOAuth2Flow {
                            token_url: grant.token_url.clone(),
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
///
/// Names the scheme [`map_security`] registers, so a strategy with no
/// scheme has no requirement rather than a dangling one.
pub(super) fn map_security_requirements(api: &RestApi) -> Vec<SecurityRequirement> {
    map_security(&api.auth)
        .map(|(name, _)| vec![IndexMap::from([(name, Vec::new())])])
        .unwrap_or_default()
}

/// Every method a policy accepts, in order: its explicit methods, then its
/// environment fallback when no explicit method already names that method.
pub(super) fn accepted_methods(policy: &AuthPolicy) -> Vec<AuthMethod> {
    let mut methods = policy.explicit.clone();
    if let Some(fallback) = &policy.env_fallback {
        let method = fallback_method(fallback);
        if !methods.contains(&method) {
            methods.push(method);
        }
    }
    methods
}

fn fallback_method(fallback: &EnvAuthStrategy) -> AuthMethod {
    match fallback {
        EnvAuthStrategy::BearerToken { header } => AuthMethod::BearerToken { header: header.clone() },
        EnvAuthStrategy::ApiKey { header, value_prefix } => AuthMethod::ApiKey {
            header: header.clone(),
            value_prefix: value_prefix.clone(),
        },
        EnvAuthStrategy::ApiKeyParam { name, location } => AuthMethod::ApiKeyParam {
            name: name.clone(),
            location: *location,
        },
        EnvAuthStrategy::Basic => AuthMethod::Basic,
    }
}

/// The operation-level `security` of `endpoint`.
///
/// `None` inherits the document's requirements; a public endpoint is
/// `Some([])`; a replacement policy lists each accepted method as its own
/// requirement object, so any one of them satisfies the operation. A
/// replacement none of whose methods has an OpenAPI scheme (an OAuth2
/// device-code grant) is `None`; the generator reports that loss.
///
/// Each scheme is registered in `schemes` through [`register_scheme`].
pub(super) fn endpoint_security(
    endpoint: &Endpoint,
    schemes: &mut IndexMap<String, ReferenceOr<SecurityScheme>>,
) -> Option<Vec<SecurityRequirement>> {
    match endpoint.auth.as_ref()? {
        EndpointAuth::Public => Some(Vec::new()),
        EndpointAuth::Policy { policy, .. } => {
            let mut requirements: Vec<SecurityRequirement> = Vec::new();
            for (base, scheme) in accepted_methods(policy).iter().filter_map(method_scheme) {
                let name = register_scheme(schemes, &base, scheme);
                let requirement = IndexMap::from([(name, Vec::new())]);
                if !requirements.contains(&requirement) {
                    requirements.push(requirement);
                }
            }
            (!requirements.is_empty()).then_some(requirements)
        }
    }
}

/// Registers `scheme` and returns the component name that holds it.
///
/// An identical scheme already registered under any name is reused. Otherwise
/// `base` is taken when free, then `base2`, `base3`, ...; a name is never
/// reassigned to a different scheme. Endpoints are visited in definition
/// order, so the numbering is deterministic.
pub(super) fn register_scheme(
    schemes: &mut IndexMap<String, ReferenceOr<SecurityScheme>>,
    base: &str,
    scheme: SecurityScheme,
) -> String {
    let scheme = ReferenceOr::Item(scheme);
    if let Some((name, _)) = schemes.iter().find(|(_, existing)| **existing == scheme) {
        return name.clone();
    }
    let mut name = base.to_string();
    let mut suffix = 2;
    while schemes.contains_key(&name) {
        name = format!("{base}{suffix}");
        suffix += 1;
    }
    schemes.insert(name.clone(), scheme);
    name
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
        let auth = AuthStrategy::Basic {
            username_env: crate::EnvList::single("USER"),
            password_env: crate::EnvList::single("PASS"),
        };
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "basicAuth");
        match scheme {
            SecurityScheme::HTTP { scheme, .. } => {
                assert_eq!(scheme, "basic");
            }
            _ => panic!("Expected HTTP scheme"),
        }
    }

    use crate::auth::ApiKeyLocation;
    use crate::oauth::{DeviceCodeGrant, OAuth2ClientAuthMethod, OAuth2Config};
    use crate::{DefinitionContext, Endpoint};

    fn bearer() -> SecurityScheme {
        map_security(&AuthStrategy::BearerToken { header: None }).unwrap().1
    }

    fn query_key() -> AuthMethod {
        AuthMethod::ApiKeyParam { name: "key".into(), location: ApiKeyLocation::Query }
    }

    #[test]
    fn register_scheme_reuses_identical_content_and_never_overwrites_a_different_scheme() {
        let mut schemes = IndexMap::new();
        assert_eq!(register_scheme(&mut schemes, "bearerAuth", bearer()), "bearerAuth");
        assert_eq!(register_scheme(&mut schemes, "anything", bearer()), "bearerAuth");
        let (_, header_key) = method_scheme(&AuthMethod::ApiKey { header: "X-Key".into(), value_prefix: None }).unwrap();
        let (_, query) = method_scheme(&query_key()).unwrap();
        assert_eq!(register_scheme(&mut schemes, "bearerAuth", header_key.clone()), "bearerAuth2");
        assert_eq!(register_scheme(&mut schemes, "bearerAuth", query), "bearerAuth3");
        assert_eq!(schemes.len(), 3);
        assert_eq!(schemes["bearerAuth"], ReferenceOr::Item(bearer()));
        assert_eq!(schemes["bearerAuth2"], ReferenceOr::Item(header_key));
    }

    #[test]
    fn endpoint_security_maps_public_replacement_and_inheritance() {
        let endpoint = |builder: crate::builders::EndpointBuilder<crate::builders::Present, crate::builders::Present>| {
            builder.build().unwrap()
        };
        let base = || Endpoint::get("Get", "/a").describes("Get").returns_empty();
        let mut schemes = IndexMap::new();

        assert_eq!(endpoint_security(&endpoint(base()), &mut schemes), None);
        assert_eq!(endpoint_security(&endpoint(base().public()), &mut schemes), Some(vec![]));

        let two = AuthPolicy {
            explicit: vec![AuthMethod::Basic, query_key()],
            env_fallback: Some(EnvAuthStrategy::Basic),
        };
        let requirements = endpoint_security(&endpoint(base().auth(two)), &mut schemes).unwrap();
        let expected: Vec<SecurityRequirement> = vec![
            IndexMap::from([("basicAuth".to_string(), vec![])]),
            IndexMap::from([("apiKeyAuth".to_string(), vec![])]),
        ];
        assert_eq!(requirements, expected);
        assert_eq!(schemes.keys().collect::<Vec<_>>(), ["basicAuth", "apiKeyAuth"]);
    }

    #[test]
    fn a_device_code_grant_has_no_scheme_and_no_requirement() {
        let device = AuthStrategy::OAuth2(OAuth2Config::DeviceCode(DeviceCodeGrant {
            device_authorization_url: "https://auth.example.com/device".into(),
            token_url: "https://auth.example.com/token".into(),
            revocation_url: None,
            default_scopes: vec![],
            client_auth: OAuth2ClientAuthMethod::None,
        }));
        assert!(map_security(&device).is_none());
        let mut api = crate::RestApi::builder(&DefinitionContext::types_in("::m"), "Device", "https://example.com")
            .describes("Device").no_auth().build().unwrap();
        api.auth = device;
        assert!(map_security_requirements(&api).is_empty());
    }
}
