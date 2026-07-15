mod parameters;
mod responses;
mod schema;

pub use responses::map_responses;
pub use schema::{SchemaMapResult, map_all_schemas};

use std::collections::HashSet;

use openapiv3::{Operation, SecurityScheme, Server};

use super::diagnostics::OpenApiDiagnostic;
use super::naming::{deconflict_name, operation_id_fallback, sanitize_rust_ident};
use super::resolver::RefResolver;
use super::{BaseUrlPolicy, OpenApiImportOptions};
use crate::auth::{ApiKeyLocation, AuthStrategy};
use crate::types::Endpoint;
use crate::types::RestMethod;

pub fn map_info(info: &openapiv3::Info) -> (String, String) {
    let name = sanitize_rust_ident(&info.title);
    let description = info
        .description
        .clone()
        .unwrap_or_else(|| info.title.clone());
    (name, description)
}

pub fn map_server(servers: &[Server], policy: &BaseUrlPolicy) -> String {
    match policy {
        BaseUrlPolicy::Override(url) => url.clone(),
        BaseUrlPolicy::FromServers => servers
            .first()
            .map(|s| s.url.clone())
            .unwrap_or_else(|| "https://api.example.com".to_string()),
    }
}

pub fn map_operation(
    op: &Operation,
    path: &str,
    method: RestMethod,
    resolver: &RefResolver,
    used_names: &mut HashSet<String>,
    options: &OpenApiImportOptions,
) -> Result<(Endpoint, Vec<OpenApiDiagnostic>), String> {
    let mut diagnostics = Vec::new();

    let raw_id = match &op.operation_id {
        Some(id) => id.clone(),
        None => {
            let fallback = operation_id_fallback(&method.to_string(), path);
            diagnostics.push(OpenApiDiagnostic::info(
                format!(
                    "#/paths{}/{}",
                    path.replace('/', "~1"),
                    method.to_string().to_lowercase()
                ),
                format!("Missing operationId, using fallback: {}", fallback),
            ));
            fallback
        }
    };

    let sanitized_id = sanitize_rust_ident(&raw_id);
    let id = deconflict_name(&sanitized_id, used_names);
    used_names.insert(id.clone());

    let description = op
        .summary
        .clone()
        .or_else(|| op.description.clone())
        .unwrap_or_else(|| format!("{} {}", method, path));

    let request =
        responses::map_request_body(&op.request_body, resolver, &mut diagnostics, options);

    let response = map_responses(&op.responses, resolver, &mut diagnostics, options);

    let params = parameters::map_parameters(&op.parameters, resolver, &mut diagnostics);

    Ok((
        Endpoint {
            id,
            method,
            path: path.to_string(),
            description,
            request,
            response,
            headers: vec![],
            params: if params.query.is_empty()
                && params.header.is_empty()
                && params.cookie.is_empty()
            {
                None
            } else {
                Some(params)
            },
            oauth_scopes: None,
        },
        diagnostics,
    ))
}

pub fn map_security_scheme(
    scheme: &SecurityScheme,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    location: &str,
) -> Result<AuthStrategy, String> {
    match scheme {
        SecurityScheme::HTTP {
            scheme: auth_scheme,
            ..
        } => {
            let scheme_lower = auth_scheme.to_lowercase();
            match scheme_lower.as_str() {
                "bearer" => Ok(AuthStrategy::BearerToken { header: None }),
                "basic" => Ok(AuthStrategy::Basic),
                _ => Err(format!("Unsupported HTTP auth scheme: {}", auth_scheme)),
            }
        }
        SecurityScheme::APIKey { location, name, .. } => match location {
            openapiv3::APIKeyLocation::Header => Ok(AuthStrategy::ApiKey {
                header: name.clone(),
                value_prefix: None,
            }),
            openapiv3::APIKeyLocation::Query => Ok(AuthStrategy::ApiKeyParam {
                name: name.clone(),
                location: ApiKeyLocation::Query,
            }),
            openapiv3::APIKeyLocation::Cookie => Ok(AuthStrategy::ApiKeyParam {
                name: name.clone(),
                location: ApiKeyLocation::Cookie,
            }),
        },
        SecurityScheme::OAuth2 { flows, .. } => {
            if let Some(auth_code) = &flows.authorization_code {
                let scopes: Vec<String> = auth_code.scopes.keys().cloned().collect();
                Ok(AuthStrategy::OAuth2(crate::oauth::OAuth2Config {
                    grant_type: crate::oauth::OAuth2GrantType::AuthorizationCodePkce,
                    authorization_url: Some(auth_code.authorization_url.clone()),
                    token_url: auth_code.token_url.clone(),
                    revocation_url: None,
                    device_authorization_url: None,
                    default_scopes: scopes,
                    pkce: crate::oauth::PkceRequirement::Required,
                    client_auth: crate::oauth::OAuth2ClientAuthMethod::ClientSecretBasic,
                }))
            } else if let Some(client_creds) = &flows.client_credentials {
                let scopes: Vec<String> = client_creds.scopes.keys().cloned().collect();
                Ok(AuthStrategy::OAuth2(crate::oauth::OAuth2Config {
                    grant_type: crate::oauth::OAuth2GrantType::ClientCredentials,
                    authorization_url: None,
                    token_url: client_creds.token_url.clone(),
                    revocation_url: None,
                    device_authorization_url: None,
                    default_scopes: scopes,
                    pkce: crate::oauth::PkceRequirement::NotUsed,
                    client_auth: crate::oauth::OAuth2ClientAuthMethod::ClientSecretBasic,
                }))
            } else if flows.implicit.is_some() {
                diagnostics.push(OpenApiDiagnostic::warn(
                    location.to_string(),
                    "Implicit OAuth2 flow is not supported (insecure). Use authorization_code with PKCE instead. Falling back to manual auth.".to_string(),
                ));
                Ok(AuthStrategy::None)
            } else if flows.password.is_some() {
                diagnostics.push(OpenApiDiagnostic::warn(
                    location.to_string(),
                    "Resource Owner Password Credentials flow is not supported (insecure). Falling back to manual auth.".to_string(),
                ));
                Ok(AuthStrategy::None)
            } else {
                Err("No supported OAuth2 flow found in security scheme.".to_string())
            }
        }
        SecurityScheme::OpenIDConnect { .. } => Err("OpenID Connect not supported".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_info_extracts_title_and_description() {
        let info = openapiv3::Info {
            title: "My API".to_string(),
            description: Some("A test API".to_string()),
            version: "1.0".to_string(),
            ..Default::default()
        };

        let (name, description) = map_info(&info);

        assert_eq!(name, "MyApi");
        assert_eq!(description, "A test API");
    }

    #[test]
    fn map_info_uses_title_as_fallback_description() {
        let info = openapiv3::Info {
            title: "My API".to_string(),
            description: None,
            version: "1.0".to_string(),
            ..Default::default()
        };

        let (_, description) = map_info(&info);
        assert_eq!(description, "My API");
    }

    #[test]
    fn map_server_uses_override() {
        let servers = vec![Server {
            url: "https://original.com".to_string(),
            ..Default::default()
        }];
        let policy = BaseUrlPolicy::Override("https://custom.com".to_string());

        let result = map_server(&servers, &policy);
        assert_eq!(result, "https://custom.com");
    }

    #[test]
    fn map_server_uses_first_server() {
        let servers = vec![
            Server {
                url: "https://first.com".to_string(),
                ..Default::default()
            },
            Server {
                url: "https://second.com".to_string(),
                ..Default::default()
            },
        ];
        let policy = BaseUrlPolicy::FromServers;

        let result = map_server(&servers, &policy);
        assert_eq!(result, "https://first.com");
    }

    #[test]
    fn map_server_default_when_empty() {
        let servers = vec![];
        let policy = BaseUrlPolicy::FromServers;

        let result = map_server(&servers, &policy);
        assert_eq!(result, "https://api.example.com");
    }

    #[test]
    fn map_security_http_bearer() {
        let scheme = SecurityScheme::HTTP {
            scheme: "bearer".to_string(),
            bearer_format: None,
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme, &mut Vec::new(), "#/test").unwrap();
        assert!(matches!(result, AuthStrategy::BearerToken { .. }));
    }

    #[test]
    fn map_security_http_basic() {
        let scheme = SecurityScheme::HTTP {
            scheme: "basic".to_string(),
            bearer_format: None,
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme, &mut Vec::new(), "#/test").unwrap();
        assert!(matches!(result, AuthStrategy::Basic));
    }

    #[test]
    fn map_security_api_key_header() {
        let scheme = SecurityScheme::APIKey {
            location: openapiv3::APIKeyLocation::Header,
            name: "X-API-Key".to_string(),
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme, &mut Vec::new(), "#/test").unwrap();
        match result {
            AuthStrategy::ApiKey { header, .. } => {
                assert_eq!(header, "X-API-Key");
            }
            _ => panic!("Expected ApiKey"),
        }
    }

    #[test]
    fn map_security_api_key_query() {
        let scheme = SecurityScheme::APIKey {
            location: openapiv3::APIKeyLocation::Query,
            name: "api_key".to_string(),
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme, &mut Vec::new(), "#/test").unwrap();
        match result {
            AuthStrategy::ApiKeyParam { name, location } => {
                assert_eq!(name, "api_key");
                assert!(matches!(location, ApiKeyLocation::Query));
            }
            _ => panic!("Expected ApiKeyParam"),
        }
    }

    #[test]
    fn map_security_api_key_cookie() {
        let scheme = SecurityScheme::APIKey {
            location: openapiv3::APIKeyLocation::Cookie,
            name: "session".to_string(),
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme, &mut Vec::new(), "#/test").unwrap();
        match result {
            AuthStrategy::ApiKeyParam { name, location } => {
                assert_eq!(name, "session");
                assert!(matches!(location, ApiKeyLocation::Cookie));
            }
            _ => panic!("Expected ApiKeyParam"),
        }
    }

    #[test]
    fn map_security_unsupported_http_scheme() {
        let scheme = SecurityScheme::HTTP {
            scheme: "digest".to_string(),
            bearer_format: None,
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme, &mut Vec::new(), "#/test");
        assert!(result.is_err());
    }
}
