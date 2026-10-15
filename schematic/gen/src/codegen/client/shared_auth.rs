//! Request-auth helpers emitted once into the generated `shared.rs`.
//!
//! Every generated client calls these instead of carrying its own copy, so a
//! client holds a few call sites, not the logic. They cover credential
//! resolution under the client's effective [`AuthPolicy`] (explicit methods,
//! env fallbacks, the "authentication required" error), the `ApiKeyParam`
//! credential and its injection into the query or the Cookie header, header
//! parsing through the HTTP types, and the credential-free `Debug` output of
//! a client.
//!
//! [`AuthPolicy`]: schematic_define::AuthPolicy
//!
//! **Temporary:** the explicit key and the env scan stand in until a shared
//! runtime owns credentials; both are expected to be replaced there.

use proc_macro2::TokenStream;
use quote::quote;

/// Generates the shared request-auth helpers.
pub fn generate_request_auth_helpers() -> TokenStream {
    quote! {
        /// An API key for an `ApiKeyParam` policy, resolved for one request.
        ///
        /// `Debug` prints the name and location, never the key.
        #[derive(Debug, Clone)]
        pub struct ApiKeyParamCredential {
            /// Query parameter or cookie name; it owns that name on the wire.
            pub name: String,
            /// Where the key is sent.
            pub location: schematic_define::ApiKeyLocation,
            /// The key.
            pub value: schematic_define::SensitiveString,
        }

        /// The API-level headers and the API-key parameter a client sends.
        pub type RequestAuth = (Vec<(String, String)>, Option<ApiKeyParamCredential>);

        /// Resolves the credentials a client sends with one request.
        ///
        /// An explicit credential (an auth header set on `headers`, or a key
        /// set with `with_api_key_param` that `policy` accepts) wins and
        /// disables every env fallback. Otherwise `policy`'s env fallback,
        /// if any, supplies one: a header, or for `ApiKeyParam` the first
        /// non-empty variable named by `env_auth`.
        ///
        /// ## Errors
        ///
        /// Returns `SchematicError::AuthenticationRequired` when `policy`
        /// requires authentication and neither source produced a
        /// credential, and `SchematicError::Header` when a header name is
        /// invalid.
        pub fn resolve_request_auth(
            policy: &schematic_define::AuthPolicy,
            headers: &schematic_define::Headers,
            env_auth: &[String],
            api_key_param: Option<&schematic_define::SensitiveString>,
        ) -> Result<RequestAuth, SchematicError> {
            let mut headers = headers.clone();
            let mut key = explicit_api_key_param(policy, api_key_param);
            let explicit = headers.has_explicit_auth() || key.is_some();

            if !explicit {
                headers = apply_env_fallback(policy, headers);
                key = env_api_key_param(policy, env_auth);
            }

            let required = !policy.explicit.is_empty() || policy.env_fallback.is_some();
            if required && !explicit && key.is_none() && !headers_satisfy_fallback(policy, &headers) {
                return Err(authentication_required_error(policy, &headers, env_auth));
            }

            Ok((headers.build()?, key))
        }

        fn accepted_explicit_auth_methods(policy: &schematic_define::AuthPolicy) -> Vec<String> {
            policy
                .explicit
                .iter()
                .map(|method| match method {
                    schematic_define::AuthMethod::BearerToken { header } => match header.as_deref() {
                        Some(header) => format!("an explicit bearer token in `{header}`"),
                        None => "an explicit bearer token".to_string(),
                    },
                    schematic_define::AuthMethod::ApiKey { header, .. } => {
                        format!("an explicit API key in `{header}`")
                    }
                    schematic_define::AuthMethod::ApiKeyParam { name, .. } => {
                        format!("an explicit API key for `{name}` (`with_api_key_param`)")
                    }
                    schematic_define::AuthMethod::Basic => {
                        "explicit basic auth credentials".to_string()
                    }
                    schematic_define::AuthMethod::OAuth2(_) => {
                        "an explicit OAuth access token".to_string()
                    }
                    _ => "explicit authentication".to_string(),
                })
                .collect()
        }

        fn env_fallback_var_names(
            policy: &schematic_define::AuthPolicy,
            headers: &schematic_define::Headers,
            env_auth: &[String],
        ) -> Vec<String> {
            match &policy.env_fallback {
                Some(schematic_define::EnvAuthStrategy::BearerToken { .. }) => headers
                    .env_mapping()
                    .bearer_token
                    .as_ref()
                    .map(|list| list.names().to_vec())
                    .unwrap_or_default(),
                Some(schematic_define::EnvAuthStrategy::ApiKey { .. }) => headers
                    .env_mapping()
                    .api_key
                    .as_ref()
                    .map(|api_key| api_key.names.names().to_vec())
                    .unwrap_or_default(),
                Some(schematic_define::EnvAuthStrategy::ApiKeyParam { .. }) => env_auth.to_vec(),
                Some(schematic_define::EnvAuthStrategy::Basic) => {
                    let mut vars = Vec::new();
                    if let Some(user) = headers.env_mapping().basic_user.as_ref() {
                        vars.extend(user.names().iter().cloned());
                    }
                    if let Some(pass) = headers.env_mapping().basic_pass.as_ref() {
                        vars.extend(pass.names().iter().cloned());
                    }
                    vars
                }
                None => Vec::new(),
                _ => Vec::new(),
            }
        }

        fn authentication_required_error(
            policy: &schematic_define::AuthPolicy,
            headers: &schematic_define::Headers,
            env_auth: &[String],
        ) -> SchematicError {
            let explicit_methods = accepted_explicit_auth_methods(policy);
            let env_fallback_vars = env_fallback_var_names(policy, headers, env_auth);
            let mut options = Vec::new();

            if !explicit_methods.is_empty() {
                options.push(explicit_methods.join(", "));
            }

            if !env_fallback_vars.is_empty() {
                options.push(format!(
                    "set one of the fallback env vars `{}`",
                    env_fallback_vars.join("`, `")
                ));
            }

            let mut message = if options.is_empty() {
                "Authentication required.".to_string()
            } else {
                format!("Authentication required: {}.", options.join(" or "))
            };

            if policy.oauth2().is_some() {
                message.push_str(
                    " Obtain the OAuth token with schematic-oauth, then inject it with `.oauth_token(...)`.",
                );
            }

            SchematicError::AuthenticationRequired {
                message,
                explicit_methods,
                env_fallback_vars,
            }
        }

        fn apply_env_fallback(
            policy: &schematic_define::AuthPolicy,
            headers: schematic_define::Headers,
        ) -> schematic_define::Headers {
            let env_mapping = headers.env_mapping().clone();

            match &policy.env_fallback {
                Some(schematic_define::EnvAuthStrategy::BearerToken { header }) => {
                    let token = env_mapping
                        .bearer_token
                        .as_ref()
                        .and_then(|list| list.names().iter().find_map(|name| std::env::var(name).ok()));

                    match (token, header.as_deref()) {
                        (Some(token), Some(header)) => headers.use_bearer_token_with_header(token, header),
                        (Some(token), None) => headers.use_bearer_token(token),
                        _ => headers,
                    }
                }
                Some(schematic_define::EnvAuthStrategy::ApiKey { header, value_prefix }) => {
                    let key = env_mapping
                        .api_key
                        .as_ref()
                        .and_then(|api_key| {
                            api_key
                                .names
                                .names()
                                .iter()
                                .find_map(|name| std::env::var(name).ok())
                        });

                    match key {
                        Some(key) => {
                            let value = format!(
                                "{}{}",
                                value_prefix.clone().unwrap_or_default(),
                                key
                            );
                            headers.header(header.clone(), value)
                        }
                        None => headers,
                    }
                }
                Some(schematic_define::EnvAuthStrategy::Basic) => {
                    let username = env_mapping
                        .basic_user
                        .as_ref()
                        .and_then(|list| list.names().iter().find_map(|name| std::env::var(name).ok()));
                    let password = env_mapping
                        .basic_pass
                        .as_ref()
                        .and_then(|list| list.names().iter().find_map(|name| std::env::var(name).ok()));

                    match (username, password) {
                        (Some(username), Some(password)) => headers.use_basic_auth(username, password),
                        _ => headers,
                    }
                }
                None => headers,
                _ => headers,
            }
        }

        /// Whether a header env fallback produced its header. An
        /// `ApiKeyParam` fallback is checked by the key it resolves instead.
        fn headers_satisfy_fallback(
            policy: &schematic_define::AuthPolicy,
            headers: &schematic_define::Headers,
        ) -> bool {
            match &policy.env_fallback {
                Some(schematic_define::EnvAuthStrategy::BearerToken { header }) => {
                    header
                        .as_deref()
                        .map_or_else(|| headers.has_authorization(), |header| headers.has_header(header))
                }
                Some(schematic_define::EnvAuthStrategy::ApiKey { header, .. }) => headers.has_header(header),
                Some(schematic_define::EnvAuthStrategy::Basic) => headers.has_authorization(),
                None => false,
                _ => false,
            }
        }

        /// The explicit key set with a client's `with_api_key_param`, sent
        /// where `policy`'s explicit `ApiKeyParam` method says.
        ///
        /// `None` when no key is set or the policy accepts no explicit
        /// parameter key.
        fn explicit_api_key_param(
            policy: &schematic_define::AuthPolicy,
            key: Option<&schematic_define::SensitiveString>,
        ) -> Option<ApiKeyParamCredential> {
            let key = key?;
            policy.explicit.iter().find_map(|method| match method {
                schematic_define::AuthMethod::ApiKeyParam { name, location } => {
                    api_key_param_credential(name, *location, key.clone())
                }
                _ => None,
            })
        }

        /// The env-fallback key: the first non-empty variable among
        /// `env_names`, read only when `policy`'s env fallback is
        /// `ApiKeyParam`. A disabled fallback reads no variable.
        fn env_api_key_param(
            policy: &schematic_define::AuthPolicy,
            env_names: &[String],
        ) -> Option<ApiKeyParamCredential> {
            let Some(schematic_define::EnvAuthStrategy::ApiKeyParam { name, location }) =
                &policy.env_fallback
            else {
                return None;
            };
            let value = env_names
                .iter()
                .find_map(|var| std::env::var(var).ok().filter(|value| !value.is_empty()))?;
            api_key_param_credential(name, *location, schematic_define::SensitiveString::new(value))
        }

        /// A credential for a location the client can send; `None` otherwise,
        /// so an unsendable key reads as no key rather than being dropped.
        fn api_key_param_credential(
            name: &str,
            location: schematic_define::ApiKeyLocation,
            value: schematic_define::SensitiveString,
        ) -> Option<ApiKeyParamCredential> {
            match location {
                schematic_define::ApiKeyLocation::Query
                | schematic_define::ApiKeyLocation::Cookie => Some(ApiKeyParamCredential {
                    name: name.to_string(),
                    location,
                    value,
                }),
                _ => None,
            }
        }

        /// Adds `credential` to the request: a query key replaces every query
        /// pair of the same name (compared case-sensitively) and is appended;
        /// a cookie key is encoded with [`encode_cookie_value`] and pushed to
        /// `cookies` for [`combine_cookie_headers`], which lets it replace a
        /// same-named cookie. Each is encoded once, where its location
        /// requires.
        pub fn apply_api_key_param(
            credential: Option<ApiKeyParamCredential>,
            query: &mut Vec<(String, String)>,
            cookies: &mut Vec<(String, String)>,
        ) {
            let Some(credential) = credential else {
                return;
            };
            match credential.location {
                schematic_define::ApiKeyLocation::Cookie => {
                    cookies.push((credential.name, encode_cookie_value(credential.value.as_str())));
                }
                _ => {
                    query.retain(|(name, _)| name != &credential.name);
                    query.push((credential.name, credential.value.into_inner()));
                }
            }
        }

        /// Parses one header through the HTTP types.
        ///
        /// ## Errors
        ///
        /// Returns `SchematicError::Header` naming the header, never its
        /// value, when the name or the value is invalid.
        pub fn header_pair(
            name: &str,
            value: &str,
        ) -> Result<(reqwest::header::HeaderName, reqwest::header::HeaderValue), SchematicError> {
            let header_name = reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
                SchematicError::Header(schematic_define::HeaderError::InvalidHeaderName(
                    name.to_string(),
                ))
            })?;
            let header_value = reqwest::header::HeaderValue::from_str(value).map_err(|_| {
                SchematicError::Header(schematic_define::HeaderError::InvalidHeaderValue(
                    name.to_string(),
                ))
            })?;
            Ok((header_name, header_value))
        }

        /// The `Debug` output of a generated client: its type name, base URL,
        /// and auth policy. Headers and keys are left out, since they can
        /// hold credentials.
        pub fn debug_client(
            f: &mut std::fmt::Formatter<'_>,
            name: &str,
            base_url: &str,
            auth_policy: &schematic_define::AuthPolicy,
        ) -> std::fmt::Result {
            f.debug_struct(name)
                .field("base_url", &base_url)
                .field("auth_policy", auth_policy)
                .finish_non_exhaustive()
        }
    }
}
