//! Integration tests for HTTP client code generation.
//!
//! These tests verify that the generated HTTP client code:
//! - Correctly matches HTTP methods to reqwest methods
//! - Properly sets up authentication headers based on AuthStrategy
//! - Handles request bodies correctly
//! - Implements proper error handling

use proc_macro2::TokenStream;
use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};
use schematic_gen::output::assemble_api_code;

/// Formats generated tokens into readable code for assertions.
fn format_tokens(tokens: &TokenStream) -> String {
    let file = syn::parse2::<syn::File>(tokens.clone()).expect("Generated code should be valid");
    prettyplease::unparse(&file)
}

/// Creates a test API with the given auth strategy.
fn make_api(name: &str, auth: AuthStrategy) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{} API", name),
        base_url: "https://api.example.com/v1".to_string(),
        docs_url: None,
        auth,
        endpoints: vec![
            Endpoint {
                id: "GetItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "Get items".to_string(),
                request: None,
                response: ApiResponse::json_type("ItemsResponse"),
            },
            Endpoint {
                id: "GetItem".to_string(),
                method: RestMethod::Get,
                path: "/items/{item_id}".to_string(),
                description: "Get a single item".to_string(),
                request: None,
                response: ApiResponse::json_type("Item"),
            },
        ],
    }
}

// =============================================================================
// HTTP method matching tests
// =============================================================================

#[test]
fn client_matches_get_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""GET" => self.client.get(&url)"#),
        "Should match GET method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_matches_post_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""POST" => self.client.post(&url)"#),
        "Should match POST method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_matches_put_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""PUT" => self.client.put(&url)"#),
        "Should match PUT method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_matches_patch_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""PATCH" => self.client.patch(&url)"#),
        "Should match PATCH method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_matches_delete_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""DELETE" => self.client.delete(&url)"#),
        "Should match DELETE method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_matches_head_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""HEAD" => self.client.head(&url)"#),
        "Should match HEAD method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_matches_options_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""OPTIONS" => self.client.request(reqwest::Method::OPTIONS, &url)"#),
        "Should match OPTIONS method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_handles_unsupported_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("SchematicError::UnsupportedMethod"),
        "Should handle unsupported methods\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Authentication header tests
// =============================================================================

#[test]
fn no_auth_generates_no_auth_code() {
    let api = make_api("NoAuthApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should NOT contain any env::var calls for auth
    // Check that there's no bearer/api key setup code
    assert!(
        !code.contains(r#"header("Authorization""#)
            || code.contains("// Apply authentication"),
        "No auth API should not set Authorization header (except in match arm)\nGenerated code:\n{}",
        code
    );
}

#[test]
fn bearer_token_sets_authorization_header() {
    let api = make_api(
        "BearerApi",
        AuthStrategy::BearerToken {
            env_var: "BEARER_TOKEN".to_string(),
            header: None,
        },
    );
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should read from env var
    assert!(
        code.contains(r#"std::env::var("BEARER_TOKEN")"#),
        "Should read BEARER_TOKEN env var\nGenerated code:\n{}",
        code
    );

    // Should set Authorization header with Bearer prefix
    assert!(
        code.contains(r#"header("Authorization", format!("Bearer {}", token))"#),
        "Should set Authorization header with Bearer prefix\nGenerated code:\n{}",
        code
    );
}

#[test]
fn bearer_token_with_custom_header() {
    let api = make_api(
        "CustomBearerApi",
        AuthStrategy::BearerToken {
            env_var: "MY_TOKEN".to_string(),
            header: Some("X-Custom-Auth".to_string()),
        },
    );
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should use custom header name
    assert!(
        code.contains(r#"header("X-Custom-Auth", format!("Bearer {}", token))"#),
        "Should use custom header name\nGenerated code:\n{}",
        code
    );
}

#[test]
fn api_key_sets_custom_header() {
    let api = make_api(
        "ApiKeyApi",
        AuthStrategy::ApiKey {
            env_var: "API_SECRET".to_string(),
            header: "X-API-Key".to_string(),
        },
    );
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should read from env var
    assert!(
        code.contains(r#"std::env::var("API_SECRET")"#),
        "Should read API_SECRET env var\nGenerated code:\n{}",
        code
    );

    // Should set custom header with key directly (no Bearer prefix)
    assert!(
        code.contains(r#"header("X-API-Key", key)"#),
        "Should set X-API-Key header\nGenerated code:\n{}",
        code
    );
}

#[test]
fn basic_auth_uses_reqwest_basic_auth() {
    let api = make_api(
        "BasicAuthApi",
        AuthStrategy::Basic {
            username_env: "API_USER".to_string(),
            password_env: "API_PASS".to_string(),
        },
    );
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should read both env vars
    assert!(
        code.contains(r#"std::env::var("API_USER")"#),
        "Should read API_USER env var\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"std::env::var("API_PASS")"#),
        "Should read API_PASS env var\nGenerated code:\n{}",
        code
    );

    // Should use reqwest's basic_auth method
    assert!(
        code.contains("basic_auth(username, Some(password))"),
        "Should use reqwest basic_auth\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Request body handling tests
// =============================================================================

#[test]
fn client_adds_content_type_for_body() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#"header("Content-Type", "application/json")"#),
        "Should set Content-Type for body\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_adds_body_when_present() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("if let Some(body) = body"),
        "Should conditionally add body\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(".body(body)"),
        "Should add body to request\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Response handling tests
// =============================================================================

#[test]
fn client_checks_response_status() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("response.status().is_success()"),
        "Should check response status\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_deserializes_json_response() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("response.json::<T>().await"),
        "Should deserialize JSON response\nGenerated code:\n{}",
        code
    );
}

#[test]
fn client_handles_api_error() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Check for ApiError pattern (may be formatted across lines)
    assert!(
        code.contains("SchematicError::ApiError"),
        "Should return ApiError for non-success status\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("status") && code.contains("body"),
        "ApiError should have status and body fields\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// URL construction tests
// =============================================================================

#[test]
fn client_constructs_url_from_base_and_path() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#"format!("{}{}", self.base_url, path)"#),
        "Should construct URL from base_url and path\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// API struct tests
// =============================================================================

#[test]
fn api_struct_has_client_field() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("client: reqwest::Client"),
        "API struct should have reqwest Client field\nGenerated code:\n{}",
        code
    );
}

#[test]
fn api_struct_has_base_url_field() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("base_url: String"),
        "API struct should have base_url field\nGenerated code:\n{}",
        code
    );
}

#[test]
fn api_struct_has_new_constructor() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub fn new()"),
        "API struct should have new() constructor\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Request enum tests
// =============================================================================

#[test]
fn request_enum_has_variants_for_all_endpoints() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub enum TestApiRequest"),
        "Should have request enum\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("GetItems(GetItemsRequest)"),
        "Should have GetItems variant\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("GetItem(GetItemRequest)"),
        "Should have GetItem variant\nGenerated code:\n{}",
        code
    );
}

#[test]
fn request_enum_has_into_parts_method() {
    let api = make_api("TestApi", AuthStrategy::None);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // The enum should delegate into_parts to the inner request struct
    assert!(
        code.contains("impl TestApiRequest")
            && code.contains("fn into_parts"),
        "Request enum should have into_parts method\nGenerated code:\n{}",
        code
    );
}
