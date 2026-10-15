//! Integration tests for path parameter substitution in generated code.
//!
//! These tests verify that:
//! - Path parameters are correctly extracted from endpoint paths
//! - Generated request structs have fields for each path parameter
//! - The `into_parts()` method correctly substitutes parameters into the path

use proc_macro2::TokenStream;
use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestMethod};
use schematic_gen::codegen::generate_request_struct;

/// Formats generated tokens into readable code for assertions.
fn format_tokens(tokens: &TokenStream) -> String {
    let file = syn::parse2::<syn::File>(tokens.clone()).expect("Generated code should be valid");
    prettyplease::unparse(&file)
}

// =============================================================================
// Single path parameter tests
// =============================================================================

#[test]
fn single_path_param_struct_has_field() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetUser".to_string(); fixture.method = RestMethod::Get; fixture.path = "/users/{user_id}".to_string(); fixture.description = "Get a user".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("User"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify struct has path param field
    assert!(
        code.contains("pub user_id: String"),
        "Should have user_id field"
    );
    // Verify struct is named correctly
    assert!(
        code.contains("pub struct GetUserRequest"),
        "Should be named GetUserRequest"
    );
}

#[test]
fn single_path_param_into_parts_format_string() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetUser".to_string(); fixture.method = RestMethod::Get; fixture.path = "/users/{user_id}".to_string(); fixture.description = "Get a user".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("User"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify format string uses {} placeholder
    assert!(
        code.contains("/users/{}"),
        "Should have format string template\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("self.user_id"),
        "Should have self.user_id in format args\nGenerated code:\n{}",
        code
    );
    // Verify HTTP method is correct
    assert!(
        code.contains("const METHOD: schematic_runtime::Method = schematic_runtime::Method::GET;"),
        "Should have GET method\nGenerated code:\n{code}"
    );
}

// =============================================================================
// Multiple path parameter tests
// =============================================================================

#[test]
fn multiple_path_params_struct_has_all_fields() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetMessage".to_string(); fixture.method = RestMethod::Get; fixture.path = "/threads/{thread_id}/messages/{message_id}".to_string(); fixture.description = "Get a message".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Message"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify struct has both path param fields
    assert!(
        code.contains("pub thread_id: String"),
        "Should have thread_id field"
    );
    assert!(
        code.contains("pub message_id: String"),
        "Should have message_id field"
    );
}

#[test]
fn multiple_path_params_into_parts_format_string() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetMessage".to_string(); fixture.method = RestMethod::Get; fixture.path = "/threads/{thread_id}/messages/{message_id}".to_string(); fixture.description = "Get a message".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Message"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify format string substitutes all parameters in order
    // The format may be split across lines by prettyplease
    assert!(
        code.contains("/threads/{}/messages/{}"),
        "Should have format string template\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("self.thread_id") && code.contains("self.message_id"),
        "Should have both params in format args\nGenerated code:\n{}",
        code
    );
}

#[test]
fn three_path_params_into_parts_format_string() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetIssueComment".to_string(); fixture.method = RestMethod::Get; fixture.path = "/orgs/{org}/repos/{repo}/issues/{issue}/comments".to_string(); fixture.description = "Get issue comments".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Comments"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify all three fields exist
    assert!(code.contains("pub org: String"), "Should have org field");
    assert!(code.contains("pub repo: String"), "Should have repo field");
    assert!(
        code.contains("pub issue: String"),
        "Should have issue field"
    );

    // Verify format string (may be split across lines by prettyplease)
    assert!(
        code.contains(r#""/orgs/{}/repos/{}/issues/{}/comments""#),
        "Should have format string template\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("self.org") && code.contains("self.repo") && code.contains("self.issue"),
        "Should have all path params in format args\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// No path parameter tests
// =============================================================================

#[test]
fn no_path_params_no_fields() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "ListItems".to_string(); fixture.method = RestMethod::Get; fixture.path = "/items".to_string(); fixture.description = "List all items".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("ListResponse"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Struct should exist but have no path param fields
    assert!(
        code.contains("pub struct ListItemsRequest"),
        "Should have struct"
    );

    // Verify path is used directly (no format!) - now uses let path since no query params
    assert!(
        code.contains(r#"let path = "/items".to_string()"#),
        "Should use path directly without format!\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Path params with request body tests
// =============================================================================

#[test]
fn path_param_with_body_has_both_fields() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "UpdateThread".to_string(); fixture.method = RestMethod::Patch; fixture.path = "/threads/{thread_id}".to_string(); fixture.description = "Update a thread".to_string(); fixture.request = Some(ApiRequest::json_type("UpdateThreadBody")); fixture.response = ApiResponse::json_type("Thread"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify both path param and body fields exist
    assert!(
        code.contains("pub thread_id: String"),
        "Should have thread_id field"
    );
    assert!(
        code.contains("pub body: UpdateThreadBody"),
        "Should have body field"
    );

    // Verify format string for path
    assert!(
        code.contains("/threads/{}"),
        "Should have format string template\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("self.thread_id"),
        "Should have thread_id in format args\nGenerated code:\n{}",
        code
    );

    // Verify body serialization (now with error handling)
    assert!(
        code.contains("serde_json::to_string(&self.body)"),
        "Should serialize body\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("schematic_runtime::InvalidArgument::new("),
        "Should report an unserializable body as an invalid argument\nGenerated code:\n{}",
        code
    );
}

#[test]
fn multiple_path_params_with_body() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "CreateComment".to_string(); fixture.method = RestMethod::Post; fixture.path = "/threads/{thread_id}/messages/{message_id}/comments".to_string(); fixture.description = "Create a comment".to_string(); fixture.request = Some(ApiRequest::json_type("CreateCommentBody")); fixture.response = ApiResponse::json_type("Comment"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify all fields
    assert!(
        code.contains("pub thread_id: String"),
        "Should have thread_id field"
    );
    assert!(
        code.contains("pub message_id: String"),
        "Should have message_id field"
    );
    assert!(
        code.contains("pub body: CreateCommentBody"),
        "Should have body field"
    );

    // Verify format string (may be split across lines by prettyplease)
    assert!(
        code.contains(r#""/threads/{}/messages/{}/comments""#),
        "Should have format string template\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("self.thread_id") && code.contains("self.message_id"),
        "Should have path params in format args\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// HTTP method tests
// =============================================================================

#[test]
fn all_http_methods_generate_correct_method_constant() {
    let methods_and_expected = [
        (RestMethod::Get, "GET"),
        (RestMethod::Post, "POST"),
        (RestMethod::Put, "PUT"),
        (RestMethod::Patch, "PATCH"),
        (RestMethod::Delete, "DELETE"),
        (RestMethod::Head, "HEAD"),
        (RestMethod::Options, "OPTIONS"),
    ];

    for (method, expected_str) in methods_and_expected {
        let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = format!("Test{:?}", method); fixture.method = method; fixture.path = "/test/{id}".to_string(); fixture.description = "Test endpoint".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Response"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

        let tokens = generate_request_struct(&endpoint);
        let code = format_tokens(&tokens);

        assert!(
            code.contains(&format!(
                "const METHOD: schematic_runtime::Method = schematic_runtime::Method::{expected_str};"
            )),
            "Method {:?} should generate Method::{}\nGenerated code:\n{}",
            method,
            expected_str,
            code
        );
    }
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn consecutive_path_params() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetNestedResource".to_string(); fixture.method = RestMethod::Get; fixture.path = "/{a}/{b}/{c}".to_string(); fixture.description = "Consecutive params".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Response"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(code.contains("pub a: String"), "Should have a field");
    assert!(code.contains("pub b: String"), "Should have b field");
    assert!(code.contains("pub c: String"), "Should have c field");
    assert!(
        code.contains("/{}/{}/{}"),
        "Should have format string template\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("self.a") && code.contains("self.b") && code.contains("self.c"),
        "Should have all params in format args\nGenerated code:\n{}",
        code
    );
}

#[test]
fn path_param_at_start() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetByVersion".to_string(); fixture.method = RestMethod::Get; fixture.path = "/{version}/resource".to_string(); fixture.description = "Version-prefixed path".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Response"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub version: String"),
        "Should have version field"
    );
    assert!(
        code.contains("/{}/resource"),
        "Should have format string template\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("self.version"),
        "Should have version in format args\nGenerated code:\n{}",
        code
    );
}

#[test]
fn underscore_in_param_name() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetUserProfile".to_string(); fixture.method = RestMethod::Get; fixture.path = "/users/{user_id}/profile".to_string(); fixture.description = "Get user profile".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Profile"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify snake_case param name is preserved
    assert!(
        code.contains("pub user_id: String"),
        "Should preserve underscore in field name"
    );
}

// =============================================================================
// Non-identifier path parameter names (sanitization + serde rename)
// =============================================================================

/// Builds a GET endpoint at `path` with no request body.
fn get_endpoint(id: &str, path: &str) -> Endpoint {
    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = RestMethod::Get; fixture.path = path.to_string(); fixture.description = format!("Test endpoint for {id}"); fixture.request = None; fixture.response = ApiResponse::json_type("TestResponse"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }
}

#[test]
fn special_char_path_params_are_sanitized_with_serde_rename() {
    // `{user-id}` and `{2fa}` are not valid Rust identifiers; generation must
    // not panic and must produce sanitized fields that carry the wire name.
    let endpoint = get_endpoint("GetFactor", "/users/{user-id}/factors/{2fa}");
    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub user_id: String"),
        "hyphenated param should become `user_id`:\n{code}"
    );
    assert!(
        code.contains("pub _2fa: String"),
        "digit-leading param should become `_2fa`:\n{code}"
    );
    assert!(
        code.contains(r#"#[serde(rename = "user-id")]"#),
        "must preserve the `user-id` wire name:\n{code}"
    );
    assert!(
        code.contains(r#"#[serde(rename = "2fa")]"#),
        "must preserve the `2fa` wire name:\n{code}"
    );
    // The URL format string keeps both placeholder positions.
    assert!(
        code.contains(r#""/users/{}/factors/{}""#),
        "format string should keep both placeholders:\n{code}"
    );
}

#[test]
fn path_params_are_substituted_without_declared_query_params() {
    // Query parameters are returned as separate pairs; the path holds only the
    // substituted, percent-encoded segments.
    let mut endpoint = get_endpoint("ListIssues", "/repos/{owner}/{repo}/issues");
    endpoint.params = Some(schematic_define::EndpointParams::default().with_query_params(vec![(
        "state",
        schematic_define::QueryParamType::String,
        false,
        None,
    )]));
    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#""/repos/{}/{}/issues""#),
        "path format should hold only path placeholders:\n{code}"
    );
    assert!(
        !code.contains("let mut path") && !code.contains("issues?"),
        "the path must not receive the declared query:\n{code}"
    );
    assert!(
        code.contains(r#"query.push(("state".to_string(), value.to_string()))"#),
        "the declared query should be a separate pair:\n{code}"
    );
}

#[test]
fn path_param_values_are_percent_encoded() {
    // A value such as `release/1.0` must not break out of its path segment, so
    // the emitter wraps each path argument in `urlencoding::encode`.
    let endpoint = get_endpoint("GetRelease", "/repos/{tag}");
    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("urlencoding::encode(& self.tag.to_string())"),
        "path value must be percent-encoded:\n{code}"
    );
}
