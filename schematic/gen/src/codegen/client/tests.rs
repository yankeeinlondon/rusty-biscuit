use super::helpers::{generate_auth_helper_methods, generate_auth_setup, to_snake_case};
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
        auth_policy: None,
        env_auth,
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "ListItems".to_string(),
            method: RestMethod::Get,
            path: "/items".to_string(),
            description: "List items".to_string(),
            request: None,
            response: ApiResponse::json_type("ListItemsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }],
        module_path: None,
        request_suffix: None,
        version: None,
    }
}

fn make_api_with_endpoints(name: &str, endpoints: Vec<Endpoint>) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{} API", name),
        base_url: "https://api.example.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints,
        module_path: None,
        request_suffix: None,
        version: None,
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
        auth_policy: None,
        env_auth: vec![password_env.to_string()], // Password from env_auth[0]
        env_username: Some(username_env.to_string()),
        env_mapping: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "ListItems".to_string(),
            method: RestMethod::Get,
            path: "/items".to_string(),
            description: "List items".to_string(),
            request: None,
            response: ApiResponse::json_type("ListItemsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }],
        module_path: None,
        request_suffix: None,
        version: None,
    }
}

#[test]
fn generate_request_method_no_auth() {
    let api = make_api("NoAuth", AuthStrategy::None, vec![]);
    let tokens = generate_request_method(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Check method signature (now includes Send + Sync + 'static for hook compatibility)
    assert!(code.contains("impl NoAuth"));
    assert!(
        code.contains(
            "pub async fn request<T: serde::de::DeserializeOwned + Send + Sync + 'static>"
        ),
        "Expected new signature with Send + Sync + 'static bounds"
    );
    assert!(code.contains("request: impl Into<NoAuthRequest>"));
    assert!(code.contains("Result<T, SchematicError>"));

    // Check build_and_send_request helper exists and returns tuple
    assert!(code.contains("async fn build_and_send_request"));
    assert!(code.contains("let request = request.into()"));
    assert!(code.contains("let endpoint_id = request.endpoint_id()"));
    assert!(code.contains("let (method, path, body, endpoint_headers) = request.into_parts()?"));
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

    // Check response handling with hook support
    assert!(code.contains("response.status().is_success()"));
    assert!(code.contains("variant_hooks.pre_response_json"));
    // Fast path still uses direct JSON deserialization
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

    assert!(code.contains("fn resolve_request_headers"));
    assert!(code.contains("match &self.auth_policy.env_fallback"));
    assert!(code.contains("schematic_define::EnvAuthStrategy::BearerToken"));
    assert!(code.contains("schematic_define::EnvAuthStrategy::ApiKey"));
    assert!(code.contains("schematic_define::EnvAuthStrategy::Basic"));
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

    assert!(code.contains("self.headers.env_mapping().clone()"));
    assert!(code.contains("EnvAuthStrategy::BearerToken"));
    assert!(code.contains("headers.use_bearer_token(token)"));
    assert!(code.contains("AuthenticationRequired"));
}

#[test]
fn generate_request_method_api_key_uses_self_env_auth() {
    let api = make_api(
        "ApiKey",
        AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
            value_prefix: None,
        },
        vec!["X_API_KEY".to_string()],
    );
    let tokens = generate_request_method(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("EnvAuthStrategy::ApiKey"));
    assert!(code.contains("headers.header(header.clone(), value)"));
    assert!(code.contains("let env_mapping = self.headers.env_mapping().clone();"));
    assert!(code.contains(".api_key"));
    assert!(code.contains("AuthenticationRequired"));
}

#[test]
fn generate_request_method_basic_auth() {
    let api = make_basic_auth_api("BasicAuth", "API_USER", "API_PASS");
    let tokens = generate_request_method(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("EnvAuthStrategy::Basic"));
    assert!(code.contains("headers.env_mapping().basic_user"));
    assert!(code.contains("headers.env_mapping().basic_pass"));
    assert!(code.contains("headers.use_basic_auth(username, password)"));
    assert!(code.contains("AuthenticationRequired"));
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
            value_prefix: None,
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

    assert_eq!(code, "self . resolve_request_headers () ?");
}

#[test]
fn generate_auth_setup_handles_all_strategies() {
    let tokens = generate_auth_helper_methods();
    let code = tokens.to_string();

    assert!(code.contains("EnvAuthStrategy :: BearerToken"));
    assert!(code.contains("EnvAuthStrategy :: ApiKey"));
    assert!(code.contains("EnvAuthStrategy :: Basic"));
    assert!(code.contains("AuthenticationRequired"));
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

    assert!(code.contains("let api_headers = self.resolve_request_headers()?;"));
    assert!(code.contains("merge_headers(&api_headers, &endpoint_headers)"));
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
            params: None,
            oauth_scopes: None,
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
            params: None,
            oauth_scopes: None,
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
            params: None,
            oauth_scopes: None,
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
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "CreateSpeech".to_string(),
                method: RestMethod::Post,
                path: "/speech".to_string(),
                description: "Creates speech audio".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
                params: None,
                oauth_scopes: None,
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
fn generate_request_method_has_must_use() {
    // Test that all async request methods have #[must_use] attribute
    // to warn users if they accidentally discard the returned Future
    let api = make_api_with_endpoints(
        "MustUseApi",
        vec![
            Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "Lists items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "CreateSpeech".to_string(),
                method: RestMethod::Post,
                path: "/speech".to_string(),
                description: "Creates speech audio".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetText".to_string(),
                method: RestMethod::Get,
                path: "/text".to_string(),
                description: "Gets plain text".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "DeleteItem".to_string(),
                method: RestMethod::Delete,
                path: "/items/{id}".to_string(),
                description: "Deletes an item".to_string(),
                request: None,
                response: ApiResponse::Empty,
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
        ],
    );
    let tokens = generate_request_method(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Verify #[must_use] on request<T> (JSON)
    assert!(
        code.contains(r#"#[must_use = "this returns a Future that must be awaited"]"#),
        "Missing #[must_use] attribute on request methods.\nGenerated code:\n{}",
        code
    );

    // Count must_use attributes - should be at least 4 (one for each request method type)
    // plus 3 convenience methods (binary, text, empty)
    let must_use_count = code
        .matches(r#"#[must_use = "this returns a Future that must be awaited"]"#)
        .count();
    assert!(
        must_use_count >= 7,
        "Expected at least 7 #[must_use] attributes (4 request methods + 3 convenience), found {}.\nGenerated code:\n{}",
        must_use_count,
        code
    );

    // Verify each method type has the attribute by checking it appears before the method
    let check_must_use_before = |method_sig: &str| {
        let must_use_attr = r#"#[must_use = "this returns a Future that must be awaited"]"#;
        if let Some(method_pos) = code.find(method_sig) {
            let before_method = &code[..method_pos];
            let last_must_use = before_method.rfind(must_use_attr);
            assert!(
                last_must_use.is_some(),
                "#[must_use] not found before {}\nGenerated code:\n{}",
                method_sig,
                code
            );
        }
    };

    check_must_use_before("pub async fn request<T");
    check_must_use_before("pub async fn request_bytes");
    check_must_use_before("pub async fn request_text");
    check_must_use_before("pub async fn request_empty");
    check_must_use_before("pub async fn create_speech");
    check_must_use_before("pub async fn get_text");
    check_must_use_before("pub async fn delete_item");
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
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetVoiceSampleAudio".to_string(),
                method: RestMethod::Get,
                path: "/audio".to_string(),
                description: "Gets voice sample audio".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
                params: None,
                oauth_scopes: None,
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

#[test]
fn generate_auth_setup_skips_env_check_when_programmatic_auth_set() {
    let api = make_api(
        "ProgrammaticAuth",
        AuthStrategy::BearerToken { header: None },
        vec!["API_TOKEN".to_string()],
    );
    let tokens = generate_request_method(&api);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(
        code.contains("if !headers.has_explicit_auth()"),
        "Should check for explicit auth before env fallback.\nGenerated code:\n{}",
        code
    );
    assert!(code.contains("headers = self.apply_env_fallback(headers);"));
}
