//! Tests for query parameter code generation in generated request structs.
//!
//! These tests verify the generated `into_parts()` method correctly builds
//! query strings with proper URL encoding and separator handling.
//!
//! ## Test Categories
//!
//! 1. **Separator tests**: First param uses `?`, subsequent use `&`
//! 2. **URL encoding tests**: Values with special characters are encoded
//! 3. **Type-specific tests**: Boolean, enum, array params generate correctly
//! 4. **Interaction tests**: Query params with path params

use proc_macro2::TokenStream;
use schematic_define::{
    ApiResponse, Endpoint, EndpointParams, ParamDef, ParamStyle, QueryParamType, RestMethod,
};
use schematic_gen::codegen::generate_request_struct;

/// Formats generated tokens into readable code for assertions.
fn format_tokens(tokens: &TokenStream) -> String {
    let file = syn::parse2::<syn::File>(tokens.clone()).expect("Generated code should be valid");
    prettyplease::unparse(&file)
}

/// Helper to create an endpoint with query parameters.
fn make_endpoint_with_query(
    id: &str,
    method: RestMethod,
    path: &str,
    query_params: Vec<(&str, QueryParamType)>,
) -> Endpoint {
    let params = if query_params.is_empty() {
        None
    } else {
        Some(EndpointParams {
            query: query_params
                .into_iter()
                .map(|(name, param_type)| ParamDef {
                    name: name.to_string(),
                    required: false,
                    description: None,
                    param_type,
                    explode: false,
                    style: ParamStyle::Form,
                })
                .collect(),
            header: vec![],
            cookie: vec![],
            pagination: None,
            response_pagination: None,
        })
    };

    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: format!("Test endpoint for {}", id),
        request: None,
        response: ApiResponse::json_type("TestResponse"),
        headers: vec![],
        params,
        oauth_scopes: None,
    }
}

// =============================================================================
// Separator tests: Collector pattern with single ?/& check at the end
// =============================================================================

#[test]
fn single_query_param_uses_question_mark() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Should use collector pattern with single ?/& check
    assert!(
        code.contains("let mut query_pairs: Vec<(&str, String)> = Vec::new()"),
        "Should use query_pairs collector\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"if path.contains('?')"#),
        "Should check if path contains '?' at the end\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"path.push_str(&format!("?{}", query_string))"#),
        "Should use '?' for query string\nGenerated code:\n{}",
        code
    );
}

#[test]
fn multiple_query_params_uses_correct_separators() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![
            ("page", QueryParamType::Integer),
            ("limit", QueryParamType::Integer),
            ("sort", QueryParamType::String),
        ],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Each param should push to query_pairs
    assert!(
        code.contains(r#"query_pairs.push(("page", value.to_string()))"#),
        "Page param should push to query_pairs\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"query_pairs.push(("limit", value.to_string()))"#),
        "Limit param should push to query_pairs\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"query_pairs.push(("sort", value.to_string()))"#),
        "Sort param should push to query_pairs\nGenerated code:\n{}",
        code
    );

    // Query string construction with URL encoding
    assert!(
        code.contains("urlencoding::encode(v)"),
        "Should URL-encode values\nGenerated code:\n{}",
        code
    );

    // Single ?/& check at the end
    assert!(
        code.contains(r#"if path.contains('?')"#),
        "Should have single '?' check at end\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"path.push_str(&format!("&{}", query_string))"#),
        "Should use '&' when path has '?'\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"path.push_str(&format!("?{}", query_string))"#),
        "Should use '?' when path has no '?'\nGenerated code:\n{}",
        code
    );
}

#[test]
fn query_params_with_existing_path_query_uses_ampersand() {
    // This test documents the behavior when path already has hardcoded query params
    // (The path normalization should happen before code generation, but the generated
    // code handles it by checking if '?' is already in the path)
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items", // Clean path - no hardcoded query
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Generated code should handle both cases with single check at end
    assert!(
        code.contains(r#"if path.contains('?')"#),
        "Should check for existing '?' at end\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"} else {"#),
        "Should have else branch for '?'\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Type-specific tests: Boolean, enum, integer params
// =============================================================================

#[test]
fn boolean_query_param_generates_correctly() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![("active", QueryParamType::Boolean)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Should have bool field type
    assert!(
        code.contains("pub active: Option<bool>"),
        "Should have Option<bool> field\nGenerated code:\n{}",
        code
    );

    // Should have builder method accepting bool
    assert!(
        code.contains("pub fn with_active(mut self, value: bool) -> Self"),
        "Should have with_active builder method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn enum_query_param_generates_as_string() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![(
            "status",
            QueryParamType::Enum(vec!["active".into(), "inactive".into(), "pending".into()]),
        )],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Enum params are currently generated as String (may change in future)
    assert!(
        code.contains("pub status: Option<String>"),
        "Enum should be Option<String>\nGenerated code:\n{}",
        code
    );
}

#[test]
fn integer_query_param_generates_correctly() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub page: Option<i64>"),
        "Should have Option<i64> field\nGenerated code:\n{}",
        code
    );

    assert!(
        code.contains("pub fn with_page(mut self, value: i64) -> Self"),
        "Should have with_page builder method with i64\nGenerated code:\n{}",
        code
    );
}

#[test]
fn number_query_param_generates_correctly() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![("score", QueryParamType::Number)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub score: Option<f64>"),
        "Should have Option<f64> field\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Array parameter tests
// =============================================================================

#[test]
fn array_query_param_generates_vec_field() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![(
            "tags",
            QueryParamType::Array(Box::new(QueryParamType::String)),
        )],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub tags: Option<Vec<String>>"),
        "Should have Option<Vec<String>> field\nGenerated code:\n{}",
        code
    );
}

#[test]
fn array_query_param_generates_loop_for_values() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![(
            "tags",
            QueryParamType::Array(Box::new(QueryParamType::String)),
        )],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Array params should generate a loop that pushes each value to collector
    assert!(
        code.contains("for value in values"),
        "Should iterate over array values\nGenerated code:\n{}",
        code
    );

    // Each array value should push to query_pairs
    assert!(
        code.contains(r#"query_pairs.push(("tags", value.to_string()))"#),
        "Array values should push to query_pairs\nGenerated code:\n{}",
        code
    );
}

#[test]
fn array_of_integers_generates_vec_i64() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![(
            "ids",
            QueryParamType::Array(Box::new(QueryParamType::Integer)),
        )],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub ids: Option<Vec<i64>>"),
        "Should have Option<Vec<i64>> field\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Query params with path params
// =============================================================================

#[test]
fn query_params_with_path_params_generates_both() {
    let endpoint = make_endpoint_with_query(
        "ListRepoIssues",
        RestMethod::Get,
        "/repos/{owner}/{repo}/issues",
        vec![
            ("state", QueryParamType::String),
            ("page", QueryParamType::Integer),
        ],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Path params
    assert!(
        code.contains("pub owner: String"),
        "Should have owner path param\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("pub repo: String"),
        "Should have repo path param\nGenerated code:\n{}",
        code
    );

    // Query params
    assert!(
        code.contains("pub state: Option<String>"),
        "Should have state query param\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("pub page: Option<i64>"),
        "Should have page query param\nGenerated code:\n{}",
        code
    );
}

#[test]
fn path_uses_mut_when_query_params_present() {
    let endpoint = make_endpoint_with_query(
        "ListRepoIssues",
        RestMethod::Get,
        "/repos/{owner}/{repo}/issues",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Path should be mutable when query params need to be appended
    assert!(
        code.contains("let mut path = format!"),
        "Path should be mutable when query params present\nGenerated code:\n{}",
        code
    );
}

#[test]
fn path_immutable_when_no_query_params() {
    let endpoint = Endpoint {
        id: "GetUser".to_string(),
        method: RestMethod::Get,
        path: "/users/{user_id}".to_string(),
        description: "Get a user".to_string(),
        request: None,
        response: ApiResponse::json_type("User"),
        headers: vec![],
        params: None,
        oauth_scopes: None,
    };

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Path should NOT be mutable when no query params
    assert!(
        code.contains("let path = format!"),
        "Path should be immutable when no query params\nGenerated code:\n{}",
        code
    );
    assert!(
        !code.contains("let mut path"),
        "Should NOT have mut path when no query params\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Builder method tests
// =============================================================================

#[test]
fn query_param_builder_methods_return_self() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![
            ("page", QueryParamType::Integer),
            ("limit", QueryParamType::Integer),
        ],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Builder methods should return Self for chaining
    assert!(
        code.contains("pub fn with_page(mut self, value: i64) -> Self"),
        "with_page should return Self\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("pub fn with_limit(mut self, value: i64) -> Self"),
        "with_limit should return Self\nGenerated code:\n{}",
        code
    );

    // Builder methods should set the field
    assert!(
        code.contains("self.page = Some(value)"),
        "with_page should set self.page\nGenerated code:\n{}",
        code
    );
}

#[test]
fn query_param_builder_for_array_accepts_vec() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![(
            "tags",
            QueryParamType::Array(Box::new(QueryParamType::String)),
        )],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub fn with_tags(mut self, value: Vec<String>) -> Self"),
        "with_tags should accept Vec<String>\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// into_parts() return type tests
// =============================================================================

#[test]
fn into_parts_returns_result_with_query_params() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub fn into_parts(self) -> Result<RequestParts, SchematicError>"),
        "into_parts should return Result<RequestParts, SchematicError>\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Default and new() with query params
// =============================================================================

#[test]
fn query_only_endpoint_has_new_method() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Query-only endpoints should have a new() method
    assert!(
        code.contains("pub fn new() -> Self"),
        "Query-only endpoint should have new() method\nGenerated code:\n{}",
        code
    );
}

#[test]
fn query_params_initialized_to_none_in_new() {
    let endpoint = make_endpoint_with_query(
        "ListRepoIssues",
        RestMethod::Get,
        "/repos/{owner}/{repo}/issues",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // new() should initialize query params to None
    assert!(
        code.contains("page: None"),
        "Query params should be initialized to None in new()\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// From impls with query params
// =============================================================================

#[test]
fn single_path_param_with_query_params_has_from_impls() {
    let endpoint = make_endpoint_with_query(
        "GetModel",
        RestMethod::Get,
        "/models/{model}",
        vec![("version", QueryParamType::String)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Should still have From<&str> and From<String> for single path param
    // even with query params (query params are initialized to None)
    assert!(
        code.contains("impl From<&str> for GetModelRequest"),
        "Should have From<&str> impl\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("impl From<String> for GetModelRequest"),
        "Should have From<String> impl\nGenerated code:\n{}",
        code
    );

    // The From impl should initialize query params to None
    assert!(
        code.contains("version: None"),
        "From impl should initialize query params to None\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// URL encoding tests
// =============================================================================

#[test]
fn generated_code_uses_urlencoding_for_query_values() {
    let endpoint = make_endpoint_with_query(
        "Search",
        RestMethod::Get,
        "/search",
        vec![("q", QueryParamType::String)],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Should use urlencoding::encode for values
    assert!(
        code.contains("urlencoding::encode(v)"),
        "Should use urlencoding::encode for values\nGenerated code:\n{}",
        code
    );

    // Should format encoded_key=encoded_value
    assert!(
        code.contains(
            r#"format!("{}={}", urlencoding::encode(k), urlencoding::encode(v))"#
        ),
        "Should format encoded_key=encoded_value\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Doc comment tests
// =============================================================================

#[test]
fn query_params_in_example_use_builder_pattern() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![
            ("page", QueryParamType::Integer),
            ("limit", QueryParamType::Integer),
        ],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    // Example should show builder pattern for query params
    assert!(
        code.contains(".with_page(/* value */)") || code.contains(".with_page"),
        "Example should show with_page builder\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(".with_limit(/* value */)") || code.contains(".with_limit"),
        "Example should show with_limit builder\nGenerated code:\n{}",
        code
    );
}

// =============================================================================
// Non-identifier query parameter names (sanitization + serde rename)
// =============================================================================

#[test]
fn special_char_query_params_are_sanitized_with_serde_rename() {
    // `$top` and `user.name` are not valid Rust identifiers; generation must
    // not panic, and the wire names must be preserved via serde rename and the
    // query-string key.
    let endpoint = make_endpoint_with_query(
        "SearchItems",
        RestMethod::Get,
        "/items",
        vec![
            ("$top", QueryParamType::Integer),
            ("user.name", QueryParamType::String),
        ],
    );

    let tokens = generate_request_struct(&endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("pub _top: Option<i64>"),
        "`$top` should become `_top`:\n{code}"
    );
    assert!(
        code.contains("pub user_name: Option<String>"),
        "`user.name` should become `user_name`:\n{code}"
    );
    assert!(
        code.contains(r#"#[serde(rename = "$top")]"#),
        "must preserve the `$top` wire name:\n{code}"
    );
    assert!(
        code.contains(r#"#[serde(rename = "user.name")]"#),
        "must preserve the `user.name` wire name:\n{code}"
    );
    // The query string is still keyed by the original wire names.
    assert!(
        code.contains(r#"query_pairs.push(("$top""#),
        "query key should stay `$top`:\n{code}"
    );
    assert!(
        code.contains(r#"query_pairs.push(("user.name""#),
        "query key should stay `user.name`:\n{code}"
    );
}
