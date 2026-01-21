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
fn make_api(name: &str, auth: AuthStrategy, env_auth: Vec<String>) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{} API", name),
        base_url: "https://api.example.com/v1".to_string(),
        docs_url: None,
        auth,
        env_auth,
        env_username: None,
        headers: vec![],
        endpoints: vec![
            Endpoint {
                id: "GetItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "Get items".to_string(),
                request: None,
                response: ApiResponse::json_type("ItemsResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "GetItem".to_string(),
                method: RestMethod::Get,
                path: "/items/{item_id}".to_string(),
                description: "Get a single item".to_string(),
                request: None,
                response: ApiResponse::json_type("Item"),
                headers: vec![],
            },
        ],
        module_path: None,
        request_suffix: None,
    }
}

/// Creates a test API with basic auth.
/// Password comes from env_auth[0].
fn make_basic_auth_api(name: &str, username_env: &str, password_env: &str) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{} API", name),
        base_url: "https://api.example.com/v1".to_string(),
        docs_url: None,
        auth: AuthStrategy::Basic,
        env_auth: vec![password_env.to_string()], // Password from env_auth[0]
        env_username: Some(username_env.to_string()),
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "GetItems".to_string(),
            method: RestMethod::Get,
            path: "/items".to_string(),
            description: "Get items".to_string(),
            request: None,
            response: ApiResponse::json_type("ItemsResponse"),
            headers: vec![],
        }],
        module_path: None,
        request_suffix: None,
    }
}

// =============================================================================
// HTTP method matching tests
// =============================================================================

#[test]
fn client_matches_get_method() {
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("NoAuthApi", AuthStrategy::None, vec![]);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should NOT contain any env::var calls for auth
    // Check that there's no bearer/api key setup code
    assert!(
        !code.contains(r#"header("Authorization""#) || code.contains("// Apply authentication"),
        "No auth API should not set Authorization header (except in match arm)\nGenerated code:\n{}",
        code
    );
}

#[test]
fn bearer_token_uses_runtime_auth_matching() {
    let api = make_api(
        "BearerApi",
        AuthStrategy::BearerToken { header: None },
        vec!["BEARER_TOKEN".to_string()],
    );
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should store env var in struct init
    assert!(
        code.contains("BEARER_TOKEN"),
        "Should reference BEARER_TOKEN env var in struct init\nGenerated code:\n{}",
        code
    );

    // Should use runtime match for auth
    assert!(
        code.contains("match &self.auth_strategy"),
        "Should use runtime auth matching\nGenerated code:\n{}",
        code
    );

    // Should handle BearerToken variant
    assert!(
        code.contains("schematic_define::AuthStrategy::BearerToken"),
        "Should handle BearerToken variant\nGenerated code:\n{}",
        code
    );

    // Should format Bearer token at runtime
    assert!(
        code.contains(r#"format!("Bearer {}", token)"#),
        "Should format Bearer prefix\nGenerated code:\n{}",
        code
    );
}

#[test]
fn bearer_token_with_custom_header() {
    let api = make_api(
        "CustomBearerApi",
        AuthStrategy::BearerToken {
            header: Some("X-Custom-Auth".to_string()),
        },
        vec!["MY_TOKEN".to_string()],
    );
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should store custom header in auth_strategy init
    assert!(
        code.contains("X-Custom-Auth"),
        "Should include custom header in auth_strategy init\nGenerated code:\n{}",
        code
    );

    // Runtime handling extracts header via header.as_deref()
    assert!(
        code.contains("header.as_deref()"),
        "Should use header.as_deref() for runtime extraction\nGenerated code:\n{}",
        code
    );
}

#[test]
fn api_key_uses_runtime_auth_matching() {
    let api = make_api(
        "ApiKeyApi",
        AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
        },
        vec!["API_SECRET".to_string()],
    );
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should store env var in struct init
    assert!(
        code.contains("API_SECRET"),
        "Should reference API_SECRET env var in struct init\nGenerated code:\n{}",
        code
    );

    // Should store header name in auth_strategy init
    assert!(
        code.contains("X-API-Key"),
        "Should include header name in auth_strategy\nGenerated code:\n{}",
        code
    );

    // Should handle ApiKey variant at runtime
    assert!(
        code.contains("schematic_define::AuthStrategy::ApiKey"),
        "Should handle ApiKey variant\nGenerated code:\n{}",
        code
    );

    // Runtime uses header.as_str() for header name
    assert!(
        code.contains("header.as_str()"),
        "Should use header.as_str() at runtime\nGenerated code:\n{}",
        code
    );
}

#[test]
fn basic_auth_uses_reqwest_basic_auth() {
    let api = make_basic_auth_api("BasicAuthApi", "API_USER", "API_PASS");
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // Should contain env var names
    assert!(
        code.contains("API_USER"),
        "Should reference API_USER env var\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("API_PASS"),
        "Should reference API_PASS env var\nGenerated code:\n{}",
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
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
    let api = make_api("TestApi", AuthStrategy::None, vec![]);
    let tokens = assemble_api_code(&api);
    let code = format_tokens(&tokens);

    // The enum should delegate into_parts to the inner request struct
    assert!(
        code.contains("impl TestApiRequest") && code.contains("fn into_parts"),
        "Request enum should have into_parts method\nGenerated code:\n{}",
        code
    );
}
