mod classify;
mod discriminator;
mod parameters;
mod responses;
mod schema;

pub use classify::{ResponseClassification, classify_responses};
pub use schema::map_all_schemas;

use std::collections::HashSet;

use openapiv3::{Operation, Parameter, ReferenceOr, SecurityScheme, Server};
use serde_json::{Map, Value};

use super::diagnostics::OpenApiDiagnostic;
use super::naming::{deconflict_name, operation_id_fallback, sanitize_rust_ident};
use super::resolver::RefResolver;
use super::{BaseUrlPolicy, OpenApiImportOptions};
use crate::auth::{ApiKeyLocation, AuthStrategy};
use crate::constraints::{Constraints, PatternAdmission, admit_pattern, endpoint_problems};
use crate::headers::EnvList;
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

/// Maps one operation; `path_item_parameters` are the enclosing path item's
/// `parameters`, which the operation's own list overrides by name and location.
///
/// Value rules the endpoint cannot carry (a numeric rule on a text form
/// field, a source `minimum` above its `maximum`) are removed from the input
/// that names them, each reported as a WARN at the operation's pointer; the
/// import itself never fails for them.
#[allow(clippy::too_many_arguments)]
pub fn map_operation<'a>(
    api: &str,
    op: &'a Operation,
    path_item_parameters: &'a [ReferenceOr<Parameter>],
    path: &str,
    method: RestMethod,
    resolver: &RefResolver<'a>,
    used_names: &mut HashSet<String>,
    options: &OpenApiImportOptions,
) -> Result<(Endpoint, Vec<OpenApiDiagnostic>), String> {
    let mut diagnostics = Vec::new();
    let operation_pointer = format!(
        "#/paths/{}/{}",
        path.replace('~', "~0").replace('/', "~1"),
        method.to_string().to_lowercase()
    );

    let raw_id = match &op.operation_id {
        Some(id) => id.clone(),
        None => {
            let fallback = operation_id_fallback(&method.to_string(), path);
            diagnostics.push(OpenApiDiagnostic::info(
                operation_pointer.clone(),
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

    let (request, request_constraints) =
        responses::map_request_body(&op.request_body, &operation_pointer, resolver, &mut diagnostics, options)
            .map_or((None, None), |(request, rules)| (Some(request), rules));

    let classification = classify_responses(api, &id, &op.responses, resolver).map_err(|error| error.to_string())?;
    diagnostics.extend(classification.diagnostics);

    let params = parameters::map_parameters(
        path,
        method,
        path_item_parameters,
        &op.parameters,
        resolver,
        &mut diagnostics,
    );

    let mut endpoint = Endpoint { request_constraints, rate_limit: None, retry_guidance: None, deprecated: op.deprecated, auth: None, errors: classification.errors, no_declared_errors: false,
        id,
        method,
        path: path.to_string(),
        description,
        request,
        response: classification.successes,
        headers: vec![],
        params: if params.query.is_empty()
            && params.header.is_empty()
            && params.cookie.is_empty()
            && params.path.is_empty()
        {
            None
        } else {
            Some(params)
        },
        cross_origin: None, pagination: None,
    };
    drop_unpreservable_rules(&mut endpoint, &operation_pointer, &mut diagnostics);
    Ok((endpoint, diagnostics))
}

/// Keywords no parameter, form field, or body root rule can carry.
const UNCARRIED_KEYWORDS: [&str; 7] =
    ["minProperties", "maxProperties", "patternProperties", "propertyNames", "contains", "prefixItems", "dependencies"];

/// The value rules of an inline input schema found at the RFC 6901
/// `pointer`, read from its exact source form; `None` when it has none.
///
/// Every pattern (the root's and its `items`') is checked with
/// [`admit_pattern`]: one outside the enforced subset, or not a valid
/// regular expression, is kept unenforced and reported as a WARN at
/// `{pointer}/pattern`. A keyword in [`UNCARRIED_KEYWORDS`] is reported as a
/// WARN at `{pointer}/{keyword}`. `format`, `title`, and `description` are
/// annotations and draw no diagnostic.
pub(super) fn import_constraints(
    schema: &impl serde::Serialize,
    pointer: &str,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<Box<Constraints>> {
    let Value::Object(source) = resolver.source_schema(schema, pointer) else { return None };
    let rules = admit_rules(&source, pointer, diagnostics);
    (!rules.is_empty()).then(|| Box::new(rules))
}

fn admit_rules(source: &Map<String, Value>, pointer: &str, diagnostics: &mut Vec<OpenApiDiagnostic>) -> Constraints {
    for keyword in UNCARRIED_KEYWORDS.into_iter().filter(|keyword| source.contains_key(*keyword)) {
        diagnostics.push(OpenApiDiagnostic::warn(
            format!("{pointer}/{keyword}"),
            format!("`{keyword}` cannot be carried by a parameter, form field, or body rule; not preserved"),
        ));
    }
    let mut rules = Constraints::from_schema(source);
    rules.items = match source.get("items") {
        Some(Value::Object(items)) => {
            Some(Box::new(admit_rules(items, &format!("{pointer}/items"), diagnostics))).filter(|items| !items.is_empty())
        }
        _ => None,
    };
    let Some(pattern) = rules.pattern.take() else { return rules };
    let (message, rules) = match admit_pattern(&pattern) {
        PatternAdmission::Enforced { .. } => return rules.pattern(pattern),
        PatternAdmission::Unsupported { reason } => (
            format!("Pattern is outside the enforced pattern subset ({reason}); preserved and not enforced"),
            rules.unenforced_pattern(pattern),
        ),
        PatternAdmission::Invalid { reason } => (
            format!("Pattern is not a valid regular expression ({reason}); preserved and not enforced"),
            rules.unenforced_pattern(pattern),
        ),
    };
    diagnostics.push(OpenApiDiagnostic::warn(format!("{pointer}/pattern"), message));
    rules
}

/// Removes the rules of every input [`endpoint_problems`] names, reporting
/// each problem as a WARN at `operation_pointer`.
fn drop_unpreservable_rules(endpoint: &mut Endpoint, operation_pointer: &str, diagnostics: &mut Vec<OpenApiDiagnostic>) {
    let problems = endpoint_problems(endpoint);
    if problems.is_empty() {
        return;
    }
    for problem in &problems {
        diagnostics.push(OpenApiDiagnostic::warn(
            operation_pointer.to_string(),
            format!("Rule `{}` on {} not preserved: {}", problem.keyword, problem.path, problem.reason),
        ));
    }
    // A problem path is its input's path plus one `[]` per array level.
    let named = |input: &str| {
        problems.iter().any(|problem| {
            problem.path.strip_prefix(input).is_some_and(|rest| rest.split("[]").all(str::is_empty))
        })
    };
    if let Some(params) = &mut endpoint.params {
        let located = [("path", &mut params.path), ("query", &mut params.query), ("header", &mut params.header), ("cookie", &mut params.cookie)];
        for (location, params) in located {
            for param in params.iter_mut().filter(|param| named(&format!("{location}.{}", param.name))) {
                param.constraints = None;
            }
        }
    }
    if let Some(crate::ApiRequest::FormData { fields } | crate::ApiRequest::UrlEncoded { fields }) = &mut endpoint.request {
        for field in fields.iter_mut().filter(|field| named(&format!("form.{}", field.name))) {
            field.constraints = None;
        }
    }
    if named("body") {
        endpoint.request_constraints = None;
    }
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
                "basic" => Ok(AuthStrategy::Basic {
                    username_env: EnvList::new(vec![]),
                    password_env: EnvList::new(vec![]),
                }),
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
                Ok(AuthStrategy::OAuth2(crate::oauth::OAuth2Config::AuthorizationCodePkce(
                    crate::oauth::AuthorizationCodeGrant {
                        authorization_url: auth_code.authorization_url.clone(),
                        token_url: auth_code.token_url.clone(),
                        revocation_url: None,
                        default_scopes: scopes,
                        pkce: crate::oauth::PkceRequirement::Required,
                        client_auth: crate::oauth::OAuth2ClientAuthMethod::ClientSecretBasic,
                    },
                )))
            } else if let Some(client_creds) = &flows.client_credentials {
                let scopes: Vec<String> = client_creds.scopes.keys().cloned().collect();
                Ok(AuthStrategy::OAuth2(crate::oauth::OAuth2Config::ClientCredentials(
                    crate::oauth::ClientCredentialsGrant {
                        token_url: client_creds.token_url.clone(),
                        revocation_url: None,
                        default_scopes: scopes,
                        client_auth: crate::oauth::OAuth2ClientAuthMethod::ClientSecretBasic,
                    },
                )))
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
        assert_eq!(result, AuthStrategy::Basic {
            username_env: EnvList::new(vec![]),
            password_env: EnvList::new(vec![]),
        });
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
