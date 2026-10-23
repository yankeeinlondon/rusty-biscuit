//! Auth blocks for Postman collections.

use schematic_define::{OAuth2ClientAuthMethod, OAuth2Config};

use crate::export::auth::{ApiKeyLocation, ExportAuth};

use super::types::{PostmanAuth, PostmanVariable};

/// Builds collection-level authentication.
pub(crate) fn build_collection_auth(auth: &ExportAuth) -> Option<PostmanAuth> {
    match auth {
        ExportAuth::Bearer { variable } => Some(PostmanAuth {
            type_field: "bearer".to_string(),
            bearer: Some(vec![PostmanVariable {
                key: "token".to_string(),
                value: Some(format!("{{{{{}}}}}", variable)),
                description: None,
            }]),
            apikey: None,
            basic: None,
            oauth2: None,
        }),
        ExportAuth::ApiKey {
            header,
            variable,
            location,
        } => {
            let reference = format!("{{{{{}}}}}", variable);
            // Postman's API-key auth adds only a header or a query parameter,
            // so a cookie key is sent as a `Cookie` header holding the pair.
            let (key, value, placement) = match location {
                ApiKeyLocation::Header => (header.clone(), reference, "header"),
                ApiKeyLocation::Query => (header.clone(), reference, "query"),
                ApiKeyLocation::Cookie => (
                    "Cookie".to_string(),
                    format!("{header}={reference}"),
                    "header",
                ),
            };
            let attribute = |key: &str, value: String| PostmanVariable {
                key: key.to_string(),
                value: Some(value),
                description: None,
            };
            Some(PostmanAuth {
                type_field: "apikey".to_string(),
                bearer: None,
                apikey: Some(vec![
                    attribute("key", key),
                    attribute("value", value),
                    attribute("in", placement.to_string()),
                ]),
                basic: None,
                oauth2: None,
            })
        }
        ExportAuth::Basic {
            username_var,
            password_var,
        } => Some(PostmanAuth {
            type_field: "basic".to_string(),
            bearer: None,
            apikey: None,
            basic: Some(vec![
                PostmanVariable {
                    key: "username".to_string(),
                    value: Some(format!("{{{{{}}}}}", username_var)),
                    description: None,
                },
                PostmanVariable {
                    key: "password".to_string(),
                    value: Some(format!("{{{{{}}}}}", password_var)),
                    description: None,
                },
            ]),
            oauth2: None,
        }),
        ExportAuth::OAuth2 { config, client_id_var, client_secret_var } => Some(PostmanAuth {
            type_field: "oauth2".to_string(),
            bearer: None,
            apikey: None,
            basic: None,
            oauth2: Some(oauth2_attributes(config, client_id_var, client_secret_var)),
        }),
        ExportAuth::None => Some(PostmanAuth {
            type_field: "noauth".to_string(),
            bearer: None,
            apikey: None,
            basic: None,
            oauth2: None,
        }),
    }
}

/// The attributes of a Postman `oauth2` auth for `config`: grant, the
/// grant's URLs, space-separated scopes, client credentials as variables,
/// and how the client authenticates to the token endpoint.
///
/// Postman has no device-code grant, so a device-code configuration carries
/// its token URL, scopes, and an access-token variable
/// ([`device_token_variable`]) for a token obtained outside Postman. A token
/// revocation URL has no Postman attribute and is not exported.
fn oauth2_attributes(config: &OAuth2Config, client_id_var: &str, client_secret_var: &str) -> Vec<PostmanVariable> {
    let attribute = |key: &str, value: String| PostmanVariable { key: key.to_string(), value: Some(value), description: None };
    let reference = |variable: &str| format!("{{{{{variable}}}}}");
    let mut attributes = Vec::new();
    match config {
        OAuth2Config::AuthorizationCodePkce(grant) => {
            attributes.push(attribute("grant_type", "authorization_code_with_pkce".to_string()));
            attributes.push(attribute("authUrl", grant.authorization_url.clone()));
            attributes.push(attribute("accessTokenUrl", grant.token_url.clone()));
            attributes.push(attribute("challengeAlgorithm", "S256".to_string()));
        }
        OAuth2Config::ClientCredentials(grant) => {
            attributes.push(attribute("grant_type", "client_credentials".to_string()));
            attributes.push(attribute("accessTokenUrl", grant.token_url.clone()));
        }
        OAuth2Config::DeviceCode(grant) => {
            attributes.push(attribute("accessTokenUrl", grant.token_url.clone()));
            attributes.push(attribute("accessToken", reference(&device_token_variable(client_id_var))));
        }
        // `OAuth2Config` is non-exhaustive; a later grant exports its token
        // URL and scopes only.
        other => attributes.push(attribute("accessTokenUrl", other.token_url().to_string())),
    }
    let scopes = config.default_scopes();
    if !scopes.is_empty() {
        attributes.push(attribute("scope", scopes.join(" ")));
    }
    attributes.push(attribute("clientId", reference(client_id_var)));
    let client_auth = config.client_auth();
    if *client_auth != OAuth2ClientAuthMethod::None {
        attributes.push(attribute("clientSecret", reference(client_secret_var)));
    }
    let placement = match client_auth {
        OAuth2ClientAuthMethod::ClientSecretBasic => "header",
        _ => "body",
    };
    attributes.push(attribute("client_authentication", placement.to_string()));
    attributes.push(attribute("addTokenTo", "header".to_string()));
    attributes
}

/// The access-token variable of a device-code grant: the client id
/// variable with `ClientId` replaced by `AccessToken`
/// (`oauth2AccessToken`), so endpoint and member suffixes carry over.
fn device_token_variable(client_id_var: &str) -> String {
    client_id_var.replacen("ClientId", "AccessToken", 1)
}

/// The collection variables the auth block [`build_collection_auth`] builds
/// for `auth` references, each valued empty so importers prompt the user.
///
/// Keys are the variable names `auth` carries (`bearerToken`, `apiKey`,
/// `username`/`password`, `oauth2ClientId`, ...), including any member or
/// endpoint suffix, so the declaration always matches the reference.
///
/// ## Returns
///
/// - [`ExportAuth::Bearer`] and [`ExportAuth::ApiKey`] → one variable.
/// - [`ExportAuth::Basic`] → the username and password variables.
/// - [`ExportAuth::OAuth2`] → the client id, the client secret unless the
///   client is public, and a device-code grant's access token.
/// - [`ExportAuth::None`] → none.
pub(crate) fn auth_variables(auth: &ExportAuth) -> Vec<PostmanVariable> {
    match auth {
        ExportAuth::Bearer { variable } => vec![PostmanVariable {
            key: variable.clone(),
            value: Some(String::new()),
            description: Some("Bearer token for Authorization header".to_string()),
        }],
        ExportAuth::ApiKey { variable, .. } => vec![PostmanVariable {
            key: variable.clone(),
            value: Some(String::new()),
            description: Some("API key value".to_string()),
        }],
        ExportAuth::Basic {
            username_var,
            password_var,
        } => vec![
            PostmanVariable {
                key: username_var.clone(),
                value: Some(String::new()),
                description: Some("Username for Basic authentication".to_string()),
            },
            PostmanVariable {
                key: password_var.clone(),
                value: Some(String::new()),
                description: Some("Password for Basic authentication".to_string()),
            },
        ],
        ExportAuth::OAuth2 { config, client_id_var, client_secret_var } => {
            let mut variables = vec![PostmanVariable {
                key: client_id_var.clone(),
                value: Some(String::new()),
                description: Some("OAuth2 client id".to_string()),
            }];
            if *config.client_auth() != OAuth2ClientAuthMethod::None {
                variables.push(PostmanVariable {
                    key: client_secret_var.clone(),
                    value: Some(String::new()),
                    description: Some("OAuth2 client secret".to_string()),
                });
            }
            if matches!(config, OAuth2Config::DeviceCode(_)) {
                variables.push(PostmanVariable {
                    key: device_token_variable(client_id_var),
                    value: Some(String::new()),
                    description: Some("OAuth2 access token from a device-code sign-in outside Postman".to_string()),
                });
            }
            variables
        }
        ExportAuth::None => Vec::new(),
    }
}
