//! API struct generation for REST APIs.
//!
//! Generates the main API struct that serves as the client entry point,
//! with constructors and the base URL constant.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

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
    let description = format!("{} client.", api.description);

    quote! {
        #[doc = #description]
        pub struct #struct_name {
            client: reqwest::Client,
            base_url: String,
        }

        impl #struct_name {
            /// Base URL for the API.
            pub const BASE_URL: &'static str = #base_url;

            /// Creates a new API client with the default base URL.
            pub fn new() -> Self {
                Self {
                    client: reqwest::Client::new(),
                    base_url: Self::BASE_URL.to_string(),
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
            env_password: None,
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
}
