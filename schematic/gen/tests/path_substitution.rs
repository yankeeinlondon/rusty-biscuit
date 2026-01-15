//! Integration tests for path parameter substitution in generated code.
//!
//! These tests verify that:
//! - Path parameters are correctly extracted from endpoint paths
//! - Generated request structs have fields for each path parameter
//! - The `into_parts()` method correctly substitutes parameters into the path

use proc_macro2::TokenStream;
use schematic_define::{ApiResponse, Endpoint, RestMethod, Schema};
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
    let endpoint = Endpoint {
        id: "GetUser".to_string(),
        method: RestMethod::Get,
        path: "/users/{user_id}".to_string(),
        description: "Get a user".to_string(),
        request: None,
        response: ApiResponse::json_type("User"),
        headers: vec![],
    };

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
    let endpoint = Endpoint {
        id: "GetUser".to_string(),
        method: RestMethod::Get,
        path: "/users/{user_id}".to_string(),
        description: "Get a user".to_string(),
        request: None,
        response: ApiResponse::json_type("User"),
        headers: vec![],
    };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify format string uses {} placeholder
    assert!(
        code.contains(r#"format!("/users/{}", self.user_id)"#),
        "Should have format string with self.user_id\nGenerated code:\n{}",
        code
    );
    // Verify HTTP method is correct
    assert!(code.contains(r#""GET""#), "Should have GET method");
}

// =============================================================================
// Multiple path parameter tests
// =============================================================================

#[test]
fn multiple_path_params_struct_has_all_fields() {
    let endpoint = Endpoint {
        id: "GetMessage".to_string(),
        method: RestMethod::Get,
        path: "/threads/{thread_id}/messages/{message_id}".to_string(),
        description: "Get a message".to_string(),
        request: None,
        response: ApiResponse::json_type("Message"),
        headers: vec![],
    };

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
    let endpoint = Endpoint {
        id: "GetMessage".to_string(),
        method: RestMethod::Get,
        path: "/threads/{thread_id}/messages/{message_id}".to_string(),
        description: "Get a message".to_string(),
        request: None,
        response: ApiResponse::json_type("Message"),
        headers: vec![],
    };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify format string substitutes all parameters in order
    assert!(
        code.contains(r#"format!("/threads/{}/messages/{}", self.thread_id, self.message_id)"#),
        "Should have format string with both parameters\nGenerated code:\n{}",
        code
    );
}

#[test]
fn three_path_params_into_parts_format_string() {
    let endpoint = Endpoint {
        id: "GetIssueComment".to_string(),
        method: RestMethod::Get,
        path: "/orgs/{org}/repos/{repo}/issues/{issue}/comments".to_string(),
        description: "Get issue comments".to_string(),
        request: None,
        response: ApiResponse::json_type("Comments"),
        headers: vec![],
    };

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
    let endpoint = Endpoint {
        id: "ListItems".to_string(),
        method: RestMethod::Get,
        path: "/items".to_string(),
        description: "List all items".to_string(),
        request: None,
        response: ApiResponse::json_type("ListResponse"),
        headers: vec![],
    };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Struct should exist but have no path param fields
    assert!(
        code.contains("pub struct ListItemsRequest"),
        "Should have struct"
    );

    // Verify path is used directly (no format!)
    assert!(
        code.contains(r#""/items".to_string()"#),
        "Should use path directly without format!\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Path params with request body tests
// =============================================================================

#[test]
fn path_param_with_body_has_both_fields() {
    let endpoint = Endpoint {
        id: "UpdateThread".to_string(),
        method: RestMethod::Patch,
        path: "/threads/{thread_id}".to_string(),
        description: "Update a thread".to_string(),
        request: Some(Schema::new("UpdateThreadBody")),
        response: ApiResponse::json_type("Thread"),
        headers: vec![],
    };

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
        code.contains(r#"format!("/threads/{}", self.thread_id)"#),
        "Should have format string for path\nGenerated code:\n{}",
        code
    );

    // Verify body serialization (now with error handling)
    assert!(
        code.contains("serde_json::to_string(&self.body)"),
        "Should serialize body\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("SerializationError"),
        "Should handle serialization errors\nGenerated code:\n{}",
        code
    );
}

#[test]
fn multiple_path_params_with_body() {
    let endpoint = Endpoint {
        id: "CreateComment".to_string(),
        method: RestMethod::Post,
        path: "/threads/{thread_id}/messages/{message_id}/comments".to_string(),
        description: "Create a comment".to_string(),
        request: Some(Schema::new("CreateCommentBody")),
        response: ApiResponse::json_type("Comment"),
        headers: vec![],
    };

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
fn all_http_methods_generate_correct_method_string() {
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
        let endpoint = Endpoint {
            id: format!("Test{:?}", method),
            method,
            path: "/test/{id}".to_string(),
            description: "Test endpoint".to_string(),
            request: None,
            response: ApiResponse::json_type("Response"),
            headers: vec![],
        };

        let tokens = generate_request_struct(&endpoint);
        let code = format_tokens(&tokens);

        assert!(
            code.contains(&format!(r#""{}""#, expected_str)),
            "Method {:?} should generate string \"{}\"\nGenerated code:\n{}",
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
    let endpoint = Endpoint {
        id: "GetNestedResource".to_string(),
        method: RestMethod::Get,
        path: "/{a}/{b}/{c}".to_string(),
        description: "Consecutive params".to_string(),
        request: None,
        response: ApiResponse::json_type("Response"),
        headers: vec![],
    };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(code.contains("pub a: String"), "Should have a field");
    assert!(code.contains("pub b: String"), "Should have b field");
    assert!(code.contains("pub c: String"), "Should have c field");
    assert!(
        code.contains(r#"format!("/{}/{}/{}", self.a, self.b, self.c)"#),
        "Should have format string\nGenerated code:\n{}",
        code
    );
}

#[test]
fn path_param_at_start() {
    let endpoint = Endpoint {
        id: "GetByVersion".to_string(),
        method: RestMethod::Get,
        path: "/{version}/resource".to_string(),
        description: "Version-prefixed path".to_string(),
        request: None,
        response: ApiResponse::json_type("Response"),
        headers: vec![],
    };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub version: String"),
        "Should have version field"
    );
    assert!(
        code.contains(r#"format!("/{}/resource", self.version)"#),
        "Should have format string\nGenerated code:\n{}",
        code
    );
}

#[test]
fn underscore_in_param_name() {
    let endpoint = Endpoint {
        id: "GetUserProfile".to_string(),
        method: RestMethod::Get,
        path: "/users/{user_id}/profile".to_string(),
        description: "Get user profile".to_string(),
        request: None,
        response: ApiResponse::json_type("Profile"),
        headers: vec![],
    };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Verify snake_case param name is preserved
    assert!(
        code.contains("pub user_id: String"),
        "Should preserve underscore in field name"
    );
}
