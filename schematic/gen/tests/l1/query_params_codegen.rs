//! Tests for query parameter code generation in generated request structs.
//!
//! These tests verify the generated `EndpointSpec::into_parts()` returns
//! declared query parameters as unencoded pairs apart from the path; the
//! runtime encodes them once, when it assembles the URL. What actually
//! reaches a server is asserted by
//! `fresh_output_compile::slow_query_wire_matches_pre_split_baseline`.
//!
//! ## Test Categories
//!
//! 1. **Separation tests**: pairs are collected apart from the path, in order
//! 2. **URL encoding tests**: `into_parts` leaves query values unencoded
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
                .map(|(name, param_type)| ParamDef { rust_name: None, secret: false, pager_managed: false,
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


        })
    };

    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = method; fixture.path = path.to_string(); fixture.description = format!("Test endpoint for {}", id); fixture.request = None; fixture.response = ApiResponse::json_type("TestResponse"); fixture.headers = vec![]; fixture.params = params; fixture.cross_origin = None; fixture.pagination = None; fixture }
}

// =============================================================================
// Separation tests: pairs are collected apart from the path
// =============================================================================

#[test]
fn single_query_param_is_collected_apart_from_the_path() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#"let path = "/items".to_string();"#),
        "The path should hold no declared query\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains("let mut query: Vec<(String, String)> = Vec::new()"),
        "Should collect query pairs\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"query.push(("page".to_string(), value.to_string()))"#),
        "Should push the unencoded value under its wire name\nGenerated code:\n{}",
        code
    );
    let flat: String = code.split_whitespace().collect();
    assert!(
        flat.contains("RequestParts{method:<Selfasschematic_runtime::EndpointSpec>::METHOD,path,query,headers,"),
        "Query pairs should fill the `query` field of RequestParts\nGenerated code:\n{}",
        code
    );
    assert!(
        !code.contains("push_str"),
        "into_parts should no longer append a query string to the path\nGenerated code:\n{}",
        code
    );
}

#[test]
fn multiple_query_params_keep_declaration_order() {
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    let positions: Vec<usize> = ["page", "limit", "sort"]
        .iter()
        .map(|name| {
            let push = format!(r#"query.push(("{name}".to_string(), value.to_string()))"#);
            code.find(&push)
                .unwrap_or_else(|| panic!("missing push for {name}\nGenerated code:\n{code}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "Pairs should be pushed in declaration order\nGenerated code:\n{}",
        code
    );
}

#[test]
fn legacy_path_template_query_stays_in_the_path() {
    // A query written into the path template is not declared query data: it
    // stays in the path verbatim, and declared pairs travel separately.
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items?beta=true",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains(r#"let path = "/items?beta=true".to_string();"#),
        "The embedded query should stay in the path\nGenerated code:\n{}",
        code
    );
    assert!(
        code.contains(r#"query.push(("page".to_string(), value.to_string()))"#),
        "The declared parameter should still be a separate pair\nGenerated code:\n{}",
        code
    );
}

#[test]
fn endpoint_without_query_params_returns_empty_pairs() {
    let endpoint = make_endpoint_with_query("ListItems", RestMethod::Get, "/items", vec![]);

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    assert!(
        code.contains("let query: Vec<(String, String)> = Vec::new();"),
        "Should return an empty pair list\nGenerated code:\n{}",
        code
    );
    assert!(!code.contains("let mut query"), "No mutation needed\nGenerated code:\n{}", code);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    // Array params should generate a loop that pushes each value to collector
    assert!(
        code.contains("for value in values"),
        "Should iterate over array values\nGenerated code:\n{}",
        code
    );

    // Each array value becomes its own pair under the same key
    assert!(
        code.contains(r#"query.push(("tags".to_string(), value.to_string()))"#),
        "Array values should push repeated pairs\nGenerated code:\n{}",
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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
fn path_is_immutable_when_query_params_present() {
    let endpoint = make_endpoint_with_query(
        "ListRepoIssues",
        RestMethod::Get,
        "/repos/{owner}/{repo}/issues",
        vec![("page", QueryParamType::Integer)],
    );

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    // Query params no longer touch the path
    assert!(
        code.contains("let path = format!"),
        "Path should be built once when query params are present\nGenerated code:\n{}",
        code
    );
    assert!(
        !code.contains("let mut path"),
        "Path should not be mutated for query params\nGenerated code:\n{}",
        code
    );
}

#[test]
fn path_immutable_when_no_query_params() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetUser".to_string(); fixture.method = RestMethod::Get; fixture.path = "/users/{user_id}".to_string(); fixture.description = "Get a user".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("User"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    let flat = code.split_whitespace().collect::<String>().replace(",)", ")");
    assert!(
        flat.contains(
            "fninto_parts(self)->Result<schematic_runtime::RequestParts,schematic_runtime::InvalidArgument>"
        ),
        "into_parts should return Result<RequestParts, InvalidArgument>\nGenerated code:\n{}",
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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
fn query_values_are_left_unencoded_by_into_parts() {
    let endpoint = make_endpoint_with_query(
        "Search",
        RestMethod::Get,
        "/search",
        vec![("q", QueryParamType::String)],
    );

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
    let code = format_tokens(&tokens);

    assert!(
        !code.contains("urlencoding::encode(v)") && !code.contains("urlencoding::encode(value"),
        "into_parts must not encode query values\nGenerated code:\n{}",
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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

    let tokens = generate_request_struct(&crate::common::components::api_for(&endpoint), &endpoint);
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
    // The query pairs are still keyed by the original wire names.
    assert!(
        code.contains(r#"query.push(("$top".to_string()"#),
        "query key should stay `$top`:\n{code}"
    );
    assert!(
        code.contains(r#"query.push(("user.name".to_string()"#),
        "query key should stay `user.name`:\n{code}"
    );
}
