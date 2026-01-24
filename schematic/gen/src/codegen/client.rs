//! HTTP client method generation for REST APIs.
//!
//! Generates request methods that execute HTTP requests using reqwest,
//! with proper authentication handling based on the API's AuthStrategy.
//!
//! ## Response Types
//!
//! Different methods are generated based on endpoint response types:
//! - `request<T>()` - For JSON responses (deserializes to type T)
//! - `request_bytes()` - For binary responses (returns `bytes::Bytes`)
//! - `request_text()` - For text responses (returns `String`)
//! - `request_empty()` - For empty responses (returns `()`)

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

/// Generates all request methods for the API struct.
///
/// Analyzes which response types the API uses and generates the appropriate
/// request methods. Also generates convenience methods for non-JSON endpoints.
///
/// ## Generated Methods
///
/// - `request<T>()` - Generated if any endpoint returns JSON
/// - `request_bytes()` - Generated if any endpoint returns Binary
/// - `request_text()` - Generated if any endpoint returns Text
/// - `request_empty()` - Generated if any endpoint returns Empty
///
/// ## Examples
///
/// For an API with mixed response types:
/// ```ignore
/// impl ElevenLabs {
///     // JSON responses
///     pub async fn request<T: serde::de::DeserializeOwned>(...) -> Result<T, SchematicError>
///
///     // Binary responses (audio files)
///     pub async fn request_bytes(...) -> Result<bytes::Bytes, SchematicError>
///
///     // Convenience method for CreateSpeech endpoint
///     pub async fn create_speech(req: CreateSpeechRequest) -> Result<bytes::Bytes, SchematicError>
/// }
/// ```
pub fn generate_request_method(api: &RestApi) -> TokenStream {
    generate_request_method_with_suffix(api, "Request")
}

/// Generates all request methods for the API struct with a custom request suffix.
///
/// This is the same as `generate_request_method` but allows specifying a custom
/// suffix for request struct names (e.g., "BasicRequest" or "BearerRequest").
pub fn generate_request_method_with_suffix(api: &RestApi, request_suffix: &str) -> TokenStream {
    let struct_name = format_ident!("{}", api.name);
    let request_enum = format_ident!("{}Request", api.name);

    // Check which response types the API uses
    let has_json = api.endpoints.iter().any(|e| e.response.is_json());
    let has_binary = api.endpoints.iter().any(|e| e.response.is_binary());
    let has_text = api.endpoints.iter().any(|e| e.response.is_text());
    let has_empty = api.endpoints.iter().any(|e| e.response.is_empty());

    let auth_setup = generate_auth_setup(api);

    // Generate shared helper method
    let build_request_method =
        generate_build_request_method(&struct_name, &request_enum, &auth_setup);

    // Generate merge_headers helper
    let merge_headers_method = generate_merge_headers_method();

    // Generate response-specific methods
    let json_method = if has_json {
        generate_json_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    let bytes_method = if has_binary {
        generate_bytes_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    let text_method = if has_text {
        generate_text_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    let empty_method = if has_empty {
        generate_empty_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    // Generate convenience methods for non-JSON endpoints
    let convenience_methods = generate_convenience_methods(api, request_suffix);

    quote! {
        impl #struct_name {
            #build_request_method
            #merge_headers_method
            #json_method
            #bytes_method
            #text_method
            #empty_method
            #convenience_methods
        }
    }
}

/// Generates the shared build_request helper method.
fn generate_build_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
    auth_setup: &TokenStream,
) -> TokenStream {
    quote! {
        /// Builds and sends an HTTP request, returning the raw response.
        ///
        /// This is an internal helper method used by the public request methods.
        async fn build_and_send_request(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<reqwest::Response, SchematicError> {
            let request = request.into();
            let (method, path, body, endpoint_headers) = request.into_parts()?;
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

            // Merge API-level and endpoint-level headers
            let merged_headers = Self::merge_headers(&self.headers, &endpoint_headers);
            for (key, value) in merged_headers {
                req_builder = req_builder.header(key.as_str(), value.as_str());
            }

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

            Ok(response)
        }
    }
}

/// Generates the merge_headers helper method.
fn generate_merge_headers_method() -> TokenStream {
    quote! {
        /// Merges API-level and endpoint-level headers.
        ///
        /// Endpoint headers override API headers for matching keys (case-insensitive).
        /// Returns a new Vec with the merged headers.
        fn merge_headers(
            api_headers: &[(String, String)],
            endpoint_headers: &[(String, String)],
        ) -> Vec<(String, String)> {
            let mut result: Vec<(String, String)> = Vec::new();

            // Add API headers that don't have endpoint overrides
            for (api_key, api_value) in api_headers {
                let has_override = endpoint_headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case(api_key));
                if !has_override {
                    result.push((api_key.clone(), api_value.clone()));
                }
            }

            // Add all endpoint headers (they take precedence)
            for (key, value) in endpoint_headers {
                result.push((key.clone(), value.clone()));
            }

            result
        }
    }
}

/// Generates the request<T> method for JSON responses.
fn generate_json_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting a JSON response.
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
            let response = self.build_and_send_request(request).await?;
            let result = response.json::<T>().await?;
            Ok(result)
        }
    }
}

/// Generates the request_bytes method for binary responses.
fn generate_bytes_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting a binary response.
        ///
        /// Returns the raw bytes of the response body. Use this for endpoints
        /// that return binary data like audio files, images, or ZIP archives.
        ///
        /// ## Errors
        ///
        /// Returns an error if:
        /// - The HTTP request fails (network error, timeout, etc.)
        /// - The response indicates a non-success status code
        pub async fn request_bytes(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<bytes::Bytes, SchematicError> {
            let response = self.build_and_send_request(request).await?;
            let bytes = response.bytes().await?;
            Ok(bytes)
        }
    }
}

/// Generates the request_text method for text responses.
fn generate_text_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting a plain text response.
        ///
        /// Returns the response body as a String.
        ///
        /// ## Errors
        ///
        /// Returns an error if:
        /// - The HTTP request fails (network error, timeout, etc.)
        /// - The response indicates a non-success status code
        pub async fn request_text(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<String, SchematicError> {
            let response = self.build_and_send_request(request).await?;
            let text = response.text().await?;
            Ok(text)
        }
    }
}

/// Generates the request_empty method for empty responses.
fn generate_empty_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting no response body.
        ///
        /// Use this for endpoints that return 204 No Content or where
        /// the response body should be ignored.
        ///
        /// ## Errors
        ///
        /// Returns an error if:
        /// - The HTTP request fails (network error, timeout, etc.)
        /// - The response indicates a non-success status code
        pub async fn request_empty(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<(), SchematicError> {
            let _response = self.build_and_send_request(request).await?;
            Ok(())
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
/// ```ignore
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

/// Converts a CamelCase identifier to snake_case.
fn to_snake_case(s: &str) -> String {
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
/// Returns a TokenStream that generates a runtime match on `self.auth_strategy`
/// and reads credentials from the appropriate environment variables stored in
/// `self.env_auth` and `self.env_username`.
///
/// This approach allows the `variant()` method to change auth configuration
/// at runtime, rather than being locked in at code generation time.
fn generate_auth_setup(_api: &RestApi) -> TokenStream {
    // Generate runtime auth matching using struct fields
    quote! {
        match &self.auth_strategy {
            schematic_define::AuthStrategy::None => {}
            schematic_define::AuthStrategy::BearerToken { header } => {
                let header_name = header.as_deref().unwrap_or("Authorization");
                let token = self.env_auth
                    .iter()
                    .find_map(|var| std::env::var(var).ok())
                    .ok_or_else(|| SchematicError::MissingCredential {
                        env_vars: self.env_auth.clone(),
                    })?;
                req_builder = req_builder.header(header_name, format!("Bearer {}", token));
            }
            schematic_define::AuthStrategy::ApiKey { header } => {
                let key = self.env_auth
                    .iter()
                    .find_map(|var| std::env::var(var).ok())
                    .ok_or_else(|| SchematicError::MissingCredential {
                        env_vars: self.env_auth.clone(),
                    })?;
                req_builder = req_builder.header(header.as_str(), key);
            }
            schematic_define::AuthStrategy::Basic => {
                // Username from env_username, password from env_auth[0]
                let username_env = self.env_username.as_deref().unwrap_or("USERNAME");
                let password_env = self.env_auth.first().map(String::as_str).unwrap_or("PASSWORD");
                let username = std::env::var(username_env)
                    .map_err(|_| SchematicError::MissingCredential {
                        env_vars: vec![username_env.to_string()],
                    })?;
                let password = std::env::var(password_env)
                    .map_err(|_| SchematicError::MissingCredential {
                        env_vars: vec![password_env.to_string()],
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
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestMethod};

    fn make_api(name: &str, auth: AuthStrategy, env_auth: Vec<String>) -> RestApi {
        RestApi {
            name: name.to_string(),
            description: format!("{} API", name),
            base_url: "https://api.example.com".to_string(),
            docs_url: None,
            auth,
            env_auth,
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
            }],
            module_path: None,
            request_suffix: None,
        }
    }

    fn make_api_with_endpoints(name: &str, endpoints: Vec<Endpoint>) -> RestApi {
        RestApi {
            name: name.to_string(),
            description: format!("{} API", name),
            base_url: "https://api.example.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints,
            module_path: None,
            request_suffix: None,
        }
    }

    /// Creates a basic auth API where password comes from env_auth[0]
    fn make_basic_auth_api(name: &str, username_env: &str, password_env: &str) -> RestApi {
        RestApi {
            name: name.to_string(),
            description: format!("{} API", name),
            base_url: "https://api.example.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::Basic,
            env_auth: vec![password_env.to_string()], // Password from env_auth[0]
            env_username: Some(username_env.to_string()),
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
            }],
            module_path: None,
            request_suffix: None,
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

        // Check build_and_send_request helper exists
        assert!(code.contains("async fn build_and_send_request"));
        assert!(code.contains("let request = request.into()"));
        assert!(
            code.contains("let (method, path, body, endpoint_headers) = request.into_parts()?")
        );
        assert!(code.contains("format!(\"{}{}\", self.base_url, path)"));

        // Check HTTP method matching
        assert!(code.contains(r#""GET" => self.client.get(&url)"#));
        assert!(code.contains(r#""POST" => self.client.post(&url)"#));
        assert!(code.contains(r#""PUT" => self.client.put(&url)"#));
        assert!(code.contains(r#""PATCH" => self.client.patch(&url)"#));
        assert!(code.contains(r#""DELETE" => self.client.delete(&url)"#));
        assert!(code.contains(r#""HEAD" => self.client.head(&url)"#));
        assert!(
            code.contains(r#""OPTIONS" => self.client.request(reqwest::Method::OPTIONS, &url)"#)
        );

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
    fn generate_request_method_uses_runtime_auth_matching() {
        let api = make_api(
            "RuntimeAuth",
            AuthStrategy::BearerToken { header: None },
            vec!["API_TOKEN".to_string()],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check that runtime auth matching is used
        assert!(code.contains("match &self.auth_strategy"));
        assert!(code.contains("schematic_define::AuthStrategy::None"));
        assert!(code.contains("schematic_define::AuthStrategy::BearerToken"));
        assert!(code.contains("schematic_define::AuthStrategy::ApiKey"));
        assert!(code.contains("schematic_define::AuthStrategy::Basic"));
    }

    #[test]
    fn generate_request_method_bearer_uses_self_env_auth() {
        let api = make_api(
            "Bearer",
            AuthStrategy::BearerToken { header: None },
            vec!["API_TOKEN".to_string()],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check that self.env_auth is used at runtime
        assert!(code.contains("self.env_auth"));
        assert!(code.contains(r#"format!("Bearer {}", token)"#));
        assert!(code.contains("MissingCredential"));
    }

    #[test]
    fn generate_request_method_api_key_uses_self_env_auth() {
        let api = make_api(
            "ApiKey",
            AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
            },
            vec!["X_API_KEY".to_string()],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check that self.env_auth is used at runtime
        assert!(code.contains("self.env_auth"));
        assert!(code.contains("header.as_str()"));
        assert!(code.contains("MissingCredential"));
    }

    #[test]
    fn generate_request_method_basic_auth() {
        let api = make_basic_auth_api("BasicAuth", "API_USER", "API_PASS");
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check basic auth setup uses struct fields
        assert!(code.contains("self.env_username"));
        assert!(code.contains("self.env_auth"));
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
    fn generate_auth_setup_produces_runtime_match() {
        let api = make_api("Test", AuthStrategy::None, vec![]);
        let tokens = generate_auth_setup(&api);
        let code = tokens.to_string();

        // Should produce runtime match code
        assert!(code.contains("match & self . auth_strategy"));
    }

    #[test]
    fn generate_auth_setup_handles_all_strategies() {
        let api = make_api("Test", AuthStrategy::None, vec![]);
        let tokens = generate_auth_setup(&api);
        let code = tokens.to_string();

        // Should handle all auth strategy variants
        assert!(code.contains("AuthStrategy :: None"));
        assert!(code.contains("AuthStrategy :: BearerToken"));
        assert!(code.contains("AuthStrategy :: ApiKey"));
        assert!(code.contains("AuthStrategy :: Basic"));
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

        // Should not have any naked unwrap() or expect() that could panic
        // Note: unwrap_or and unwrap_or_default are safe
        let naked_unwrap_count = code.matches(".unwrap()").count();
        let expect_count = code.matches(".expect(").count();
        assert_eq!(
            naked_unwrap_count, 0,
            "Should not have naked .unwrap() calls"
        );
        assert_eq!(expect_count, 0, "Should not have .expect() calls");
    }

    #[test]
    fn generate_request_method_applies_headers() {
        let api = make_api("HeadersApi", AuthStrategy::None, vec![]);
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should call merge_headers and iterate to apply them
        assert!(code.contains("merge_headers(&self.headers, &endpoint_headers)"));
        assert!(code.contains("for (key, value) in merged_headers"));
        assert!(code.contains("req_builder.header(key.as_str(), value.as_str())"));
    }

    #[test]
    fn generate_request_method_has_merge_headers() {
        let api = make_api("MergeApi", AuthStrategy::None, vec![]);
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should have merge_headers helper method
        assert!(code.contains("fn merge_headers"));
        assert!(code.contains("api_headers: &[(String, String)]"));
        assert!(code.contains("endpoint_headers: &[(String, String)]"));
        assert!(code.contains("eq_ignore_ascii_case"));
    }

    // === New tests for response-type-specific methods ===

    #[test]
    fn generate_request_method_binary_endpoint() {
        let api = make_api_with_endpoints(
            "BinaryApi",
            vec![Endpoint {
                id: "CreateSpeech".to_string(),
                method: RestMethod::Post,
                path: "/speech".to_string(),
                description: "Creates speech audio".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
            }],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should have request_bytes method
        assert!(
            code.contains("pub async fn request_bytes"),
            "Missing request_bytes method"
        );
        assert!(
            code.contains("Result<bytes::Bytes, SchematicError>"),
            "Missing bytes::Bytes return type"
        );
        assert!(
            code.contains("response.bytes().await"),
            "Missing bytes() call"
        );

        // Should NOT have request<T> for JSON since no JSON endpoints
        assert!(
            !code.contains("pub async fn request<T"),
            "Should not have request<T> method"
        );

        // Should have convenience method
        assert!(
            code.contains("pub async fn create_speech"),
            "Missing create_speech convenience method"
        );
    }

    #[test]
    fn generate_request_method_text_endpoint() {
        let api = make_api_with_endpoints(
            "TextApi",
            vec![Endpoint {
                id: "GetText".to_string(),
                method: RestMethod::Get,
                path: "/text".to_string(),
                description: "Gets plain text".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
            }],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should have request_text method
        assert!(
            code.contains("pub async fn request_text"),
            "Missing request_text method"
        );
        assert!(
            code.contains("Result<String, SchematicError>"),
            "Missing String return type"
        );
        assert!(
            code.contains("response.text().await"),
            "Missing text() call"
        );

        // Should have convenience method
        assert!(
            code.contains("pub async fn get_text"),
            "Missing get_text convenience method"
        );
    }

    #[test]
    fn generate_request_method_empty_endpoint() {
        let api = make_api_with_endpoints(
            "EmptyApi",
            vec![Endpoint {
                id: "DeleteItem".to_string(),
                method: RestMethod::Delete,
                path: "/items/{id}".to_string(),
                description: "Deletes an item".to_string(),
                request: None,
                response: ApiResponse::Empty,
                headers: vec![],
            }],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should have request_empty method
        assert!(
            code.contains("pub async fn request_empty"),
            "Missing request_empty method"
        );
        assert!(
            code.contains("Result<(), SchematicError>"),
            "Missing () return type"
        );

        // Should have convenience method
        assert!(
            code.contains("pub async fn delete_item"),
            "Missing delete_item convenience method"
        );
    }

    #[test]
    fn generate_request_method_mixed_endpoints() {
        let api = make_api_with_endpoints(
            "MixedApi",
            vec![
                Endpoint {
                    id: "ListItems".to_string(),
                    method: RestMethod::Get,
                    path: "/items".to_string(),
                    description: "Lists items".to_string(),
                    request: None,
                    response: ApiResponse::json_type("ListItemsResponse"),
                    headers: vec![],
                },
                Endpoint {
                    id: "CreateSpeech".to_string(),
                    method: RestMethod::Post,
                    path: "/speech".to_string(),
                    description: "Creates speech audio".to_string(),
                    request: None,
                    response: ApiResponse::Binary,
                    headers: vec![],
                },
            ],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should have BOTH request methods
        assert!(
            code.contains("pub async fn request<T"),
            "Missing request<T> method"
        );
        assert!(
            code.contains("pub async fn request_bytes"),
            "Missing request_bytes method"
        );

        // Should have convenience method for binary endpoint only
        assert!(
            code.contains("pub async fn create_speech"),
            "Missing create_speech convenience method"
        );
        // Should NOT have convenience method for JSON endpoint
        assert!(
            !code.contains("pub async fn list_items"),
            "Should not have list_items convenience method"
        );
    }

    #[test]
    fn generate_convenience_methods_snake_case() {
        let api = make_api_with_endpoints(
            "TestApi",
            vec![
                Endpoint {
                    id: "CreateSpeechWithTimestamps".to_string(),
                    method: RestMethod::Post,
                    path: "/speech".to_string(),
                    description: "Creates speech with timestamps".to_string(),
                    request: None,
                    response: ApiResponse::Binary,
                    headers: vec![],
                },
                Endpoint {
                    id: "GetVoiceSampleAudio".to_string(),
                    method: RestMethod::Get,
                    path: "/audio".to_string(),
                    description: "Gets voice sample audio".to_string(),
                    request: None,
                    response: ApiResponse::Binary,
                    headers: vec![],
                },
            ],
        );
        let tokens = generate_request_method(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check snake_case conversion
        assert!(
            code.contains("pub async fn create_speech_with_timestamps"),
            "Missing create_speech_with_timestamps method"
        );
        assert!(
            code.contains("pub async fn get_voice_sample_audio"),
            "Missing get_voice_sample_audio method"
        );
    }

    #[test]
    fn to_snake_case_converts_correctly() {
        assert_eq!(to_snake_case("CreateSpeech"), "create_speech");
        assert_eq!(
            to_snake_case("GetVoiceSampleAudio"),
            "get_voice_sample_audio"
        );
        assert_eq!(to_snake_case("ListVoices"), "list_voices");
        assert_eq!(to_snake_case("A"), "a");
        assert_eq!(to_snake_case("ABC"), "a_b_c");
    }
}
