//! API struct generation for REST APIs.
//!
//! Generates the main API struct that serves as the client entry point,
//! with constructors and the base URL constant.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AuthStrategy, RestApi};

/// Generates the API struct for the given API definition.
///
/// Creates a struct with:
/// - `BASE_URL` constant containing the API's base URL
/// - `new()` constructor using the default base URL
/// - `with_base_url()` constructor for custom base URLs
/// - `with_client()` constructor for custom reqwest clients
/// - `with_client_and_base_url()` constructor for both custom client and URL
/// - `Default` trait implementation
///
/// ## Examples
///
/// For an API named "OpenAi" with base URL `https://api.openai.com/v1`:
/// ```ignore
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

    // Generate env_username initialization
    let env_username_init = match &api.env_username {
        Some(name) => quote! { Some(#name.to_string()) },
        None => quote! { None },
    };

    // Generate headers initialization
    let headers_init = generate_headers_init(&api.headers);

    quote! {
        #[doc = #description]
        pub struct #struct_name {
            client: reqwest::Client,
            base_url: String,
            /// Environment variable names for authentication credentials.
            env_auth: Vec<String>,
            /// Authentication strategy for this API client.
            auth_strategy: schematic_define::AuthStrategy,
            /// Environment variable for Basic auth username.
            env_username: Option<String>,
            /// Default HTTP headers to include with every request.
            headers: Vec<(String, String)>,
        }

        impl #struct_name {
            /// Base URL for the API.
            pub const BASE_URL: &'static str = #base_url;

            /// Creates a new API client with the default base URL.
            pub fn new() -> Self {
                Self {
                    client: reqwest::Client::new(),
                    base_url: Self::BASE_URL.to_string(),
                    env_auth: vec![#(#env_auth.to_string()),*],
                    auth_strategy: #auth_strategy_init,
                    env_username: #env_username_init,
                    headers: #headers_init,
                }
            }

            /// Creates a new API client with a custom base URL.
            ///
            /// ## Examples
            ///
            /// ```ignore
            /// let client = Api::with_base_url("http://localhost:8080/v1");
            /// ```
            pub fn with_base_url(base_url: impl Into<String>) -> Self {
                Self {
                    client: reqwest::Client::new(),
                    base_url: base_url.into(),
                    env_auth: vec![#(#env_auth.to_string()),*],
                    auth_strategy: #auth_strategy_init,
                    env_username: #env_username_init,
                    headers: #headers_init,
                }
            }

            /// Creates a new API client with a pre-configured reqwest client.
            ///
            /// Use this when you need custom timeouts, connection pools, or middleware.
            ///
            /// ## Examples
            ///
            /// ```ignore
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
                    env_username: #env_username_init,
                    headers: #headers_init,
                }
            }

            /// Creates a new API client with a pre-configured reqwest client and custom base URL.
            ///
            /// ## Examples
            ///
            /// ```ignore
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
                    env_username: #env_username_init,
                    headers: #headers_init,
                }
            }

            /// Creates a variant of this API client with different configuration.
            ///
            /// This method clones the underlying HTTP client and allows customizing:
            /// - Base URL (for proxies, mock servers, or different environments)
            /// - Authentication credentials (different env var names)
            /// - Authentication strategy (via `UpdateStrategy`)
            ///
            /// ## Arguments
            ///
            /// * `base_url` - New base URL for this variant
            /// * `env_auth` - New environment variable names for credentials
            /// * `strategy` - How to update the auth strategy:
            ///   - `UpdateStrategy::NoChange` - Keep current auth strategy
            ///   - `UpdateStrategy::ChangeTo(auth)` - Use specified auth strategy
            ///
            /// ## Examples
            ///
            /// ```ignore
            /// use schematic_define::UpdateStrategy;
            ///
            /// let api = Api::new();
            ///
            /// // Create a variant pointing to a staging server
            /// let staging = api.variant(
            ///     "https://staging.api.com/v1",
            ///     vec!["STAGING_API_KEY".to_string()],
            ///     UpdateStrategy::NoChange,
            /// );
            ///
            /// // Create a variant with different auth
            /// let other = api.variant(
            ///     "https://other.api.com/v1",
            ///     vec!["OTHER_TOKEN".to_string()],
            ///     UpdateStrategy::ChangeTo(schematic_define::AuthStrategy::ApiKey {
            ///         header: "X-API-Key".to_string(),
            ///     }),
            /// );
            /// ```
            pub fn variant(
                &self,
                base_url: impl Into<String>,
                env_auth: Vec<String>,
                strategy: schematic_define::UpdateStrategy,
            ) -> Self {
                let auth_strategy = match strategy {
                    schematic_define::UpdateStrategy::NoChange => self.auth_strategy.clone(),
                    schematic_define::UpdateStrategy::ChangeTo(auth) => auth,
                };
                Self {
                    client: self.client.clone(),
                    base_url: base_url.into(),
                    env_auth,
                    auth_strategy,
                    env_username: self.env_username.clone(),
                    headers: self.headers.clone(),
                }
            }
        }

        impl Default for #struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

/// Generates the initialization code for an AuthStrategy value.
fn generate_auth_strategy_init(auth: &AuthStrategy) -> TokenStream {
    match auth {
        AuthStrategy::None => quote! { schematic_define::AuthStrategy::None },
        AuthStrategy::BearerToken { header } => match header {
            Some(h) => {
                quote! { schematic_define::AuthStrategy::BearerToken { header: Some(#h.to_string()) } }
            }
            None => quote! { schematic_define::AuthStrategy::BearerToken { header: None } },
        },
        AuthStrategy::ApiKey { header } => {
            quote! { schematic_define::AuthStrategy::ApiKey { header: #header.to_string() } }
        }
        AuthStrategy::Basic => quote! { schematic_define::AuthStrategy::Basic },
    }
}

/// Generates the initialization code for the headers Vec.
fn generate_headers_init(headers: &[(String, String)]) -> TokenStream {
    if headers.is_empty() {
        quote! { vec![] }
    } else {
        let header_pairs = headers.iter().map(|(k, v)| {
            quote! { (#k.to_string(), #v.to_string()) }
        });
        quote! { vec![#(#header_pairs),*] }
    }
}

#[cfg(test)]
mod tests {
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
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
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
            env_auth: vec!["BEARER_TOKEN".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
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
            env_auth: vec!["API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
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
            env_auth: vec!["BASIC_PASS".to_string()],
            env_username: Some("BASIC_USER".to_string()),
            headers: vec![],
            endpoints: vec![],
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

        // Check variant() method exists with correct signature
        assert!(code.contains("pub fn variant("));
        assert!(code.contains("&self"));
        assert!(code.contains("base_url: impl Into<String>"));
        assert!(code.contains("env_auth: Vec<String>"));
        assert!(code.contains("strategy: schematic_define::UpdateStrategy"));
    }

    #[test]
    fn variant_method_handles_no_change() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should check for UpdateStrategy::NoChange and clone auth_strategy
        assert!(code.contains("UpdateStrategy::NoChange => self.auth_strategy.clone()"));
    }

    #[test]
    fn variant_method_handles_change_to() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should check for UpdateStrategy::ChangeTo and use new auth
        assert!(code.contains("UpdateStrategy::ChangeTo(auth) => auth"));
    }

    #[test]
    fn variant_method_clones_client() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should clone the client instead of creating new
        assert!(code.contains("client: self.client.clone()"));
    }

    #[test]
    fn variant_method_clones_env_username() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should clone env_username
        assert!(code.contains("env_username: self.env_username.clone()"));
    }

    #[test]
    fn generate_api_struct_has_headers_field() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should have headers field in struct
        assert!(code.contains("headers: Vec<(String, String)>"));
    }

    #[test]
    fn generate_api_struct_with_headers() {
        let api = RestApi {
            name: "HeaderApi".to_string(),
            description: "API with headers".to_string(),
            base_url: "https://api.headers.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            headers: vec![
                ("X-Api-Version".to_string(), "2024-01".to_string()),
                ("X-Custom-Header".to_string(), "custom-value".to_string()),
            ],
            endpoints: vec![],
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
    fn variant_method_clones_headers() {
        let api = make_api("TestApi", "https://api.test.com", "Test API");
        let tokens = generate_api_struct(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should clone headers
        assert!(code.contains("headers: self.headers.clone()"));
    }
}
