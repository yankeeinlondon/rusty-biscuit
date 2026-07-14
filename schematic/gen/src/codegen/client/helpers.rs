use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

/// Converts a CamelCase identifier to snake_case.
pub(crate) fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Generates auth setup code that reads from struct fields at runtime.
///
/// Returns a TokenStream that resolves headers through the generated auth helper.
pub(crate) fn generate_auth_setup(_api: &RestApi) -> TokenStream {
    quote! { self.resolve_request_headers()? }
}

pub(crate) fn generate_auth_helper_methods() -> TokenStream {
    quote! {
        fn auth_is_required(&self) -> bool {
            !self.auth_policy.explicit.is_empty() || self.auth_policy.env_fallback.is_some()
        }

        fn accepted_explicit_auth_methods(&self) -> Vec<String> {
            self.auth_policy
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

        fn env_fallback_var_names(&self) -> Vec<String> {
            match &self.auth_policy.env_fallback {
                Some(schematic_define::EnvAuthStrategy::BearerToken { .. }) => self
                    .headers
                    .env_mapping()
                    .bearer_token
                    .as_ref()
                    .map(|list| list.names().to_vec())
                    .unwrap_or_default(),
                Some(schematic_define::EnvAuthStrategy::ApiKey { .. }) => self
                    .headers
                    .env_mapping()
                    .api_key
                    .as_ref()
                    .map(|api_key| api_key.names.names().to_vec())
                    .unwrap_or_default(),
                Some(schematic_define::EnvAuthStrategy::Basic) => {
                    let mut vars = Vec::new();
                    if let Some(user) = self.headers.env_mapping().basic_user.as_ref() {
                        vars.extend(user.names().iter().cloned());
                    }
                    if let Some(pass) = self.headers.env_mapping().basic_pass.as_ref() {
                        vars.extend(pass.names().iter().cloned());
                    }
                    vars
                }
                None => Vec::new(),
                _ => Vec::new(),
            }
        }

        fn authentication_required_error(&self) -> SchematicError {
            let explicit_methods = self.accepted_explicit_auth_methods();
            let env_fallback_vars = self.env_fallback_var_names();
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

            if self.auth_policy.oauth2().is_some() {
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

        fn apply_env_fallback(&self,
            headers: schematic_define::Headers,
        ) -> schematic_define::Headers {
            let env_mapping = self.headers.env_mapping().clone();

            match &self.auth_policy.env_fallback {
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

        fn headers_satisfy_fallback(&self,
            headers: &schematic_define::Headers,
        ) -> bool {
            match &self.auth_policy.env_fallback {
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

        fn resolve_request_headers(&self,
        ) -> Result<Vec<(String, String)>, SchematicError> {
            let mut headers = self.headers.clone();

            if !headers.has_explicit_auth() {
                headers = self.apply_env_fallback(headers);
            }

            if self.auth_is_required()
                && !headers.has_explicit_auth()
                && !self.headers_satisfy_fallback(&headers)
            {
                return Err(self.authentication_required_error());
            }

            headers.build().map_err(SchematicError::from)
        }
    }
}

/// Generates convenience methods for non-JSON endpoints.
///
/// For each Binary, Text, or Empty endpoint, generates a named method
/// that provides compile-time type safety and better ergonomics.
///
/// ## Examples
///
/// For a Binary endpoint with id "CreateSpeech":
/// ```text
/// pub async fn create_speech(&self, req: CreateSpeechRequest) -> Result<bytes::Bytes, SchematicError> {
///     self.request_bytes(req).await
/// }
/// ```
pub fn generate_convenience_methods(api: &RestApi, request_suffix: &str) -> TokenStream {
    let methods: Vec<TokenStream> = api
        .endpoints
        .iter()
        .filter(|ep| !ep.response.is_json())
        .map(|ep| {
            let method_name = format_ident!("{}", to_snake_case(&ep.id));
            let request_struct = format_ident!("{}{}", ep.id, request_suffix);
            let doc = format!(" Convenience method for the `{}` endpoint.", ep.id);
            let desc_doc = format!(" {}", ep.description);

            if ep.response.is_binary() {
                quote! {
                    #[doc = #doc]
                    ///
                    #[doc = #desc_doc]
                    #[must_use = "this returns a Future that must be awaited"]
                    pub async fn #method_name(
                        &self,
                        request: #request_struct,
                    ) -> Result<bytes::Bytes, SchematicError> {
                        self.request_bytes(request).await
                    }
                }
            } else if ep.response.is_text() {
                quote! {
                    #[doc = #doc]
                    ///
                    #[doc = #desc_doc]
                    #[must_use = "this returns a Future that must be awaited"]
                    pub async fn #method_name(
                        &self,
                        request: #request_struct,
                    ) -> Result<String, SchematicError> {
                        self.request_text(request).await
                    }
                }
            } else if ep.response.is_empty() {
                quote! {
                    #[doc = #doc]
                    ///
                    #[doc = #desc_doc]
                    #[must_use = "this returns a Future that must be awaited"]
                    pub async fn #method_name(
                        &self,
                        request: #request_struct,
                    ) -> Result<(), SchematicError> {
                        self.request_empty(request).await
                    }
                }
            } else {
                quote! {}
            }
        })
        .collect();

    quote! { #(#methods)* }
}
