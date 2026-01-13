//! HTTP client method generation for REST APIs.
//!
//! Generates the `request()` method that executes HTTP requests using reqwest,
//! with proper authentication handling based on the API's AuthStrategy.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AuthStrategy, RestApi};

/// Generates the request method for the API struct.
///
/// Creates an async `request()` method that:
/// - Accepts any type that converts into the request enum
/// - Extracts HTTP method, path, and body from the request
/// - Builds the full URL from base_url and path
/// - Applies authentication headers based on AuthStrategy
/// - Sends the request and handles the response
/// - Returns deserialized JSON or an error
///
/// ## Examples
///
/// For an API with BearerToken authentication:
/// ```ignore
/// impl OpenAi {
///     pub async fn request<T: serde::de::DeserializeOwned>(
///         &self,
///         request: impl Into<OpenAiRequest>,
///     ) -> Result<T, SchematicError> {
///         let request = request.into();
///         let (method, path, body) = request.into_parts();
///         let url = format!("{}{}", self.base_url, path);
///
///         let mut req_builder = match method {
///             "GET" => self.client.get(&url),
///             // ... other methods
///         };
///
///         // Apply Bearer token auth
///         if let Ok(token) = std::env::var("OPENAI_API_KEY") {
///             req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
///         }
///
///         // ... send and process response
///     }
/// }
/// ```
pub fn generate_request_method(api: &RestApi) -> TokenStream {
    let struct_name = format_ident!("{}", api.name);
    let request_enum = format_ident!("{}Request", api.name);

    let auth_setup = generate_auth_setup(api);

    quote! {
        impl #struct_name {
            /// Executes an API request.
            ///
            /// Takes any request type that can be converted into the request enum
            /// and returns the deserialized response.
            ///
            /// ## Errors
            ///
            /// Returns an error if:
            /// - The HTTP request fails (network error, timeout, etc.)
            /// - The response indicates a non-success status code
            /// - The response body cannot be deserialized as JSON
            pub async fn request<T: serde::de::DeserializeOwned>(
                &self,
                request: impl Into<#request_enum>,
            ) -> Result<T, SchematicError> {
                let request = request.into();
                let (method, path, body) = request.into_parts()?;
                let url = format!("{}{}", self.base_url, path);

                let mut req_builder = match method {
                    "GET" => self.client.get(&url),
                    "POST" => self.client.post(&url),
                    "PUT" => self.client.put(&url),
                    "PATCH" => self.client.patch(&url),
                    "DELETE" => self.client.delete(&url),
                    "HEAD" => self.client.head(&url),
                    "OPTIONS" => self.client.request(reqwest::Method::OPTIONS, &url),
                    _ => return Err(SchematicError::UnsupportedMethod(method.to_string())),
                };

                // Apply authentication
                #auth_setup

                // Add body if present
                if let Some(body) = body {
                    req_builder = req_builder
                        .header("Content-Type", "application/json")
                        .body(body);
                }

                let response = req_builder.send().await?;

                if !response.status().is_success() {
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    return Err(SchematicError::ApiError { status, body });
                }

                let result = response.json::<T>().await?;
                Ok(result)
            }
        }
    }
}

/// Generates auth setup code based on the AuthStrategy and RestApi credentials.
///
/// Returns the appropriate TokenStream for setting up authentication
/// headers on the request builder. Returns an error if required credentials
/// are missing.
fn generate_auth_setup(api: &RestApi) -> TokenStream {
    match &api.auth {
        AuthStrategy::None => quote! {},
        AuthStrategy::BearerToken { header } => {
            let header_name = header.as_deref().unwrap_or("Authorization");
            let env_vars = &api.env_auth;
            quote! {
                let token = [#(#env_vars),*]
                    .iter()
                    .find_map(|var| std::env::var(var).ok())
                    .ok_or_else(|| SchematicError::MissingCredential {
                        env_vars: vec![#(#env_vars.to_string()),*],
                    })?;
                req_builder = req_builder.header(#header_name, format!("Bearer {}", token));
            }
        }
        AuthStrategy::ApiKey { header } => {
            let env_vars = &api.env_auth;
            quote! {
                let key = [#(#env_vars),*]
                    .iter()
                    .find_map(|var| std::env::var(var).ok())
                    .ok_or_else(|| SchematicError::MissingCredential {
                        env_vars: vec![#(#env_vars.to_string()),*],
                    })?;
                req_builder = req_builder.header(#header, key);
            }
        }
        AuthStrategy::Basic => {
            let username_env = api.env_username.as_deref().unwrap_or("USERNAME");
            let password_env = api.env_password.as_deref().unwrap_or("PASSWORD");
            quote! {
                let username = std::env::var(#username_env)
                    .map_err(|_| SchematicError::MissingCredential {
                        env_vars: vec![#username_env.to_string()],
                    })?;
                let password = std::env::var(#password_env)
                    .map_err(|_| SchematicError::MissingCredential {
                        env_vars: vec![#password_env.to_string()],
                    })?;
                req_builder = req_builder.basic_auth(username, Some(password));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::request_structs::{format_generated_code, validate_generated_code};
    use schematic_define::{ApiResponse, Endpoint, RestMethod};

    fn make_api(name: &str, auth: AuthStrategy, env_auth: Vec<String>) -> RestApi {
        RestApi {
            name: name.to_string(),
            description: format!("{} API", name),
            base_url: "https://api.example.com".to_string(),
            docs_url: None,
            auth,
            env_auth,
            env_username: None,
            env_password: None,
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
            }],
        }
    }

    fn make_basic_auth_api(name: &str, username_env: &str, password_env: &str) -> RestApi {
        RestApi {
            name: name.to_string(),
            description: format!("{} API", name),
            base_url: "https://api.example.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::Basic,
            env_auth: vec![],
            env_username: Some(username_env.to_string()),
            env_password: Some(password_env.to_string()),
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
            }],
        }
    }

    #[test]
    fn generate_request_method_no_auth() {
        let api = make_api("NoAuth", AuthStrategy::None, vec![]);
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check method signature
        assert!(code.contains("impl NoAuth"));
        assert!(code.contains("pub async fn request<T: serde::de::DeserializeOwned>"));
        assert!(code.contains("request: impl Into<NoAuthRequest>"));
        assert!(code.contains("Result<T, SchematicError>"));

        // Check request handling
        assert!(code.contains("let request = request.into()"));
        assert!(code.contains("let (method, path, body) = request.into_parts()?"));
        assert!(code.contains("format!(\"{}{}\", self.base_url, path)"));

        // Check HTTP method matching
        assert!(code.contains(r#""GET" => self.client.get(&url)"#));
        assert!(code.contains(r#""POST" => self.client.post(&url)"#));
        assert!(code.contains(r#""PUT" => self.client.put(&url)"#));
        assert!(code.contains(r#""PATCH" => self.client.patch(&url)"#));
        assert!(code.contains(r#""DELETE" => self.client.delete(&url)"#));
        assert!(code.contains(r#""HEAD" => self.client.head(&url)"#));
        assert!(code.contains(r#""OPTIONS" => self.client.request(reqwest::Method::OPTIONS, &url)"#));

        // Check error handling
        assert!(code.contains("SchematicError::UnsupportedMethod"));
        assert!(code.contains("SchematicError::ApiError"));

        // Check body handling
        assert!(code.contains("if let Some(body) = body"));
        assert!(code.contains(r#"header("Content-Type", "application/json")"#));

        // Check response handling
        assert!(code.contains("response.status().is_success()"));
        assert!(code.contains("response.json::<T>().await"));
    }

    #[test]
    fn generate_request_method_bearer_token_default_header() {
        let api = make_api(
            "Bearer",
            AuthStrategy::BearerToken { header: None },
            vec!["API_TOKEN".to_string()],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check bearer token auth setup
        assert!(code.contains("API_TOKEN"));
        assert!(code.contains(r#"header("Authorization", format!("Bearer {}", token))"#));
        assert!(code.contains("MissingCredential"));
    }

    #[test]
    fn generate_request_method_bearer_token_custom_header() {
        let api = make_api(
            "CustomBearer",
            AuthStrategy::BearerToken {
                header: Some("X-Auth-Token".to_string()),
            },
            vec!["MY_TOKEN".to_string()],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check custom header is used
        assert!(code.contains("MY_TOKEN"));
        assert!(code.contains(r#"header("X-Auth-Token", format!("Bearer {}", token))"#));
    }

    #[test]
    fn generate_request_method_api_key() {
        let api = make_api(
            "ApiKey",
            AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
            },
            vec!["X_API_KEY".to_string()],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check API key auth setup
        assert!(code.contains("X_API_KEY"));
        assert!(code.contains(r#"header("X-API-Key", key)"#));
        assert!(code.contains("MissingCredential"));
    }

    #[test]
    fn generate_request_method_basic_auth() {
        let api = make_basic_auth_api("BasicAuth", "API_USER", "API_PASS");
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check basic auth setup
        assert!(code.contains("API_USER"));
        assert!(code.contains("API_PASS"));
        assert!(code.contains("basic_auth(username, Some(password))"));
        assert!(code.contains("MissingCredential"));
    }

    #[test]
    fn generate_request_method_validates_syntax() {
        let api = make_api("Validated", AuthStrategy::None, vec![]);
        let tokens = generate_request_method(&api);
        assert!(validate_generated_code(&tokens).is_ok());
    }

    #[test]
    fn generate_request_method_all_auth_strategies_validate() {
        // Test no auth
        let api = make_api("Test", AuthStrategy::None, vec![]);
        assert!(validate_generated_code(&generate_request_method(&api)).is_ok());

        // Test bearer token
        let api = make_api(
            "Test",
            AuthStrategy::BearerToken { header: None },
            vec!["TOKEN".to_string()],
        );
        assert!(validate_generated_code(&generate_request_method(&api)).is_ok());

        // Test bearer token with custom header
        let api = make_api(
            "Test",
            AuthStrategy::BearerToken {
                header: Some("Custom-Header".to_string()),
            },
            vec!["TOKEN".to_string()],
        );
        assert!(validate_generated_code(&generate_request_method(&api)).is_ok());

        // Test API key
        let api = make_api(
            "Test",
            AuthStrategy::ApiKey {
                header: "X-Key".to_string(),
            },
            vec!["KEY".to_string()],
        );
        assert!(validate_generated_code(&generate_request_method(&api)).is_ok());

        // Test basic auth
        let api = make_basic_auth_api("Test", "USER", "PASS");
        assert!(validate_generated_code(&generate_request_method(&api)).is_ok());
    }

    #[test]
    fn generate_auth_setup_none() {
        let api = make_api("Test", AuthStrategy::None, vec![]);
        let tokens = generate_auth_setup(&api);
        let code = tokens.to_string();

        // Should produce empty code
        assert!(code.is_empty());
    }

    #[test]
    fn generate_auth_setup_bearer_preserves_env_var_name() {
        let api = make_api(
            "Test",
            AuthStrategy::BearerToken { header: None },
            vec!["MY_SPECIAL_TOKEN".to_string()],
        );
        let tokens = generate_auth_setup(&api);
        let code = tokens.to_string();

        assert!(code.contains("MY_SPECIAL_TOKEN"));
    }

    #[test]
    fn generate_auth_setup_api_key_preserves_header_name() {
        let api = make_api(
            "Test",
            AuthStrategy::ApiKey {
                header: "X-Custom-Header".to_string(),
            },
            vec!["KEY".to_string()],
        );
        let tokens = generate_auth_setup(&api);
        let code = tokens.to_string();

        assert!(code.contains("X-Custom-Header"));
    }

    #[test]
    fn generate_request_method_doc_comments() {
        let api = make_api("Documented", AuthStrategy::None, vec![]);
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check doc comments are present
        assert!(code.contains("Executes an API request"));
        assert!(code.contains("## Errors"));
    }

    #[test]
    fn generate_request_method_no_unwrap_in_error_path() {
        let api = make_api("SafeError", AuthStrategy::None, vec![]);
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // The only unwrap_or_default is for error body text, which is acceptable
        // since it's for error reporting, not control flow
        assert!(code.contains("unwrap_or_default()"));

        // Should not have any unwrap() or expect() that could panic
        let unwrap_count = code.matches(".unwrap()").count();
        let expect_count = code.matches(".expect(").count();
        assert_eq!(unwrap_count, 0, "Should not have .unwrap() calls");
        assert_eq!(expect_count, 0, "Should not have .expect() calls");
    }
}
