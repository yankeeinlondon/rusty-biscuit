use super::*;
use schematic_define::{ApiRequest, SuccessShape, QueryParamType, RestMethod};

fn make_endpoint(
    id: &str,
    method: RestMethod,
    path: &str,
    request: Option<ApiRequest>,
) -> Endpoint {
    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = method; fixture.path = path.to_string(); fixture.description = format!("Test endpoint for {}", id); fixture.request = request; fixture.response = vec![SuccessShape::json_type("TestResponse")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }
}

#[test]
fn generate_get_no_params() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct ListModelsRequest"));
    assert!(code.contains("#[derive(Debug, Clone, Default, Serialize, Deserialize)]"));
    assert!(code.contains("impl schematic_runtime::EndpointSpec for ListModelsRequest"), "{code}");
    assert!(
        squash(&code).contains(&squash(
            "fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument>"
        )),
        "{code}"
    );
    assert!(code.contains(r#"const ENDPOINT_ID: &'static str = "ListModels";"#), "{code}");
    assert!(
        code.contains("const METHOD: schematic_runtime::Method = schematic_runtime::Method::GET;"),
        "{code}"
    );
    assert!(code.contains(r#""/models".to_string()"#));
    assert!(!code.contains("SchematicError"), "{code}");
}

#[test]
fn generate_get_with_path_param() {
    let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct RetrieveModelRequest"));
    assert!(code.contains("pub model: String"));
    assert!(
        code.contains(
            r#"format!("/models/{}", urlencoding::encode(& self.model.to_string()))"#
        ),
        "path encoding mismatch:\n{code}"
    );
    assert!(code.contains("schematic_runtime::Method::GET"), "{code}");
}

#[test]
fn reserved_path_param_is_emitted_without_encoding() {
    // `{+repo_id}` opts out of percent-encoding so a slash-bearing value like
    // `meta-llama/Llama-3` keeps its separators in the built URL.
    let endpoint = make_endpoint("GetModel", RestMethod::Get, "/models/{+repo_id}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    // The struct field is the bare name; the `+` never reaches Rust identifiers.
    assert!(code.contains("pub repo_id: String"));
    assert!(
        code.contains(r#"format!("/models/{}", self.repo_id)"#),
        "reserved param should be raw, got:\n{code}"
    );
    assert!(
        !code.contains("urlencoding::encode(& self.repo_id"),
        "reserved param must not be percent-encoded:\n{code}"
    );
    assert!(
        !code.contains("urlencoding::encode(&self.repo_id"),
        "reserved param must not be percent-encoded:\n{code}"
    );
}

#[test]
fn plain_path_param_is_still_encoded() {
    let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(
        code.contains(r#"format!("/models/{}", urlencoding::encode(& self.model.to_string()))"#),
        "plain param must stay percent-encoded:\n{code}"
    );
}

#[test]
fn generate_get_with_multiple_path_params() {
    let endpoint = make_endpoint(
        "GetMessage",
        RestMethod::Get,
        "/threads/{thread_id}/messages/{message_id}",
        None,
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct GetMessageRequest"));
    assert!(code.contains("pub thread_id: String"));
    assert!(code.contains("pub message_id: String"));
    // The path is now let path (no mut since no query params)
    assert!(
        code.contains("let path = format!("),
        "Expected let path format, got:\n{}",
        code
    );
    assert!(code.contains("/threads/{}/messages/{}"));
    assert!(code.contains("self.thread_id"));
    assert!(code.contains("self.message_id"));
}

#[test]
fn generate_post_with_body() {
    let endpoint = make_endpoint(
        "CreateCompletion",
        RestMethod::Post,
        "/completions",
        Some(ApiRequest::json_type("CreateCompletionRequest")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct CreateCompletionRequest"));
    assert!(code.contains("#[derive(Debug, Clone, Default, Serialize, Deserialize)]"));
    assert!(code.contains("pub body: CreateCompletionRequest"));
    assert!(code.contains("schematic_runtime::Method::POST"), "{code}");
    assert!(code.contains("schematic_runtime::RequestBody::Json("), "{code}");
    assert!(code.contains("serde_json::to_string(&self.body)"));
    // A body that cannot be serialized is an invalid argument, not a send failure.
    assert!(
        squash(&code).contains(&squash(
            r#".map_err(|_| { schematic_runtime::InvalidArgument::new("the request body cannot be serialized as JSON") })?"#
        )),
        "{code}"
    );
}

#[test]
fn generate_post_with_path_param_and_body() {
    let endpoint = make_endpoint(
        "CreateMessage",
        RestMethod::Post,
        "/threads/{thread_id}/messages",
        Some(ApiRequest::json_type("CreateMessageRequest")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct CreateMessageRequest"));
    assert!(code.contains("pub thread_id: String"));
    assert!(code.contains("pub body: CreateMessageRequest"));
    assert!(code.contains("#[derive(Debug, Clone, Default, Serialize, Deserialize)]"));
}

#[test]
fn generate_delete_with_path_param() {
    let endpoint = make_endpoint("DeleteModel", RestMethod::Delete, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct DeleteModelRequest"));
    assert!(code.contains("pub model: String"));
    assert!(code.contains("schematic_runtime::Method::DELETE"), "{code}");
    assert!(code.contains("body: schematic_runtime::RequestBody::Empty"), "{code}");
}

#[test]
fn generate_patch_with_body() {
    let endpoint = make_endpoint(
        "UpdateThread",
        RestMethod::Patch,
        "/threads/{thread_id}",
        Some(ApiRequest::json_type("UpdateThreadRequest")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct UpdateThreadRequest"));
    assert!(code.contains("pub thread_id: String"));
    assert!(code.contains("pub body: UpdateThreadRequest"));
    assert!(code.contains("schematic_runtime::Method::PATCH"), "{code}");
}

#[test]
fn validate_generated_code_passes() {
    let endpoint = make_endpoint("TestEndpoint", RestMethod::Get, "/test/{id}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    assert!(validate_generated_code(&tokens).is_ok());
}

#[test]
fn format_generated_code_produces_valid_rust() {
    let endpoint = make_endpoint(
        "ComplexEndpoint",
        RestMethod::Post,
        "/orgs/{org}/repos/{repo}/issues",
        Some(ApiRequest::json_type("CreateIssueRequest")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);

    let formatted = format_generated_code(&tokens);
    assert!(formatted.is_ok());

    let code = formatted.unwrap();
    assert!(code.contains("pub org: String"));
    assert!(code.contains("pub repo: String"));
    assert!(code.contains("pub body: CreateIssueRequest"));
}

#[test]
fn all_http_methods_generate_correct_string() {
    let methods = [
        (RestMethod::Get, "GET"),
        (RestMethod::Post, "POST"),
        (RestMethod::Put, "PUT"),
        (RestMethod::Patch, "PATCH"),
        (RestMethod::Delete, "DELETE"),
        (RestMethod::Head, "HEAD"),
        (RestMethod::Options, "OPTIONS"),
    ];

    for (method, expected_str) in methods {
        let endpoint = make_endpoint("Test", method, "/test", None);
        let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains(&format!(
                "const METHOD: schematic_runtime::Method = schematic_runtime::Method::{expected_str};"
            )),
            "Expected method {expected_str} not found in generated code:\n{code}"
        );
        // `into_parts` reads the method from the constant rather than repeating it.
        assert!(
            code.contains("method: <Self as schematic_runtime::EndpointSpec>::METHOD"),
            "{code}"
        );
    }
}

#[test]
fn generates_new_for_path_param_only() {
    let endpoint = make_endpoint("GetModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(
        code.contains("pub fn new(model: impl Into<String>) -> Self"),
        "Expected new() with impl Into<String> param, got:\n{}",
        code
    );
    assert!(
        code.contains("model: model.into()"),
        "Expected model.into() field init, got:\n{}",
        code
    );
}

#[test]
fn generates_new_for_path_param_and_body() {
    let endpoint = make_endpoint(
        "CreateMessage",
        RestMethod::Post,
        "/threads/{thread_id}/messages",
        Some(ApiRequest::json_type("CreateMessageBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(
        code.contains("pub fn new(thread_id: impl Into<String>, body: CreateMessageBody) -> Self"),
        "Expected new() with path param and body, got:\n{}",
        code
    );
    assert!(
        code.contains("thread_id: thread_id.into()"),
        "Expected thread_id.into() field init, got:\n{}",
        code
    );
}

#[test]
fn generates_new_for_body_only() {
    let endpoint = make_endpoint(
        "CreateCompletion",
        RestMethod::Post,
        "/completions",
        Some(ApiRequest::json_type("CreateCompletionBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(
        code.contains("pub fn new(body: CreateCompletionBody) -> Self"),
        "Expected new() with body only, got:\n{}",
        code
    );
}

#[test]
fn skips_new_for_empty_request() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // No new() needed - Default suffices
    assert!(
        !code.contains("pub fn new("),
        "Expected no new() for empty request, got:\n{}",
        code
    );
}

#[test]
fn generates_new_with_multiple_path_params() {
    let endpoint = make_endpoint(
        "GetMessage",
        RestMethod::Get,
        "/threads/{thread_id}/messages/{message_id}",
        None,
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(
        code.contains(
            "pub fn new(thread_id: impl Into<String>, message_id: impl Into<String>) -> Self"
        ),
        "Expected new() with multiple impl Into<String> params, got:\n{}",
        code
    );
}

// === Example generation tests ===

#[test]
fn generates_example_with_default_for_no_required_fields() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should have example section
    assert!(
        code.contains("## Example"),
        "Expected example section, got:\n{}",
        code
    );
    // Should use default() for no required fields
    assert!(
        code.contains("ListModelsRequest::default()"),
        "Expected default() usage for empty struct, got:\n{}",
        code
    );
    assert!(code.contains("let request = ListModelsRequest::default();"), "the statement ends on its own line:\n{code}");
    assert!(code.contains("```no_run"), "Expected a no_run doctest, got:\n{code}");
    assert!(!code.contains("///;"), "{code}");
}

#[test]
fn generates_example_with_new_for_path_params() {
    let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should have example section
    assert!(
        code.contains("## Example"),
        "Expected example section, got:\n{}",
        code
    );
    // Should use new() with path param
    assert!(
        code.contains("RetrieveModelRequest::new("),
        "Expected new() usage for path param struct, got:\n{}",
        code
    );
    // Should show model_value placeholder
    assert!(
        code.contains("\"model_value\""),
        "Expected model_value placeholder, got:\n{}",
        code
    );
}

#[test]
fn generates_example_with_body_construction() {
    let endpoint = make_endpoint(
        "CreateCompletion",
        RestMethod::Post,
        "/completions",
        Some(ApiRequest::json_type("CreateCompletionBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should have example section
    assert!(
        code.contains("## Example"),
        "Expected example section, got:\n{}",
        code
    );
    // Should show body type import
    assert!(
        code.contains("CreateCompletionBody"),
        "Expected body type in import, got:\n{}",
        code
    );
    assert!(
        code.contains("use schematic_schema::api::{CreateCompletionRequest, CreateCompletionBody};"),
        "a body type named from the module is imported, got:\n{code}"
    );
    assert!(
        code.contains("let body: CreateCompletionBody = Default::default();"),
        "Expected body construction, got:\n{code}"
    );
    // Should use new() with body
    assert!(
        code.contains("CreateCompletionRequest::new(body)"),
        "Expected new(body) usage, got:\n{}",
        code
    );
}

#[test]
fn generates_example_with_path_params_and_body() {
    let endpoint = make_endpoint(
        "CreateMessage",
        RestMethod::Post,
        "/threads/{thread_id}/messages",
        Some(ApiRequest::json_type("CreateMessageBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should use new() with path param and body
    assert!(
        code.contains("CreateMessageRequest::new(\"thread_id_value\", body)"),
        "Expected new(thread_id_value, body) usage, got:\n{}",
        code
    );
}

#[test]
fn generates_example_with_custom_module_path() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let tokens = generate_request_struct_with_options(&crate::test_utils::api_for(&endpoint), &endpoint, "Request", Some("openai"), "OpenAI");
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should use custom module path
    assert!(
        code.contains("use schematic_schema::openai::ListModelsRequest"),
        "Expected custom module path, got:\n{}",
        code
    );
}

#[test]
fn generates_example_with_multiple_path_params() {
    let endpoint = make_endpoint(
        "GetMessage",
        RestMethod::Get,
        "/threads/{thread_id}/messages/{message_id}",
        None,
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should show both path param values
    assert!(
        code.contains("\"thread_id_value\""),
        "Expected thread_id_value placeholder, got:\n{}",
        code
    );
    assert!(
        code.contains("\"message_id_value\""),
        "Expected message_id_value placeholder, got:\n{}",
        code
    );
    // Should have both in new() call
    assert!(
        code.contains("GetMessageRequest::new(\"thread_id_value\", \"message_id_value\")"),
        "Expected new with both params, got:\n{}",
        code
    );
}

// === From<Body> tests ===

#[test]
fn generates_from_body_for_body_only_request() {
    let endpoint = make_endpoint(
        "CreateCompletion",
        RestMethod::Post,
        "/completions",
        Some(ApiRequest::json_type("CreateCompletionBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should have From<Body> impl for body-only request
    assert!(
        code.contains("impl From<CreateCompletionBody> for CreateCompletionRequest"),
        "Expected From<Body> impl for body-only request, got:\n{}",
        code
    );
    assert!(
        code.contains("fn from(body: CreateCompletionBody) -> Self"),
        "Expected from() method signature, got:\n{}",
        code
    );
    assert!(
        code.contains("Self { body }"),
        "Expected Self {{ body }} in from(), got:\n{}",
        code
    );
}

#[test]
fn skips_from_body_for_request_with_path_params() {
    let endpoint = make_endpoint(
        "CreateMessage",
        RestMethod::Post,
        "/threads/{thread_id}/messages",
        Some(ApiRequest::json_type("CreateMessageBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should NOT have From<Body> impl for request with path params
    assert!(
        !code.contains("impl From<CreateMessageBody> for CreateMessageRequest"),
        "Expected no From<Body> impl for request with path params, got:\n{}",
        code
    );
}

// === From<&str> and From<String> impl tests ===

#[test]
fn generates_from_str_for_single_path_param_no_body() {
    let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should generate From<&str>
    assert!(
        code.contains("impl From<&str> for RetrieveModelRequest"),
        "Expected From<&str> impl, got:\n{}",
        code
    );
    assert!(
        code.contains("model: param.to_string()"),
        "Expected param.to_string() in From<&str>, got:\n{}",
        code
    );
}

#[test]
fn generates_from_string_for_single_path_param_no_body() {
    let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should generate From<String>
    assert!(
        code.contains("impl From<String> for RetrieveModelRequest"),
        "Expected From<String> impl, got:\n{}",
        code
    );
    // The From<String> impl should directly use the param
    assert!(
        code.contains("Self { model: param }"),
        "Expected direct param assignment in From<String>, got:\n{}",
        code
    );
}

#[test]
fn skips_from_string_impls_for_multi_param_requests() {
    let endpoint = make_endpoint(
        "GetMessage",
        RestMethod::Get,
        "/threads/{thread_id}/messages/{message_id}",
        None,
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should NOT generate From impls for multi-param
    assert!(
        !code.contains("impl From<&str> for GetMessageRequest"),
        "Should NOT have From<&str> for multi-param, got:\n{}",
        code
    );
    assert!(
        !code.contains("impl From<String> for GetMessageRequest"),
        "Should NOT have From<String> for multi-param, got:\n{}",
        code
    );
}

#[test]
fn skips_from_string_impls_for_requests_with_body() {
    let endpoint = make_endpoint(
        "CreateCompletion",
        RestMethod::Post,
        "/completions",
        Some(ApiRequest::json_type("CreateCompletionBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should NOT generate From<&str>/From<String> impls for body requests
    assert!(
        !code.contains("impl From<&str> for CreateCompletionRequest"),
        "Should NOT have From<&str> for body request, got:\n{}",
        code
    );
    assert!(
        !code.contains("impl From<String> for CreateCompletionRequest"),
        "Should NOT have From<String> for body request, got:\n{}",
        code
    );
}

#[test]
fn skips_from_string_impls_for_single_param_with_body() {
    let endpoint = make_endpoint(
        "UpdateModel",
        RestMethod::Patch,
        "/models/{model}",
        Some(ApiRequest::json_type("UpdateModelBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should NOT generate From<&str>/From<String> impls - has body
    assert!(
        !code.contains("impl From<&str> for UpdateModelRequest"),
        "Should NOT have From<&str> for body request, got:\n{}",
        code
    );
    assert!(
        !code.contains("impl From<String> for UpdateModelRequest"),
        "Should NOT have From<String> for body request, got:\n{}",
        code
    );
}

#[test]
fn skips_from_string_impls_for_no_params() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Should NOT generate From<&str>/From<String> impls - no params
    assert!(
        !code.contains("impl From<&str> for ListModelsRequest"),
        "Should NOT have From<&str> for no params, got:\n{}",
        code
    );
    assert!(
        !code.contains("impl From<String> for ListModelsRequest"),
        "Should NOT have From<String> for no params, got:\n{}",
        code
    );
}

// === Merged impl block tests ===

#[test]
fn one_inherent_impl_and_one_endpoint_spec_impl_for_path_param_request() {
    // A path param request has `new()` inherently and `into_parts()` on the trait
    let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Count occurrences of "impl RetrieveModelRequest" (non-From impls)
    let impl_count = code.matches("impl RetrieveModelRequest").count();

    assert_eq!(
        impl_count, 1,
        "Expected exactly 1 impl block for RetrieveModelRequest (excluding From impls), got {}\n\nCode:\n{}",
        impl_count, code
    );

    assert!(
        code.contains("pub fn new("),
        "Expected new() method in impl block"
    );
    assert_eq!(
        code.matches("impl schematic_runtime::EndpointSpec for RetrieveModelRequest").count(),
        1,
        "{code}"
    );
    assert!(
        !code.contains("pub fn into_parts("),
        "into_parts() belongs to the EndpointSpec impl, not the inherent one:\n{code}"
    );
}

#[test]
fn single_impl_block_with_body_and_both_methods() {
    // A request with a body has one inherent impl, holding `new()`; its
    // `into_parts()` lives in the `EndpointSpec` impl.
    let endpoint = make_endpoint(
        "CreateCompletion",
        RestMethod::Post,
        "/completions",
        Some(ApiRequest::json_type("CreateCompletionBody")),
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Count occurrences of "impl CreateCompletionRequest" (non-From impls)
    let impl_count = code.matches("impl CreateCompletionRequest").count();

    assert_eq!(
        impl_count, 1,
        "Expected exactly 1 impl block for CreateCompletionRequest (excluding From impls), got {}\n\nCode:\n{}",
        impl_count, code
    );
}

#[test]
fn no_inherent_impl_for_no_param_request() {
    // A request with no params has no `new()`, so only the EndpointSpec impl remains
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert_eq!(
        code.matches("impl ListModelsRequest").count(),
        0,
        "Expected no inherent impl block for ListModelsRequest\n\nCode:\n{code}"
    );
    assert!(
        !code.contains("pub fn new("),
        "Expected no new() method for no-param request"
    );
    assert_eq!(
        code.matches("impl schematic_runtime::EndpointSpec for ListModelsRequest").count(),
        1,
        "{code}"
    );
}

// === Query parameter tests ===

fn make_endpoint_with_query(
    id: &str,
    method: RestMethod,
    path: &str,
    request: Option<ApiRequest>,
    query_params: Vec<(&str, bool, Option<&str>, QueryParamType)>,
) -> Endpoint {
    use schematic_define::{EndpointParams, ParamDef, ParamStyle};

    let params = if query_params.is_empty() {
        None
    } else {
        Some(EndpointParams { path: vec![],
            query: query_params
                .into_iter()
                .map(|(name, required, desc, param_type)| ParamDef { deprecated: false, rust_name: None, secret: false, pager_managed: false,
                    name: name.to_string(),
                    required,
                    description: desc.map(|s| s.to_string()),
                    param_type,
                    explode: false,
                    style: ParamStyle::Form,
                })
                .collect(),
            header: vec![],
            cookie: vec![],


        })
    };

    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = method; fixture.path = path.to_string(); fixture.description = format!("Test endpoint for {}", id); fixture.request = request; fixture.response = vec![SuccessShape::json_type("TestResponse")]; fixture.headers = vec![]; fixture.params = params; fixture.cross_origin = None; fixture.pagination = None; fixture }
}

#[test]
fn generates_query_param_fields() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        None,
        vec![
            ("page", false, Some("Page number"), QueryParamType::Integer),
            (
                "limit",
                false,
                Some("Items per page"),
                QueryParamType::Integer,
            ),
        ],
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub page: Option<i64>"));
    assert!(code.contains("pub limit: Option<i64>"));
    assert!(code.contains("Query parameter: Page number"));
    assert!(code.contains("Query parameter: Items per page"));
}

#[test]
fn generates_query_param_builder_methods() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        None,
        vec![("page", false, None, QueryParamType::Integer)],
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(
        code.contains("pub fn with_page(mut self, value: i64) -> Self"),
        "Expected with_page builder method, got:\n{}",
        code
    );
}

#[test]
fn generates_query_param_in_into_parts() {
    let endpoint = make_endpoint_with_query(
        "ListItems",
        RestMethod::Get,
        "/items",
        None,
        vec![
            ("page", false, None, QueryParamType::Integer),
            ("limit", false, None, QueryParamType::Integer),
        ],
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Pairs are collected apart from the path and returned unencoded
    assert!(
        code.contains("let mut query: Vec<(String, schematic_runtime::QueryValue)> = Vec::new()"),
        "Expected query collector, got:\n{}",
        code
    );
    assert!(
        code.contains(r#"query.push(("page".to_string(), value.to_string().into()))"#),
        "Expected page param push, got:\n{}",
        code
    );
    assert!(
        code.contains(r#"query.push(("limit".to_string(), value.to_string().into()))"#),
        "Expected limit param push, got:\n{}",
        code
    );
    assert!(
        !code.contains("urlencoding::encode(v)"),
        "Query values are encoded at URL assembly, not here, got:\n{}",
        code
    );
    assert!(
        code.contains(r#"let path = "/items".to_string();"#)
            && squash(&code).contains("path,query,headers,"),
        "Expected an unmodified path and the pairs in the `query` field, got:\n{}",
        code
    );
}

#[test]
fn generates_different_query_param_types() {
    let endpoint = make_endpoint_with_query(
        "Search",
        RestMethod::Get,
        "/search",
        None,
        vec![
            ("q", false, None, QueryParamType::String),
            (
                "sort",
                false,
                None,
                QueryParamType::Enum(vec!["asc".into(), "desc".into()]),
            ),
            ("flag", false, None, QueryParamType::Boolean),
        ],
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub q: Option<String>"), "{code}");
    // An enum gets its own type, sent by its wire spelling.
    assert!(code.contains("pub sort: Option<SearchRequestSort>"), "{code}");
    assert!(code.contains("pub enum SearchRequestSort {"), "{code}");
    assert!(code.contains("Self::Desc => \"desc\""), "{code}");
    assert!(code.contains("pub fn with_sort(mut self, value: SearchRequestSort) -> Self"), "{code}");
    assert!(code.contains("pub flag: Option<bool>"), "{code}");
}

#[test]
fn generates_array_query_param() {
    let endpoint = make_endpoint_with_query(
        "Filter",
        RestMethod::Get,
        "/items",
        None,
        vec![(
            "tags",
            false,
            None,
            QueryParamType::Array(Box::new(QueryParamType::String)),
        )],
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub tags: Option<Vec<String>>"), "{code}");
    // A form array without explode is one pair of comma-delimited elements.
    assert!(
        squash(&code).contains(&squash(
            r#"query.push(("tags".to_string(), schematic_runtime::QueryValue::delimited(value.iter().map(|element| element.to_string()), schematic_runtime::QueryDelimiter::Comma)));"#
        )),
        "Expected one delimited tags pair, got:\n{code}"
    );
    assert!(!code.contains("for element in value"), "{code}");
}

#[test]
fn query_params_with_path_params() {
    let endpoint = make_endpoint_with_query(
        "ListRepoIssues",
        RestMethod::Get,
        "/repos/{owner}/{repo}/issues",
        None,
        vec![
            ("state", false, None, QueryParamType::String),
            ("page", false, None, QueryParamType::Integer),
        ],
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Both path and query params
    assert!(code.contains("pub owner: String"));
    assert!(code.contains("pub repo: String"));
    assert!(code.contains("pub state: Option<String>"));
    assert!(code.contains("pub page: Option<i64>"));

    // new() should include path params
    assert!(code.contains("pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self"));
}

#[test]
fn generates_snake_case_query_param_fields_from_camel_case() {
    let endpoint = make_endpoint_with_query(
        "SetMute",
        RestMethod::Get,
        "/audio/mute",
        None,
        vec![
            (
                "isMute",
                false,
                Some("Whether to mute"),
                QueryParamType::Boolean,
            ),
            ("openType", false, None, QueryParamType::String),
        ],
    );
    let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Rust fields should be snake_case
    assert!(
        code.contains("pub is_mute: Option<bool>"),
        "Expected snake_case field is_mute, got:\n{}",
        code
    );
    assert!(
        code.contains("pub open_type: Option<String>"),
        "Expected snake_case field open_type, got:\n{}",
        code
    );

    // Builder methods should be snake_case
    assert!(
        code.contains("fn with_is_mute("),
        "Expected with_is_mute builder method, got:\n{}",
        code
    );
    assert!(
        code.contains("fn with_open_type("),
        "Expected with_open_type builder method, got:\n{}",
        code
    );

    // Wire-format query string names should preserve original camelCase
    assert!(
        code.contains(r#""isMute""#),
        "Expected original isMute in query string, got:\n{}",
        code
    );
    assert!(
        code.contains(r#""openType""#),
        "Expected original openType in query string, got:\n{}",
        code
    );
}

// === Pagination metadata in doc comments tests ===

mod pagination_doc_tests {
    use super::*;
    use schematic_define::params::{EndpointParams, QueryParamType};

    fn make_endpoint_with_pagination(id: &str, path: &str, params: EndpointParams) -> Endpoint {

        { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = RestMethod::Get; fixture.path = path.to_string(); fixture.description = format!("Test endpoint for {}", id); fixture.request = None; fixture.response = vec![SuccessShape::json_type("TestResponse")]; fixture.headers = vec![]; fixture.params = Some(params); fixture.cross_origin = None; fixture.pagination = None; fixture }
    }

    #[test]
    fn github_pagination_docs_include_default_and_max_values() {
        let endpoint =
            make_endpoint_with_pagination("ListRepos", "/repos", EndpointParams::new().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)")).with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)")));
        let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Verify page param has default 1
        assert!(
            code.contains("Query parameter: Page number (1-indexed, default: 1)"),
            "Expected page doc with default value, got:\n{}",
            code
        );
        // Verify per_page param has default and max
        assert!(
            code.contains("Query parameter: Items per page (default: 100, max: 100)"),
            "Expected per_page doc with default and max values, got:\n{}",
            code
        );
    }

    #[test]
    fn bitbucket_pagination_docs_have_correct_values() {
        let endpoint = make_endpoint_with_pagination(
            "ListPullRequests",
            "/pullrequests",
            EndpointParams::new().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)")).with_query_param("pagelen", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)")),
        );
        let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Verify page param has default 1
        assert!(
            code.contains("Query parameter: Page number (1-indexed, default: 1)"),
            "Expected page doc with default value, got:\n{}",
            code
        );
        // Verify pagelen param (Bitbucket-style) has default 50, max 100
        assert!(
            code.contains("Query parameter: Items per page (default: 50, max: 100)"),
            "Expected pagelen doc with default 50 and max 100, got:\n{}",
            code
        );
        // Verify field name is pagelen
        assert!(
            code.contains("pub pagelen: Option<i64>"),
            "Expected pagelen field, got:\n{}",
            code
        );
    }

    #[test]
    fn gitea_pagination_docs_have_limit_param() {
        let endpoint =
            make_endpoint_with_pagination("ListIssues", "/issues", EndpointParams::new().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)")).with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)")));
        let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Verify limit param (Gitea-style) has default 50, max 100
        assert!(
            code.contains("Query parameter: Items per page (default: 50, max: 100)"),
            "Expected limit doc with default 50 and max 100, got:\n{}",
            code
        );
        // Verify field name is limit
        assert!(
            code.contains("pub limit: Option<i64>"),
            "Expected limit field, got:\n{}",
            code
        );
    }

    #[test]
    fn offset_limit_pagination_docs_show_values() {
        let endpoint = make_endpoint_with_pagination(
            "ListItems",
            "/items",
            EndpointParams::new().with_query_param("offset", QueryParamType::Integer, false, Some("Number of items to skip")).with_query_param("limit", QueryParamType::Integer, false, Some("Maximum items to return (default: 25, max: 200)")),
        );
        let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Verify offset param description
        assert!(
            code.contains("Query parameter: Number of items to skip"),
            "Expected offset doc, got:\n{}",
            code
        );
        // Verify limit param has default 25, max 200
        assert!(
            code.contains("Query parameter: Maximum items to return (default: 25, max: 200)"),
            "Expected limit doc with default 25 and max 200, got:\n{}",
            code
        );
    }

    #[test]
    fn cursor_pagination_docs_show_default_limit() {
        let endpoint = make_endpoint_with_pagination(
            "ListEvents",
            "/events",
            EndpointParams::new().with_query_param("after", QueryParamType::String, false, Some("Pagination cursor from previous response")).with_query_param("limit", QueryParamType::Integer, false, Some("Maximum items to return (default: 20)")),
        );
        let tokens = generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Verify cursor param description
        assert!(
            code.contains("Query parameter: Pagination cursor from previous response"),
            "Expected cursor doc, got:\n{}",
            code
        );
        // Verify limit param has default 20
        assert!(
            code.contains("Query parameter: Maximum items to return (default: 20)"),
            "Expected limit doc with default 20, got:\n{}",
            code
        );
    }
}

// ---- raw text and binary bodies (R21, R22) ----

#[test]
fn text_body_is_a_string_field_sent_as_raw_bytes_under_its_content_type() {
    let endpoint = make_endpoint(
        "SendNote",
        RestMethod::Post,
        "/notes",
        Some(ApiRequest::Text {
            content_type: "text/plain; charset=utf-8".to_string(),
        }),
    );
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();

    assert!(code.contains("pub body: String,"), "{code}");
    assert!(code.contains("pub fn new(body: String) -> Self"), "{code}");
    assert!(code.contains("impl From<String> for SendNoteRequest"), "{code}");
    assert!(
        code.contains(r#"content_type: "text/plain; charset=utf-8".to_string()"#),
        "{code}"
    );
    assert!(code.contains("bytes: self.body.into_bytes()"), "{code}");
    assert!(!code.contains("RequestBody::Empty"), "the body must not be dropped:\n{code}");
    // The example does not pretend `String` is a schema type to import.
    assert!(!code.contains("SendNoteRequest, String}"), "{code}");
}

#[test]
fn binary_body_is_a_byte_vector_sent_as_raw_bytes_under_its_content_type() {
    let endpoint = make_endpoint(
        "UploadBlob",
        RestMethod::Put,
        "/blobs/{name}",
        Some(ApiRequest::Binary {
            content_type: "application/octet-stream".to_string(),
        }),
    );
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();

    assert!(code.contains("pub body: Vec<u8>,"), "{code}");
    assert!(code.contains("pub fn new(name: impl Into<String>, body: Vec<u8>) -> Self"), "{code}");
    assert!(
        code.contains(r#"content_type: "application/octet-stream".to_string()"#),
        "{code}"
    );
    // `Vec<u8>` converts into the runtime's byte buffer without copying.
    assert!(code.contains("bytes: self.body.into(),"), "{code}");
    assert!(validate_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).is_ok());
}

// ---- header and cookie parameters (R25, R26) ----

/// `code` with all whitespace, and any trailing comma before `)` or `]`,
/// removed, so assertions survive line wrapping.
fn squash(code: &str) -> String {
    let flat: String = code.split_whitespace().collect();
    flat.replace(",)", ")").replace(",]", "]")
}

fn named_param(name: &str, param_type: QueryParamType) -> schematic_define::ParamDef {
    schematic_define::ParamDef { deprecated: false, rust_name: None, secret: false, pager_managed: false,
        name: name.to_string(),
        required: true,
        description: None,
        param_type,
        explode: false,
        style: schematic_define::ParamStyle::Form,
    }
}

fn endpoint_with_named_params(
    headers: Vec<schematic_define::ParamDef>,
    cookies: Vec<schematic_define::ParamDef>,
) -> Endpoint {
    { let mut fixture = make_endpoint("Search", RestMethod::Get, "/search", None); fixture.headers = vec![("X-Mode".to_string(), "static".to_string())]; fixture.params = Some(schematic_define::EndpointParams {
            header: headers,
            cookie: cookies,
            ..Default::default()
        }); fixture }
}

#[test]
fn header_parameters_become_optional_fields_sent_under_their_wire_names() {
    let endpoint = endpoint_with_named_params(
        vec![
            named_param("X-Trace-Id", QueryParamType::String),
            named_param("X-Ids", QueryParamType::Array(Box::new(QueryParamType::Integer))),
        ],
        vec![],
    );
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();
    let flat = squash(&code);

    // `required` is not enforced: every parameter is an `Option` with a builder.
    assert!(code.contains(r#"#[serde(rename = "X-Trace-Id")]"#), "{code}");
    assert!(code.contains("pub x_trace_id: Option<String>,"), "{code}");
    assert!(code.contains("pub x_ids: Option<Vec<i64>>,"), "{code}");
    assert!(code.contains("pub fn with_x_trace_id(mut self, value: String) -> Self"), "{code}");
    assert!(code.contains("pub fn with_x_ids(mut self, value: Vec<i64>) -> Self"), "{code}");
    // A set parameter replaces a same-named endpoint header, case-insensitively.
    assert!(
        flat.contains(&squash(r#"vec![("X-Mode".to_string(), "static".to_string())]"#)),
        "{code}"
    );
    assert!(
        flat.contains(&squash(r#"headers.retain(|(name, _)| !name.eq_ignore_ascii_case("X-Trace-Id"))"#)),
        "{code}"
    );
    assert!(flat.contains(&squash(r#"headers.push(("X-Trace-Id".to_string(), value))"#)), "{code}");
    // Arrays are joined with `,`; header values never join the query.
    assert!(code.contains(r#".join(",")"#), "{code}");
    assert!(code.contains("let query: Vec<(String, schematic_runtime::QueryValue)> = Vec::new()"), "{code}");
    assert!(!code.contains("combine_cookie_headers"), "{code}");
}

#[test]
fn cookie_parameters_are_encoded_and_folded_into_one_cookie_header() {
    let endpoint = endpoint_with_named_params(
        vec![],
        vec![
            named_param("session", QueryParamType::String),
            named_param("prefs", QueryParamType::Array(Box::new(QueryParamType::String))),
        ],
    );
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();
    let flat = squash(&code);

    assert!(code.contains("pub session: Option<String>,"), "{code}");
    assert!(code.contains("pub fn with_prefs(mut self, value: Vec<String>) -> Self"), "{code}");
    assert!(
        flat.contains(&squash(
            r#"cookies.push(("session".to_string(), crate::shared::encode_cookie_value(&value.to_string()),))"#
        )),
        "{code}"
    );
    // Each array element is encoded on its own before the `,` join.
    assert!(flat.contains(&squash("crate::shared::encode_cookie_value(&item.to_string(),)")), "{code}");
    assert!(
        flat.contains(&squash("crate::shared::combine_cookie_headers(&mut headers, &cookies)?")),
        "{code}"
    );
    // Cookies fold into the endpoint's headers, so `headers` is built mutably.
    assert!(
        flat.contains(&squash(r#"let mut headers: Vec<(String, String)> = vec![("X-Mode".to_string(), "static".to_string())];"#)),
        "{code}"
    );
}

#[test]
fn named_parameters_start_unset_in_every_constructor() {
    let endpoint = { let mut fixture = endpoint_with_named_params(
            vec![named_param("X-Trace-Id", QueryParamType::String)],
            vec![named_param("session", QueryParamType::String)],
        ); fixture.path = "/search/{scope}".to_string(); fixture };
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();

    assert!(code.contains("x_trace_id: None"), "{code}");
    assert!(code.contains("session: None"), "{code}");
    assert!(code.contains("impl From<&str> for SearchRequest"), "{code}");
    assert!(validate_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).is_ok());
}

#[test]
fn endpoints_without_named_parameters_keep_their_inline_headers() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();

    assert!(code.contains("let headers: Vec<(String, String)> = vec![];"), "{code}");
    assert!(!code.contains("let mut headers"), "{code}");
}

#[test]
fn explicit_rust_parameter_name_preserves_wire_query_and_ordinary_setters() {
    let mut endpoint = make_endpoint("ListTree", RestMethod::Get, "/tree", None);
    let mut reference = named_param("ref", QueryParamType::String);
    reference.rust_name = Some("git_ref".into());
    let mut query = vec![reference, named_param("recursive", QueryParamType::Boolean), named_param("beta", QueryParamType::Boolean)];
    for param in &mut query {
        param.required = false;
    }
    endpoint.params = Some(schematic_define::EndpointParams { query, ..Default::default() });
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();
    assert!(code.contains("pub git_ref: Option<String>"), "{code}");
    assert!(code.contains("#[serde(rename = \"ref\")]"), "{code}");
    assert!(code.contains("fn with_git_ref("), "{code}");
    assert!(code.contains("self.git_ref"), "{code}");
    assert!(code.contains("query.push((\"ref\".to_string(), value.to_string().into()))"), "{code}");
    assert!(code.contains("fn with_recursive("), "{code}");
    assert!(code.contains("fn with_beta("), "{code}");
    assert!(!code.contains("query.push((\"git_ref\""), "{code}");
    endpoint.params.as_mut().unwrap().query[0].rust_name = Some("GitRef".into());
    let code = format_generated_code(&generate_request_struct(&crate::test_utils::api_for(&endpoint), &endpoint)).unwrap();
    assert!(code.contains("pub GitRef: Option<String>"), "explicit identifiers keep their spelling: {code}");
    assert!(code.contains("self.GitRef"), "{code}");
}

#[test]
fn qualified_and_container_body_examples_are_valid_rust() {
    for body in ["::schematic_definitions::openai::BetaCreateResponse", "::serde_json::Value", "::std::vec::Vec<::schematic_definitions::openai::Model>"] {
        let lines = generate_doc_example("CreateResponseRequest", &PathExample { args: vec![], imports: vec![] }, &[], true, Some(body), Some("openai"));
        let start = lines.iter().position(|line| line.trim() == "```no_run").unwrap();
        let source = lines[start + 1..].iter().take_while(|line| line.trim() != "```").cloned().collect::<Vec<_>>().join("\n");
        syn::parse_file(&format!("fn example() {{\n{source}\n}}"))
            .unwrap_or_else(|error| panic!("invalid example for {body}: {error}\n{source}"));
        assert!(!source.contains("::{CreateResponseRequest, ::"), "{source}");
    }
}

/// An API whose `Model` component is spelled `crate::types::Model`, with one
/// POST endpoint sending and returning `reference`.
fn wrapped_api(reference: schematic_define::TypeRef) -> (RestApi, Endpoint) {
    let mut endpoint = make_endpoint("Wrap", RestMethod::Post, "/wrap", Some(ApiRequest::json(reference.clone())));
    endpoint.response = vec![SuccessShape::json(reference)];
    let mut api = crate::test_utils::api_for(&endpoint);
    if let Some(model) = api.schemas.iter_mut().find(|c| c.name == "Model") {
        model.rust_path = Some("crate::types::Model".into());
    }
    (api, endpoint)
}

#[test]
fn wrapper_references_spell_their_rust_types_through_the_components() {
    use schematic_define::{PrimitiveType, TypeRef};
    let model = || Box::new(TypeRef::Named("Model".into()));
    let cases = [
        (TypeRef::Named("Model".into()), "crate::types::Model"),
        (TypeRef::Array(model()), "::std::vec::Vec<crate::types::Model>"),
        (TypeRef::Optional(model()), "::std::option::Option<crate::types::Model>"),
        (TypeRef::Map(model()), "::std::collections::HashMap<::std::string::String, crate::types::Model>"),
        (
            TypeRef::KeyedMap { key: "crate::types::Key".into(), value: Box::new(TypeRef::Primitive(PrimitiveType::U32)) },
            "::std::collections::HashMap<crate::types::Key, u32>",
        ),
        (TypeRef::Optional(Box::new(TypeRef::Array(model()))), "::std::option::Option<::std::vec::Vec<crate::types::Model>>"),
        (TypeRef::Primitive(PrimitiveType::Json), "::serde_json::Value"),
        (TypeRef::Primitive(PrimitiveType::String), "::std::string::String"),
        (TypeRef::Primitive(PrimitiveType::Unit), "()"),
    ];
    for (reference, expected) in cases {
        let (api, endpoint) = wrapped_api(reference.clone());
        // prettyplease may break a long generic list with a trailing comma.
        let code = squash(&format_generated_code(&generate_request_struct(&api, &endpoint)).expect("parses"))
            .replace(",>", ">");
        assert!(code.contains(&squash(&format!("type Success = {expected};"))), "{reference:?}: {code}");
        assert!(code.contains(&squash(&format!("pub body: {expected},"))), "{reference:?}: {code}");
    }
}

#[test]
fn a_json_form_part_spells_its_rust_type_through_the_components() {
    use schematic_define::{FormField, TypeRef};
    let reference = TypeRef::Optional(Box::new(TypeRef::Named("Model".into())));
    let mut endpoint = make_endpoint(
        "Upload",
        RestMethod::Post,
        "/upload",
        Some(ApiRequest::form_data(vec![FormField::json("meta", reference)])),
    );
    endpoint.response = vec![SuccessShape::empty()];
    let mut api = crate::test_utils::api_for(&endpoint);
    api.schemas[0].rust_path = Some("crate::types::Model".into());

    let code = squash(&format_generated_code(&generate_request_struct(&api, &endpoint)).expect("parses"));
    assert!(code.contains(&squash("pub meta: ::std::option::Option<crate::types::Model>,")), "{code}");
}

#[test]
fn an_unresolved_reference_is_spelled_as_a_compile_error() {
    let endpoint = make_endpoint("Get", RestMethod::Get, "/get", None);
    let mut api = crate::test_utils::api_for(&endpoint);
    api.schemas.clear();

    let code = format_generated_code(&generate_request_struct(&api, &endpoint)).expect("parses");
    assert!(code.contains("::core::compile_error!"), "{code}");
    assert!(code.contains("TestResponse"), "{code}");
}

// ---- typed path parameters, deprecation, metadata, and auth docs ----

use schematic_define::metadata::{Backoff, RateLimit, ResetUnit, RetryGuidance, StatusSelector};
use schematic_define::{
    ApiKeyEnv, AuthMethod, AuthPolicy, EndpointAuth, EndpointParams, EnvAuthStrategy, EnvList,
    EnvMapping, ParamDef,
};

/// `endpoint` with `path` declared as its typed path parameters.
fn with_typed_paths(mut endpoint: Endpoint, path: Vec<ParamDef>) -> Endpoint {
    endpoint.params = Some(EndpointParams { path, query: vec![], header: vec![], cookie: vec![] });
    endpoint
}

fn generated(endpoint: &Endpoint) -> String {
    format_generated_code(&generate_request_struct(&crate::test_utils::api_for(endpoint), endpoint))
        .expect("generated request parses")
}

/// One typed path parameter of every scalar kind, an enum, and a string.
fn every_kind_endpoint() -> Endpoint {
    with_typed_paths(
        make_endpoint("GetItem", RestMethod::Get, "/items/{id}/{flag}/{ratio}/{kind}/{name}", None),
        vec![
            ParamDef::path("id", QueryParamType::Integer).describes("The item id."),
            ParamDef::path("flag", QueryParamType::Boolean),
            ParamDef::path("ratio", QueryParamType::Number),
            ParamDef::path("kind", QueryParamType::Enum(vec!["asc".into(), "desc-order".into()])),
            ParamDef::path("name", QueryParamType::String),
        ],
    )
}

#[test]
fn typed_path_parameters_become_fields_of_their_declared_types() {
    let code = generated(&every_kind_endpoint());

    assert!(code.contains("pub id: i64,"), "{code}");
    assert!(code.contains("pub flag: bool,"), "{code}");
    assert!(code.contains("pub ratio: f64,"), "{code}");
    assert!(code.contains("pub kind: GetItemRequestKind,"), "{code}");
    assert!(code.contains("pub name: String,"), "{code}");
    assert!(code.contains("/// The item id.\n    pub id: i64,"), "the declared description is the field doc: {code}");
    assert!(code.contains("/// Path parameter: flag\n    pub flag: bool,"), "{code}");
}

#[test]
fn typed_path_values_are_converted_to_their_wire_spelling_before_encoding() {
    let code = squash(&generated(&every_kind_endpoint()));

    assert!(
        code.contains(&squash(
            r#"let path = format!("/items/{}/{}/{}/{}/{}",
                urlencoding::encode(&self.id.to_string()),
                urlencoding::encode(&self.flag.to_string()),
                urlencoding::encode(&self.ratio.to_string()),
                urlencoding::encode(self.kind.as_str()),
                urlencoding::encode(&self.name));"#
        )),
        "{code}"
    );
}

#[test]
fn a_non_finite_number_path_value_is_an_invalid_argument_before_the_path_is_built() {
    let code = squash(&generated(&every_kind_endpoint()));

    let check = squash(
        r#"if !self.ratio.is_finite() {
            return Err(schematic_runtime::InvalidArgument::new("path parameter `ratio` must be a finite number"));
        }"#,
    );
    let check_at = code.find(&check).unwrap_or_else(|| panic!("finite check missing: {code}"));
    let path_at = code.find("letpath=format!").expect("path is built");
    assert!(check_at < path_at, "the check runs before the path is built: {code}");
    assert_eq!(code.matches("is_finite()").count(), 1, "only the number is checked: {code}");
}

#[test]
fn typed_path_constructor_takes_strings_by_into_and_other_values_by_type() {
    let code = generated(&every_kind_endpoint());

    assert!(
        squash(&code).contains(&squash(
            "pub fn new(id: i64, flag: bool, ratio: f64, kind: GetItemRequestKind, name: impl Into<String>,) -> Self {
                Self { id, flag, ratio, kind, name: name.into(), }
            }"
        )),
        "{code}"
    );
}

#[test]
fn constructor_follows_template_order_not_declaration_order() {
    let endpoint = with_typed_paths(
        make_endpoint("GetCell", RestMethod::Get, "/rows/{row}/columns/{column}", None),
        vec![
            ParamDef::path("column", QueryParamType::String),
            ParamDef::path("row", QueryParamType::Integer),
        ],
    );

    let code = generated(&endpoint);

    assert!(code.contains("pub fn new(row: i64, column: impl Into<String>) -> Self"), "{code}");
    assert!(code.find("pub row: i64").unwrap() < code.find("pub column: String").unwrap(), "{code}");
}

#[test]
fn a_repeated_placeholder_is_one_field_used_at_every_occurrence() {
    let endpoint = with_typed_paths(
        make_endpoint("CopyItem", RestMethod::Post, "/items/{id}/copies/{id}", None),
        vec![ParamDef::path("id", QueryParamType::Integer)],
    );

    let code = generated(&endpoint);

    assert_eq!(code.matches("pub id: i64").count(), 1, "{code}");
    assert!(code.contains("pub fn new(id: i64) -> Self"), "{code}");
    assert!(
        squash(&code).contains(&squash(
            r#"format!("/items/{}/copies/{}", urlencoding::encode(&self.id.to_string()), urlencoding::encode(&self.id.to_string()))"#
        )),
        "{code}"
    );
}

#[test]
fn a_reserved_typed_occurrence_keeps_its_raw_value() {
    let endpoint = with_typed_paths(
        make_endpoint("GetFile", RestMethod::Get, "/files/{+file_path}/{file_path}/{+mode}/{+depth}", None),
        vec![
            ParamDef::path("file_path", QueryParamType::String),
            ParamDef::path("mode", QueryParamType::Enum(vec!["raw".into()])),
            ParamDef::path("depth", QueryParamType::Integer),
        ],
    );

    let code = generated(&endpoint);

    assert!(
        squash(&code).contains(&squash(
            r#"format!("/files/{}/{}/{}/{}", self.file_path, urlencoding::encode(&self.file_path), self.mode.as_str(), self.depth)"#
        )),
        "each occurrence decides its own encoding: {code}"
    );
}

#[test]
fn an_enum_path_parameter_generates_a_non_exhaustive_wire_spelled_enum() {
    let code = generated(&every_kind_endpoint());

    assert!(code.contains("#[non_exhaustive]\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]\npub enum GetItemRequestKind {"), "{code}");
    assert!(code.contains("#[serde(rename = \"asc\")]\n    Asc,"), "{code}");
    assert!(code.contains("#[serde(rename = \"desc-order\")]\n    DescOrder,"), "{code}");
    assert!(code.contains("Self::DescOrder => \"desc-order\","), "{code}");
    assert!(code.contains("impl ::core::fmt::Display for GetItemRequestKind"), "{code}");
}

#[test]
fn the_enum_name_carries_the_resolved_request_suffix_and_the_rust_name() {
    let endpoint = with_typed_paths(
        make_endpoint("GetItem", RestMethod::Get, "/items/{sortOrder}", None),
        vec![ParamDef::path("sortOrder", QueryParamType::Enum(vec!["a".into()])).rust_name("order")],
    );

    let code = format_generated_code(&generate_request_struct_with_suffix(
        &crate::test_utils::api_for(&endpoint),
        &endpoint,
        "Params",
    ))
    .unwrap();

    assert!(code.contains("pub enum GetItemParamsOrder {"), "{code}");
    assert!(code.contains("#[serde(rename = \"sortOrder\")]\n    pub order: GetItemParamsOrder,"), "{code}");
    assert!(code.contains("urlencoding::encode(self.order.as_str())"), "{code}");
}

#[test]
fn a_typed_path_request_has_no_default_or_string_conversion_and_legacy_keeps_both() {
    let typed = with_typed_paths(
        make_endpoint("GetModel", RestMethod::Get, "/models/{model}", None),
        vec![ParamDef::path("model", QueryParamType::String)],
    );
    let legacy = make_endpoint("GetModel", RestMethod::Get, "/models/{model}", None);

    let typed_code = generated(&typed);
    let legacy_code = generated(&legacy);

    assert!(typed_code.contains("#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct GetModelRequest"), "{typed_code}");
    assert!(!typed_code.contains("Default"), "{typed_code}");
    assert!(!typed_code.contains("impl From<"), "{typed_code}");
    assert!(legacy_code.contains("#[derive(Debug, Clone, Default, Serialize, Deserialize)]"), "{legacy_code}");
    assert!(legacy_code.contains("impl From<&str> for GetModelRequest"), "{legacy_code}");
}

#[test]
fn a_typed_path_doc_example_calls_new_with_typed_arguments() {
    let code = format_generated_code(&generate_request_struct_with_options(
        &crate::test_utils::api_for(&every_kind_endpoint()),
        &every_kind_endpoint(),
        "Request",
        Some("fixture"),
        "FixtureApi",
    ))
    .unwrap();

    assert!(code.contains("/// use schematic_schema::fixture::{GetItemRequest, GetItemRequestKind};"), "{code}");
    assert!(
        code.contains("/// let request = GetItemRequest::new(1, true, 1.5, GetItemRequestKind::Asc, \"name_value\")"),
        "{code}"
    );
    assert!(!code.contains("GetItemRequest::default()"), "{code}");
}

#[test]
fn deprecation_is_documented_on_requests_fields_and_setters_without_attributes() {
    let mut endpoint = with_typed_paths(
        make_endpoint("GetOld", RestMethod::Get, "/old/{id}", None),
        vec![ParamDef::path("id", QueryParamType::Integer).deprecated()],
    );
    endpoint.deprecated = true;
    let params = endpoint.params.as_mut().unwrap();
    params.query = vec![ParamDef::new("legacy", QueryParamType::String).deprecated()];
    params.header = vec![ParamDef::new("X-Old", QueryParamType::String).deprecated()];
    params.cookie = vec![ParamDef::new("old_session", QueryParamType::String).deprecated()];

    let code = generated(&endpoint);

    assert!(code.contains("/// Request for `GetOld` endpoint.\n///\n/// Deprecated.\n"), "{code}");
    assert!(code.contains("/// Path parameter: id\n    ///\n    /// Deprecated.\n    pub id: i64,"), "{code}");
    assert!(code.contains("/// Query parameter: legacy\n    ///\n    /// Deprecated.\n    pub legacy:"), "{code}");
    assert!(code.contains("///\n    /// Deprecated.\n    #[serde(rename = \"X-Old\")]"), "{code}");
    assert!(code.contains("///\n    /// Deprecated.\n    pub old_session:"), "{code}");
    for setter in ["with_legacy", "with_x_old", "with_old_session"] {
        assert!(code.contains(&format!("///\n    /// Deprecated.\n    pub fn {setter}(")), "{setter}: {code}");
    }
    assert!(!code.contains("#[deprecated"), "{code}");
}

fn github_rate_limit() -> RateLimit {
    RateLimit::new("x-ratelimit-limit", "x-ratelimit-remaining", "x-ratelimit-reset", ResetUnit::EpochSeconds)
        .unwrap()
        .with_retry_after_header("retry-after")
        .unwrap()
}

#[test]
fn every_rate_limit_and_retry_guidance_field_is_documented_on_the_request() {
    let endpoint = make_endpoint("ListRepos", RestMethod::Get, "/repos", None);
    let mut api = crate::test_utils::api_for(&endpoint);
    api.rate_limit = Some(github_rate_limit());
    api.endpoints[0].retry_guidance = Some(
        RetryGuidance::new(
            [StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 599 }],
            true,
            3,
            Backoff::Exponential { base_ms: 250, cap_ms: 8000 },
        )
        .unwrap(),
    );

    let code = format_generated_code(&generate_request_struct(&api, &api.endpoints[0])).unwrap();

    let expected = "/// ## Rate limit
///
/// Response headers reporting the rate limit (documentation only; the client does not act on them):
///
/// - limit: `x-ratelimit-limit`
/// - remaining: `x-ratelimit-remaining`
/// - reset: `x-ratelimit-reset`, as Unix time in seconds
/// - retry after: `retry-after`
///
/// ## Retry guidance
///
/// The provider's retry guidance (documentation only; the client does not retry):
///
/// - retry on: `429`, `500`-`599`
/// - honor Retry-After: yes
/// - max attempts: 3, including the first
/// - backoff: exponential from 250 ms, capped at 8000 ms
///
/// ## Example";
    assert!(code.contains(expected), "{code}");
}

#[test]
fn an_endpoint_retry_guidance_replaces_the_api_value_in_its_docs() {
    let endpoint = make_endpoint("Ping", RestMethod::Get, "/ping", None);
    let mut api = crate::test_utils::api_for(&endpoint);
    api.retry_guidance = Some(RetryGuidance::new([], false, 1, Backoff::None).unwrap());
    api.endpoints[0].retry_guidance =
        Some(RetryGuidance::new([StatusSelector::Code(503)], false, 2, Backoff::Fixed { delay_ms: 500 }).unwrap());

    let replaced = format_generated_code(&generate_request_struct(&api, &api.endpoints[0])).unwrap();
    api.endpoints[0].retry_guidance = None;
    let inherited = format_generated_code(&generate_request_struct(&api, &api.endpoints[0])).unwrap();

    assert!(replaced.contains("/// - retry on: `503`\n/// - honor Retry-After: no\n/// - max attempts: 2, including the first\n/// - backoff: fixed, 500 ms"), "{replaced}");
    assert!(inherited.contains("/// - retry on: no status\n/// - honor Retry-After: no\n/// - max attempts: 1, including the first\n/// - backoff: none, retry immediately"), "{inherited}");
    assert!(!inherited.contains("## Rate limit"), "{inherited}");
}

#[test]
fn endpoint_auth_overrides_are_documented_and_inherited_auth_adds_nothing() {
    let endpoint = make_endpoint("Health", RestMethod::Get, "/health", None);
    let mut api = crate::test_utils::api_for(&endpoint);
    let inherited = format_generated_code(&generate_request_struct(&api, &api.endpoints[0])).unwrap();

    api.endpoints[0].auth = Some(EndpointAuth::Public);
    let public = format_generated_code(&generate_request_struct(&api, &api.endpoints[0])).unwrap();

    api.endpoints[0].auth = Some(EndpointAuth::Policy {
        policy: AuthPolicy {
            explicit: vec![
                AuthMethod::ApiKey { header: "X-Admin-Key".into(), value_prefix: None },
                AuthMethod::BearerToken { header: None },
            ],
            env_fallback: Some(EnvAuthStrategy::ApiKey { header: "X-Admin-Key".into(), value_prefix: None }),
        },
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::new(vec!["ADMIN_KEY".into(), "FALLBACK_ADMIN_KEY".into()]),
                header: "X-Admin-Key".into(),
            }),
            ..EnvMapping::default()
        }),
    });
    let replaced = format_generated_code(&generate_request_struct(&api, &api.endpoints[0])).unwrap();

    assert!(!inherited.contains("## Authentication"), "{inherited}");
    assert!(
        public.contains("/// ## Authentication\n///\n/// This endpoint is public: it sends no credential.\n"),
        "{public}"
    );
    assert!(
        replaced.contains(
            "/// This endpoint replaces the API's authentication. It accepts an API key in the \
             `X-Admin-Key` header, a bearer token in the `Authorization` header. Without a supplied \
             credential, it reads an API key for the `X-Admin-Key` header from `ADMIN_KEY` or \
             `FALLBACK_ADMIN_KEY`.\n"
        ),
        "{replaced}"
    );
}

#[test]
fn a_definition_adopting_nothing_new_generates_unchanged_docs() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);

    let code = generated(&endpoint);

    assert!(code.contains("/// Request for `ListModels` endpoint.\n///\n/// ## Example\n"), "{code}");
}

/// The doc example's code, between its fence lines.
fn doc_example_source(code: &str) -> String {
    let lines: Vec<&str> = code.lines().map(|line| line.trim_start().trim_start_matches("///")).collect();
    let start = lines.iter().position(|line| line.trim() == "```no_run").expect("a no_run example");
    lines[start + 1..].iter().take_while(|line| line.trim() != "```").copied().collect::<Vec<_>>().join("\n")
}

#[test]
fn optional_parameters_get_setter_calls_with_values_of_their_types() {
    let mut endpoint = endpoint_with_named_params(
        vec![named_param("X-Trace-Id", QueryParamType::String), named_param("X-Ids", QueryParamType::Array(Box::new(QueryParamType::Integer)))],
        vec![named_param("session", QueryParamType::Boolean)],
    );
    let mut query = vec![
        named_param("q", QueryParamType::String),
        named_param("order", QueryParamType::Enum(vec!["asc".into(), "desc".into()])),
        named_param("limit", QueryParamType::Integer),
    ];
    for param in &mut query {
        param.required = false;
    }
    endpoint.params.as_mut().unwrap().query = query;
    let code = format_generated_code(&generate_request_struct_with_options(
        &crate::test_utils::api_for(&endpoint),
        &endpoint,
        "Request",
        Some("fixture"),
        "Api",
    ))
    .unwrap();
    let source = doc_example_source(&code);

    assert!(source.contains("use schematic_schema::fixture::{SearchRequest, SearchRequestOrder};"), "{source}");
    for setter in [
        r#".with_q("q_value".to_string())"#,
        ".with_order(SearchRequestOrder::Asc)",
        ".with_limit(1)",
        r#".with_x_trace_id("X-Trace-Id_value".to_string())"#,
        ".with_x_ids(vec![1])",
        ".with_session(true);",
    ] {
        assert!(source.contains(setter), "missing `{setter}`:\n{source}");
    }
    assert!(!source.contains("/* value */"), "{source}");
    syn::parse_file(&format!("fn example() {{\n{}\n}}", source.replace("use ", "// use ")))
        .unwrap_or_else(|error| panic!("invalid example: {error}\n{source}"));
}
