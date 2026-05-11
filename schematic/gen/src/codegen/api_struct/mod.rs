//! API struct generation for REST APIs.
//!
//! Generates the main API struct that serves as the client entry point,
//! with constructors and the base URL constant.

mod auth;
mod helpers;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

pub(crate) use auth::{generate_auth_policy_init, generate_auth_strategy_init};
pub(crate) use helpers::{generate_explicit_auth_helpers, generate_headers_init_from_mapping};

/// Generates the API struct for the given API definition.
///
/// Creates a struct with:
/// - `BASE_URL` constant containing the API's base URL
/// - `DOCS_URL` constant containing the API's documentation URL (or `None`)
/// - `new()` constructor using the default base URL
/// - `with_base_url()` constructor for custom base URLs
/// - `with_client()` constructor for custom reqwest clients
/// - `with_client_and_base_url()` constructor for both custom client and URL
/// - `Default` trait implementation
///
/// ## Examples
///
/// For an API named "OpenAi" with base URL `https://api.openai.com/v1`:
/// ```text
/// // Generated code:
/// /// OpenAI API client.
/// pub struct OpenAi {
///     client: reqwest::Client,
///     base_url: String,
/// }
///
/// impl OpenAi {
///     pub const BASE_URL: &'static str = "https://api.openai.com/v1";
///
///     pub fn new() -> Self {
///         Self {
///             client: reqwest::Client::new(),
///             base_url: Self::BASE_URL.to_string(),
///         }
///     }
///
///     pub fn with_base_url(base_url: impl Into<String>) -> Self {
///         Self {
///             client: reqwest::Client::new(),
///             base_url: base_url.into(),
///         }
///     }
/// }
///
/// impl Default for OpenAi {
///     fn default() -> Self {
///         Self::new()
///     }
/// }
/// ```
pub fn generate_api_struct(api: &RestApi) -> TokenStream {
    let struct_name = format_ident!("{}", api.name);
    let base_url = &api.base_url;
    // Leading space for proper /// formatting
    let description = format!(" {} client.", api.description);
    let env_auth = &api.env_auth;

    // Generate auth strategy initialization
    let auth_strategy_init = generate_auth_strategy_init(&api.auth);
    let auth_policy_init = generate_auth_policy_init(&api.effective_auth_policy());

    // Generate env_username initialization
    let env_username_init = match &api.env_username {
        Some(name) => quote! { Some(#name.to_string()) },
        None => quote! { None },
    };

    // Generate headers initialization from env_mapping
    let headers_init = generate_headers_init_from_mapping(api);

    // Generate DOCS_URL constant
    let docs_url_init = match &api.docs_url {
        Some(url) => quote! { Some(#url) },
        None => quote! { None },
    };

    // Generate builder struct name
    let builder_name = format_ident!("{}VariantBuilder", api.name);
    let explicit_auth_helpers = generate_explicit_auth_helpers(api);

    quote! {
        #[doc = #description]
        pub struct #struct_name {
            client: reqwest::Client,
            base_url: String,
            /// Environment variable names for authentication credentials.
            env_auth: Vec<String>,
            /// Authentication strategy for this API client.
            auth_strategy: schematic_define::AuthStrategy,
            /// Effective authentication policy for this API client.
            auth_policy: schematic_define::AuthPolicy,
            /// Environment variable for Basic auth username.
            env_username: Option<String>,
            /// Headers builder with environment variable support for credentials.
            headers: schematic_define::Headers,
            /// Variant-specific hooks for response customization.
            variant_hooks: crate::shared::VariantHooks,
        }

        impl #struct_name {
            /// Base URL for the API.
            pub const BASE_URL: &'static str = #base_url;

            /// Official API documentation URL, if available.
            pub const DOCS_URL: Option<&'static str> = #docs_url_init;

            /// Creates a new API client with the default base URL.
            pub fn new() -> Self {
                Self {
                    client: reqwest::Client::new(),
                    base_url: Self::BASE_URL.to_string(),
                    env_auth: vec![#(#env_auth.to_string()),*],
                    auth_strategy: #auth_strategy_init,
                    auth_policy: #auth_policy_init,
                    env_username: #env_username_init,
                    headers: #headers_init,
                    variant_hooks: crate::shared::VariantHooks::default(),
                }
            }

            /// Creates a new API client with a custom base URL.
            ///
            /// ## Examples
            ///
            /// ```text
            /// let client = Api::with_base_url("http://localhost:8080/v1");
            /// ```
            pub fn with_base_url(base_url: impl Into<String>) -> Self {
                Self {
                    client: reqwest::Client::new(),
                    base_url: base_url.into(),
                    env_auth: vec![#(#env_auth.to_string()),*],
                    auth_strategy: #auth_strategy_init,
                    auth_policy: #auth_policy_init,
                    env_username: #env_username_init,
                    headers: #headers_init,
                    variant_hooks: crate::shared::VariantHooks::default(),
                }
            }

            /// Creates a new API client with a pre-configured reqwest client.
            ///
            /// Use this when you need custom timeouts, connection pools, or middleware.
            ///
            /// ## Examples
            ///
            /// ```text
            /// let custom_client = reqwest::Client::builder()
            ///     .timeout(std::time::Duration::from_secs(60))
            ///     .build()
            ///     .unwrap();
            /// let api = Api::with_client(custom_client);
            /// ```
            pub fn with_client(client: reqwest::Client) -> Self {
                Self {
                    client,
                    base_url: Self::BASE_URL.to_string(),
                    env_auth: vec![#(#env_auth.to_string()),*],
                    auth_strategy: #auth_strategy_init,
                    auth_policy: #auth_policy_init,
                    env_username: #env_username_init,
                    headers: #headers_init,
                    variant_hooks: crate::shared::VariantHooks::default(),
                }
            }

            /// Creates a new API client with a pre-configured reqwest client and custom base URL.
            ///
            /// ## Examples
            ///
            /// ```text
            /// let custom_client = reqwest::Client::builder()
            ///     .timeout(std::time::Duration::from_secs(60))
            ///     .build()
            ///     .unwrap();
            /// let api = Api::with_client_and_base_url(custom_client, "http://localhost:8080");
            /// ```
            pub fn with_client_and_base_url(client: reqwest::Client, base_url: impl Into<String>) -> Self {
                Self {
                    client,
                    base_url: base_url.into(),
                    env_auth: vec![#(#env_auth.to_string()),*],
                    auth_strategy: #auth_strategy_init,
                    auth_policy: #auth_policy_init,
                    env_username: #env_username_init,
                    headers: #headers_init,
                    variant_hooks: crate::shared::VariantHooks::default(),
                }
            }

            /// Creates a variant builder for customizing this API client.
            ///
            /// The builder pattern allows fluent configuration of:
            /// - Base URL (for proxies, mock servers, or different environments)
            /// - Authentication credentials (different env var names)
            /// - Authentication strategy
            /// - Response hooks for JSON transformation and mutation
            ///
            /// ## Examples
            ///
            /// ```text
            /// use schematic_define::UpdateStrategy;
            ///
            /// let api = Api::new();
            ///
            /// // Create a variant pointing to a staging server with a response hook
            /// let staging = api.variant()
            ///     .base_url("https://staging.api.com/v1")
            ///     .env_auth(vec!["STAGING_API_KEY".to_string()])
            ///     .mutate_response::<ListModelsRequest>(|ctx, response| {
            ///         response.data.retain(|m| !m.id.contains("deprecated"));
            ///         Ok(())
            ///     })
            ///     .build();
            /// ```
            #[must_use]
            pub fn variant(&self) -> #builder_name<'_> {
                #builder_name::new(self)
            }

            /// Creates a variant of this API client with different configuration.
            ///
            /// This is a convenience method equivalent to:
            /// ```text
            /// api.variant()
            ///     .base_url(base_url)
            ///     .env_auth(env_auth)
            ///     .auth_update(strategy)
            ///     .build()
            /// ```
            ///
            /// ## Arguments
            ///
            /// * `base_url` - New base URL for this variant
            /// * `env_auth` - New environment variable names for credentials
            /// * `strategy` - How to update the auth strategy:
            ///   - `UpdateStrategy::NoChange` - Keep current auth strategy
            ///   - `UpdateStrategy::ChangeTo(auth)` - Use specified auth strategy
            pub fn variant_with(
                &self,
                base_url: impl Into<String>,
                env_auth: Vec<String>,
                strategy: schematic_define::UpdateStrategy,
            ) -> Self {
                self.variant()
                    .base_url(base_url)
                    .env_auth(env_auth)
                    .auth_update(strategy)
                    .build()
            }

            /// Creates a variant of this API client with custom headers configuration.
            ///
            /// This is a convenience method for creating variants with fully customized
            /// headers, including environment variable mapping for credentials.
            ///
            /// ## Arguments
            ///
            /// * `headers` - Headers builder with environment mapping configured
            ///
            /// ## Examples
            ///
            /// ```text
            /// use schematic_define::{Headers, EnvMapping, EnvList};
            ///
            /// let custom_headers = Headers::default()
            ///     .with_env_mapping(EnvMapping {
            ///         bearer_token: Some(EnvList::single("STAGING_TOKEN")),
            ///         basic_user: None,
            ///         basic_pass: None,
            ///         api_key: None,
            ///     })
            ///     .header("X-Environment", "staging");
            ///
            /// let staging_client = api.variant_with_headers(custom_headers);
            /// ```
            pub fn variant_with_headers(&self, headers: schematic_define::Headers) -> Self {
                self.variant().headers_builder(headers).build()
            }

            /// Returns a reference to the underlying HTTP client.
            ///
            /// Use this for custom requests that aren't covered by the generated methods,
            /// such as paginated endpoints that require query parameters.
            pub fn http_client(&self) -> &reqwest::Client {
                &self.client
            }

            /// Returns the base URL for this API client.
            pub fn api_base_url(&self) -> &str {
                &self.base_url
            }

            /// Returns the effective authentication policy for this client.
            pub fn auth_policy(&self) -> &schematic_define::AuthPolicy {
                &self.auth_policy
            }

            /// Returns OAuth2 provider metadata, if this client accepts OAuth tokens.
            pub fn oauth_config(&self) -> Option<&schematic_define::OAuth2Config> {
                self.auth_policy.oauth2()
            }

            /// Returns the API key header name and value for authentication.
            ///
            /// Returns `None` if the authentication strategy is not `ApiKey`
            /// or if the API key environment variable is not set.
            pub fn api_key_header(&self) -> Option<(String, String)> {
                let header = self
                    .auth_policy
                    .explicit
                    .iter()
                    .find_map(|method| match method {
                        schematic_define::AuthMethod::ApiKey { header } => Some(header.clone()),
                        _ => None,
                    })
                    .or_else(|| {
                        self.headers
                            .env_mapping()
                            .api_key
                            .as_ref()
                            .map(|api_key| api_key.header.clone())
                    });

                header.and_then(|header| {
                    self.headers
                        .env_mapping()
                        .api_key
                        .as_ref()
                        .and_then(|api_key| {
                            api_key
                                .names
                                .names()
                                .iter()
                                .find_map(|env_name| std::env::var(env_name).ok())
                        })
                        .map(|value| (header, value))
                })
            }

            #explicit_auth_helpers
        }

        impl Default for #struct_name {
            fn default() -> Self {
                Self::new()
            }
        }

        /// Builder for creating customized variants of the API client.
        ///
        /// Use [`#struct_name::variant()`] to create a builder, then chain
        /// configuration methods and call [`build()`](Self::build) to create
        /// the variant client.
        pub struct #builder_name<'a> {
            base: &'a #struct_name,
            base_url: Option<String>,
            env_auth: Option<Vec<String>>,
            auth_update: schematic_define::UpdateStrategy,
            headers: Option<schematic_define::Headers>,
            pre_response_json: Option<std::sync::Arc<crate::shared::PreResponseJsonHook>>,
            response_mutators: std::collections::HashMap<
                &'static str,
                std::sync::Arc<dyn crate::shared::AnyResponseMutator>,
            >,
        }

        impl<'a> #builder_name<'a> {
            /// Creates a new variant builder from the base API client.
            fn new(base: &'a #struct_name) -> Self {
                Self {
                    base,
                    base_url: None,
                    env_auth: None,
                    auth_update: schematic_define::UpdateStrategy::NoChange,
                    headers: None,
                    pre_response_json: None,
                    response_mutators: std::collections::HashMap::new(),
                }
            }

            /// Sets the base URL for the variant.
            ///
            /// If not set, the original client's base URL is used.
            #[must_use]
            pub fn base_url(mut self, url: impl Into<String>) -> Self {
                self.base_url = Some(url.into());
                self
            }

            /// Sets the environment variable names for authentication.
            ///
            /// If not set, the original client's env_auth is used.
            #[must_use]
            pub fn env_auth(mut self, env_auth: Vec<String>) -> Self {
                self.env_auth = Some(env_auth);
                self
            }

            /// Sets how the authentication strategy should be updated.
            ///
            /// - `UpdateStrategy::NoChange` - Keep current auth strategy (default)
            /// - `UpdateStrategy::ChangeTo(auth)` - Use specified auth strategy
            #[must_use]
            pub fn auth_update(mut self, strategy: schematic_define::UpdateStrategy) -> Self {
                self.auth_update = strategy;
                self
            }

            /// Sets custom headers for the variant using a Headers builder.
            ///
            /// This replaces the entire headers configuration, including environment
            /// variable mapping. If not set, the original client's headers are used.
            ///
            /// ## Examples
            ///
            /// ```text
            /// use schematic_define::{Headers, EnvMapping, EnvList};
            ///
            /// let custom = Headers::default()
            ///     .with_env_mapping(EnvMapping {
            ///         bearer_token: Some(EnvList::single("STAGING_KEY")),
            ///         basic_user: None,
            ///         basic_pass: None,
            ///         api_key: None,
            ///     })
            ///     .header("X-Custom", "value");
            ///
            /// let variant = api.variant().headers_builder(custom).build();
            /// ```
            #[must_use]
            pub fn headers_builder(mut self, headers: schematic_define::Headers) -> Self {
                self.headers = Some(headers);
                self
            }

            /// Sets a pre-response JSON transformation hook.
            ///
            /// This hook runs after receiving an HTTP response but before
            /// deserializing to the response type. Use it to reshape JSON
            /// payloads to match expected structures.
            ///
            /// ## Examples
            ///
            /// ```text
            /// // Unwrap a { data: ... } envelope
            /// variant.pre_response_json(|ctx, json| {
            ///     Ok(json.get("data").cloned().unwrap_or(json))
            /// })
            /// ```
            #[must_use]
            pub fn pre_response_json<F>(mut self, hook: F) -> Self
            where
                F: Fn(&crate::shared::ResponseContext, serde_json::Value)
                    -> Result<serde_json::Value, crate::shared::SchematicError>
                    + Send
                    + Sync
                    + 'static,
            {
                self.pre_response_json = Some(std::sync::Arc::new(hook));
                self
            }

            /// Registers a response mutation hook for a specific endpoint.
            ///
            /// This hook runs after deserializing the response and can mutate
            /// the response object in place. The endpoint is identified by its
            /// request type.
            ///
            /// ## Examples
            ///
            /// ```text
            /// variant.mutate_response::<ListModelsRequest>(|ctx, response| {
            ///     response.data.retain(|m| !m.id.contains("deprecated"));
            ///     Ok(())
            /// })
            /// ```
            #[must_use]
            pub fn mutate_response<R, F>(mut self, hook: F) -> Self
            where
                R: crate::shared::EndpointSpec,
                R::Response: Send + Sync + 'static,
                F: Fn(&crate::shared::ResponseContext, &mut R::Response)
                    -> Result<(), crate::shared::SchematicError>
                    + Send
                    + Sync
                    + 'static,
            {
                self.response_mutators.insert(
                    R::ENDPOINT_ID,
                    std::sync::Arc::new(crate::shared::TypedMutator::new(hook)),
                );
                self
            }

            /// Builds the variant API client with the configured options.
            ///
            /// Options not explicitly set will inherit from the base client.
            #[must_use]
            pub fn build(self) -> #struct_name {
                let auth_update = self.auth_update.clone();
                let auth_policy = match auth_update.clone() {
                    schematic_define::UpdateStrategy::NoChange => self.base.auth_policy.clone(),
                    schematic_define::UpdateStrategy::ChangeTo(ref auth) => {
                        schematic_define::AuthPolicy::from_auth_strategy(auth)
                    }
                    _ => self.base.auth_policy.clone(),
                };
                let auth_strategy = match auth_update.clone() {
                    schematic_define::UpdateStrategy::NoChange => self.base.auth_strategy.clone(),
                    schematic_define::UpdateStrategy::ChangeTo(auth) => auth,
                    // Handle future variants (non_exhaustive)
                    _ => self.base.auth_strategy.clone(),
                };
                let has_env_auth_override = self.env_auth.is_some();
                let env_auth = self.env_auth.unwrap_or_else(|| self.base.env_auth.clone());
                let headers = match self.headers {
                    Some(headers) => headers,
                    None if has_env_auth_override
                        || !matches!(auth_update, schematic_define::UpdateStrategy::NoChange) =>
                    {
                        self.base.headers.clone().with_env_mapping(
                            schematic_define::RestApi::legacy_env_mapping_for(
                                &auth_strategy,
                                &env_auth,
                                self.base.env_username.as_deref(),
                            ),
                        )
                    }
                    None => self.base.headers.clone(),
                };

                #struct_name {
                    client: self.base.client.clone(),
                    base_url: self.base_url.unwrap_or_else(|| self.base.base_url.clone()),
                    env_auth,
                    auth_strategy,
                    auth_policy,
                    env_username: self.base.env_username.clone(),
                    headers,
                    variant_hooks: crate::shared::VariantHooks {
                        pre_response_json: self.pre_response_json,
                        response_mutators: self.response_mutators,
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::helpers::generate_headers_init;
    use super::*;
    use crate::codegen::request_structs::{format_generated_code, validate_generated_code};
    use schematic_define::AuthStrategy;

    fn make_api(name: &str, base_url: &str, description: &str) -> RestApi {
        RestApi {
            name: name.to_string(),
            description: description.to_string(),
            base_url: base_url.to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        }
    }

    #[test]
    fn generate_api_struct_basic() {
        let api = make_api("OpenAi", "https://api.openai.com/v1", "OpenAI API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check struct definition
        assert!(code.contains("pub struct OpenAi"));
        assert!(code.contains("client: reqwest::Client"));
        assert!(code.contains("base_url: String"));
        assert!(code.contains("env_auth: Vec<String>"));
        assert!(code.contains("auth_strategy: schematic_define::AuthStrategy"));
        assert!(code.contains("env_username: Option<String>"));

        // Check BASE_URL constant
        assert!(code.contains("pub const BASE_URL: &'static str"));
        assert!(code.contains("https://api.openai.com/v1"));

        // Check new() constructor
        assert!(code.contains("pub fn new() -> Self"));
        assert!(code.contains("reqwest::Client::new()"));
        assert!(code.contains("Self::BASE_URL.to_string()"));

        // Check with_base_url() constructor
        assert!(code.contains("pub fn with_base_url(base_url: impl Into<String>) -> Self"));
        assert!(code.contains("base_url.into()"));

        // Check Default impl
        assert!(code.contains("impl Default for OpenAi"));
        assert!(code.contains("Self::new()"));
    }

    #[test]
    fn generate_api_struct_validates_syntax() {
        let api = make_api("TestApi", "https://example.com/api", "Test API");
        let tokens = generate_api_struct(&api);
        assert!(validate_generated_code(&tokens).is_ok());
    }

    #[test]
    fn generate_api_struct_with_different_names() {
        let test_cases = [
            ("Gemini", "https://generativelanguage.googleapis.com"),
            ("Anthropic", "https://api.anthropic.com/v1"),
            ("GitHub", "https://api.github.com"),
        ];

        for (name, base_url) in test_cases {
            let api = make_api(name, base_url, &format!("{} API", name));
            let tokens = generate_api_struct(&api);
            let code = format_generated_code(&tokens).expect("Failed to format code");

            assert!(
                code.contains(&format!("pub struct {}", name)),
                "Expected struct {} in generated code",
                name
            );
            assert!(
                code.contains(base_url),
                "Expected BASE_URL {} in generated code",
                base_url
            );
        }
    }

    #[test]
    fn generate_api_struct_doc_comment_includes_description() {
        let api = make_api("Custom", "https://api.custom.com", "Custom Service API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Doc comment should include the description
        assert!(code.contains("Custom Service API client"));
    }

    #[test]
    fn generate_api_struct_with_special_url_characters() {
        let api = make_api(
            "SpecialApi",
            "https://api.example.com:8443/v2/beta",
            "API with port and path",
        );
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("https://api.example.com:8443/v2/beta"));
    }

    #[test]
    fn generate_api_struct_has_with_client_constructor() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check with_client() constructor
        assert!(code.contains("pub fn with_client(client: reqwest::Client) -> Self"));
        assert!(code.contains("Self::BASE_URL.to_string()"));
    }

    #[test]
    fn generate_api_struct_has_with_client_and_base_url_constructor() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check with_client_and_base_url() constructor
        assert!(code.contains("pub fn with_client_and_base_url"));
        assert!(code.contains("client: reqwest::Client"));
        assert!(code.contains("base_url: impl Into<String>"));
    }

    #[test]
    fn generate_api_struct_with_bearer_auth() {
        let api = RestApi {
            name: "BearerApi".to_string(),
            description: "Bearer Auth API".to_string(),
            base_url: "https://api.bearer.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["BEARER_TOKEN".to_string()],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        };
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("schematic_define::AuthStrategy::BearerToken"));
        assert!(code.contains("BEARER_TOKEN"));
    }

    #[test]
    fn generate_api_struct_with_api_key_auth() {
        let api = RestApi {
            name: "ApiKeyApi".to_string(),
            description: "API Key Auth API".to_string(),
            base_url: "https://api.apikey.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
            },
            auth_policy: None,
            env_auth: vec!["API_KEY".to_string()],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        };
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("schematic_define::AuthStrategy::ApiKey"));
        assert!(code.contains("X-API-Key"));
        assert!(code.contains("API_KEY"));
    }

    #[test]
    fn generate_api_struct_with_basic_auth() {
        let api = RestApi {
            name: "BasicApi".to_string(),
            description: "Basic Auth API".to_string(),
            base_url: "https://api.basic.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::Basic,
            auth_policy: None,
            env_auth: vec!["BASIC_PASS".to_string()],
            env_username: Some("BASIC_USER".to_string()),
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        };
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("schematic_define::AuthStrategy::Basic"));
        assert!(code.contains("BASIC_PASS"));
        assert!(code.contains("BASIC_USER"));
    }

    #[test]
    fn generate_auth_strategy_init_none() {
        let tokens = generate_auth_strategy_init(&AuthStrategy::None);
        let code = tokens.to_string();
        assert!(code.contains("AuthStrategy :: None"));
    }

    #[test]
    fn generate_auth_strategy_init_bearer_without_header() {
        let tokens = generate_auth_strategy_init(&AuthStrategy::BearerToken { header: None });
        let code = tokens.to_string();
        assert!(code.contains("AuthStrategy :: BearerToken"));
        assert!(code.contains("header : None"));
    }

    #[test]
    fn generate_auth_strategy_init_bearer_with_header() {
        let tokens = generate_auth_strategy_init(&AuthStrategy::BearerToken {
            header: Some("X-Custom".to_string()),
        });
        let code = tokens.to_string();
        assert!(code.contains("AuthStrategy :: BearerToken"));
        assert!(code.contains("X-Custom"));
    }

    #[test]
    fn generate_auth_strategy_init_api_key() {
        let tokens = generate_auth_strategy_init(&AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
        });
        let code = tokens.to_string();
        assert!(code.contains("AuthStrategy :: ApiKey"));
        assert!(code.contains("X-API-Key"));
    }

    #[test]
    fn generate_auth_strategy_init_basic() {
        let tokens = generate_auth_strategy_init(&AuthStrategy::Basic);
        let code = tokens.to_string();
        assert!(code.contains("AuthStrategy :: Basic"));
    }

    #[test]
    fn generate_api_struct_has_variant_method() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check variant() method returns a builder
        assert!(code.contains("pub fn variant(&self) -> TestApiVariantBuilder"));
        assert!(code.contains("TestApiVariantBuilder::new(self)"));
    }

    #[test]
    fn generate_api_struct_has_variant_with_method() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check variant_with() convenience method exists
        assert!(code.contains("pub fn variant_with("));
        assert!(code.contains("base_url: impl Into<String>"));
        assert!(code.contains("env_auth: Vec<String>"));
        assert!(code.contains("strategy: schematic_define::UpdateStrategy"));
    }

    #[test]
    fn variant_builder_handles_no_change() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check builder's build() method handles UpdateStrategy::NoChange
        assert!(code.contains("UpdateStrategy::NoChange => self.base.auth_strategy.clone()"));
    }

    #[test]
    fn variant_builder_handles_change_to() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check builder's build() method handles UpdateStrategy::ChangeTo
        assert!(code.contains("UpdateStrategy::ChangeTo(auth) => auth"));
    }

    #[test]
    fn variant_builder_clones_client() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Builder's build() should clone the client
        assert!(code.contains("client: self.base.client.clone()"));
    }

    #[test]
    fn variant_builder_clones_env_username() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Builder's build() should clone env_username
        assert!(code.contains("env_username: self.base.env_username.clone()"));
    }

    #[test]
    fn generate_api_struct_has_headers_field() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should have headers field in struct (now uses Headers type)
        assert!(code.contains("headers: schematic_define::Headers"));
    }

    #[test]
    fn generate_api_struct_with_headers() {
        let api = RestApi {
            name: "HeaderApi".to_string(),
            description: "API with headers".to_string(),
            base_url: "https://api.headers.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![
                ("X-Api-Version".to_string(), "2024-01".to_string()),
                ("X-Custom-Header".to_string(), "custom-value".to_string()),
            ],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        };
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should contain the header keys and values
        assert!(code.contains("X-Api-Version"));
        assert!(code.contains("2024-01"));
        assert!(code.contains("X-Custom-Header"));
        assert!(code.contains("custom-value"));
    }

    #[test]
    fn generate_headers_init_empty() {
        let headers: Vec<(String, String)> = vec![];
        let tokens = generate_headers_init(&headers);
        let code = tokens.to_string();
        assert!(code.contains("vec !"));
    }

    #[test]
    fn generate_headers_init_with_values() {
        let headers = vec![
            ("Header-One".to_string(), "value-one".to_string()),
            ("Header-Two".to_string(), "value-two".to_string()),
        ];
        let tokens = generate_headers_init(&headers);
        let code = tokens.to_string();
        assert!(code.contains("Header-One"));
        assert!(code.contains("value-one"));
        assert!(code.contains("Header-Two"));
        assert!(code.contains("value-two"));
    }

    #[test]
    fn variant_builder_clones_headers() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Builder's build() should clone headers via unwrap_or_else
        assert!(code.contains("self.base.headers.clone()"));
    }

    #[test]
    fn variant_builder_checks_env_override_before_consuming_option() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("let has_env_auth_override = self.env_auth.is_some();"));
        assert!(code.contains("let env_auth = self.env_auth.unwrap_or_else"));
        assert!(code.contains("None if has_env_auth_override"));
    }

    #[test]
    fn variant_builder_has_mutate_response_method() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check mutate_response method exists
        assert!(code.contains("pub fn mutate_response<R, F>(mut self, hook: F) -> Self"));
        assert!(code.contains("R: crate::shared::EndpointSpec"));
    }

    #[test]
    fn variant_builder_has_pre_response_json_method() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check pre_response_json method exists
        assert!(code.contains("pub fn pre_response_json<F>(mut self, hook: F) -> Self"));
    }

    #[test]
    fn generate_api_struct_has_docs_url_constant() {
        let api = RestApi {
            name: "DocsApi".to_string(),
            description: "API with documentation URL".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            docs_url: Some("https://docs.example.com/api".to_string()),
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        };
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check DOCS_URL constant exists and is Some
        assert!(
            code.contains("pub const DOCS_URL: Option<&'static str>"),
            "Expected DOCS_URL constant declaration"
        );
        assert!(
            code.contains("Some(\"https://docs.example.com/api\")"),
            "Expected DOCS_URL to contain the documentation URL"
        );
    }

    #[test]
    fn generate_api_struct_docs_url_none() {
        let api = make_api(
            "NoDocsApi",
            "https://api.nodocs.com",
            "API without documentation",
        );
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check DOCS_URL constant exists and is None
        assert!(
            code.contains("pub const DOCS_URL: Option<&'static str>"),
            "Expected DOCS_URL constant declaration"
        );
        assert!(
            code.contains("DOCS_URL: Option<&'static str> = None"),
            "Expected DOCS_URL to be None when docs_url is not provided"
        );
    }
}
