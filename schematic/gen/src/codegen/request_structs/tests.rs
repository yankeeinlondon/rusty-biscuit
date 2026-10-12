use super::*;
use schematic_define::{ApiRequest, ApiResponse, QueryParamType, RestMethod};

fn make_endpoint(
    id: &str,
    method: RestMethod,
    path: &str,
    request: Option<ApiRequest>,
) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: format!("Test endpoint for {}", id),
        request,
        response: ApiResponse::json_type("TestResponse"),
        headers: vec![],
        params: None,
        oauth_scopes: None,
    }
}

#[test]
fn generate_get_no_params() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct CreateMessageRequest"));
    assert!(code.contains("pub thread_id: String"));
    assert!(code.contains("pub body: CreateMessageRequest"));
    assert!(code.contains("#[derive(Debug, Clone, Default, Serialize, Deserialize)]"));
}

#[test]
fn generate_delete_with_path_param() {
    let endpoint = make_endpoint("DeleteModel", RestMethod::Delete, "/models/{model}", None);
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub struct UpdateThreadRequest"));
    assert!(code.contains("pub thread_id: String"));
    assert!(code.contains("pub body: UpdateThreadRequest"));
    assert!(code.contains("schematic_runtime::Method::PATCH"), "{code}");
}

#[test]
fn validate_generated_code_passes() {
    let endpoint = make_endpoint("TestEndpoint", RestMethod::Get, "/test/{id}", None);
    let tokens = generate_request_struct(&endpoint);

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
    let tokens = generate_request_struct(&endpoint);

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
        let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    // Should be in ignore code block
    assert!(
        code.contains("```text"),
        "Expected ignore code fence, got:\n{}",
        code
    );
}

#[test]
fn generates_example_with_new_for_path_params() {
    let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    // Should show Default pattern for body
    assert!(
        code.contains("..Default::default()"),
        "Expected ..Default::default() pattern, got:\n{}",
        code
    );
    // Should show body construction
    assert!(
        code.contains("let body = CreateCompletionBody {"),
        "Expected body construction, got:\n{}",
        code
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct_with_options(&endpoint, "Request", Some("openai"), "OpenAI");
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
        Some(EndpointParams {
            query: query_params
                .into_iter()
                .map(|(name, required, desc, param_type)| ParamDef {
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
            pagination: None,
            response_pagination: None,
        })
    };

    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: format!("Test endpoint for {}", id),
        request,
        response: ApiResponse::json_type("TestResponse"),
        headers: vec![],
        params,
        oauth_scopes: None,
    }
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    // Pairs are collected apart from the path and returned unencoded
    assert!(
        code.contains("let mut query: Vec<(String, String)> = Vec::new()"),
        "Expected query collector, got:\n{}",
        code
    );
    assert!(
        code.contains(r#"query.push(("page".to_string(), value.to_string()))"#),
        "Expected page param push, got:\n{}",
        code
    );
    assert!(
        code.contains(r#"query.push(("limit".to_string(), value.to_string()))"#),
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
    let tokens = generate_request_struct(&endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub q: Option<String>"));
    // Enum becomes String
    assert!(code.contains("pub sort: Option<String>"));
    assert!(code.contains("pub flag: Option<bool>"));
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
    let tokens = generate_request_struct(&endpoint);
    let code = format_generated_code(&tokens).expect("Failed to format code");

    assert!(code.contains("pub tags: Option<Vec<String>>"));
    // Array params push multiple entries to collector
    assert!(
        code.contains("for value in values"),
        "Expected loop over array values, got:\n{}",
        code
    );
    assert!(
        code.contains(r#"query.push(("tags".to_string(), value.to_string()))"#),
        "Expected tags push in loop, got:\n{}",
        code
    );
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
    let tokens = generate_request_struct(&endpoint);
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
    let tokens = generate_request_struct(&endpoint);
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

// === Paginated trait tests ===
// NOTE: These tests are disabled until generate_paginated_trait and
// generate_paginated_impl functions are implemented as part of the
// pagination codegen feature.
//
// TODO(pagination-codegen): Uncomment these tests when implementing:
// - generate_paginated_trait()
// - generate_paginated_impl(&Endpoint, &str) -> TokenStream
/*
mod paginated_trait_tests {
    use super::*;
    use crate::codegen::generate_paginated_trait;
    use schematic_define::params::{EndpointParams, PaginationStyle};

    fn make_paginated_endpoint(id: &str, path: &str) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            method: RestMethod::Get,
            path: path.to_string(),
            description: format!("Test paginated endpoint for {}", id),
            request: None,
            response: ApiResponse::json_type("TestResponse"),
            headers: vec![],
            params: Some(
                EndpointParams::default().with_pagination(PaginationStyle::github()),
            ),
            oauth_scopes: None,
        }
    }

    fn make_non_paginated_endpoint(id: &str, path: &str) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            method: RestMethod::Get,
            path: path.to_string(),
            description: format!("Test non-paginated endpoint for {}", id),
            request: None,
            response: ApiResponse::json_type("TestResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }
    }

    #[test]
    fn generate_paginated_trait_produces_valid_code() {
        let tokens = generate_paginated_trait();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should generate the Paginated trait
        assert!(
            code.contains("pub trait Paginated"),
            "Should generate Paginated trait, got:\n{}",
            code
        );
        // Should have doc comment
        assert!(
            code.contains("Marker trait for paginated request types"),
            "Should have doc comment, got:\n{}",
            code
        );
    }

    #[test]
    fn generate_paginated_impl_for_paginated_endpoint() {
        use crate::codegen::generate_paginated_impl;

        let endpoint = make_paginated_endpoint("ListPullRequests", "/repos/{owner}/{repo}/pulls");
        let tokens = generate_paginated_impl(&endpoint, "Request");
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should generate impl Paginated for the request struct
        assert!(
            code.contains("impl Paginated for ListPullRequestsRequest"),
            "Should generate Paginated impl for paginated endpoint, got:\n{}",
            code
        );
    }

    #[test]
    fn skip_paginated_impl_for_non_paginated_endpoint() {
        use crate::codegen::generate_paginated_impl;

        let endpoint = make_non_paginated_endpoint("GetUser", "/users/{id}");
        let tokens = generate_paginated_impl(&endpoint, "Request");

        // Should produce empty TokenStream (no impl)
        assert!(
            tokens.is_empty(),
            "Should not generate Paginated impl for non-paginated endpoint, got: {}",
            tokens
        );
    }

    #[test]
    fn generate_paginated_impl_with_custom_suffix() {
        use crate::codegen::generate_paginated_impl;

        let endpoint = make_paginated_endpoint("ListItems", "/items");
        let tokens = generate_paginated_impl(&endpoint, "Params");
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Should use the custom suffix
        assert!(
            code.contains("impl Paginated for ListItemsParams"),
            "Should use custom suffix, got:\n{}",
            code
        );
    }

    #[test]
    fn endpoint_with_no_params_is_not_paginated() {
        use crate::codegen::generate_paginated_impl;

        let endpoint = Endpoint {
            id: "ListAll".to_string(),
            method: RestMethod::Get,
            path: "/all".to_string(),
            description: "List all".to_string(),
            request: None,
            response: ApiResponse::json_type("TestResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        };
        let tokens = generate_paginated_impl(&endpoint, "Request");

        assert!(
            tokens.is_empty(),
            "Endpoint with no params should not implement Paginated"
        );
    }

    #[test]
    fn endpoint_with_query_params_but_no_pagination_is_not_paginated() {
        use crate::codegen::generate_paginated_impl;
        use schematic_define::params::QueryParamType;

        let endpoint = Endpoint {
            id: "Search".to_string(),
            method: RestMethod::Get,
            path: "/search".to_string(),
            description: "Search".to_string(),
            request: None,
            response: ApiResponse::json_type("SearchResponse"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_query_param("q", QueryParamType::String, true, Some("Search query")),
            ),
            oauth_scopes: None,
        };
        let tokens = generate_paginated_impl(&endpoint, "Request");

        assert!(
            tokens.is_empty(),
            "Endpoint with query params but no pagination should not implement Paginated"
        );
    }
}
*/

// === Pagination metadata in doc comments tests ===

mod pagination_doc_tests {
    use super::*;
    use schematic_define::params::{EndpointParams, PaginationStyle};

    fn make_endpoint_with_pagination(id: &str, path: &str, style: PaginationStyle) -> Endpoint {
        let params = EndpointParams::default().with_pagination(style);

        Endpoint {
            id: id.to_string(),
            method: RestMethod::Get,
            path: path.to_string(),
            description: format!("Test endpoint for {}", id),
            request: None,
            response: ApiResponse::json_type("TestResponse"),
            headers: vec![],
            params: Some(params),
            oauth_scopes: None,
        }
    }

    #[test]
    fn github_pagination_docs_include_default_and_max_values() {
        let endpoint =
            make_endpoint_with_pagination("ListRepos", "/repos", PaginationStyle::github());
        let tokens = generate_request_struct(&endpoint);
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
            PaginationStyle::bitbucket(),
        );
        let tokens = generate_request_struct(&endpoint);
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
            make_endpoint_with_pagination("ListIssues", "/issues", PaginationStyle::gitea());
        let tokens = generate_request_struct(&endpoint);
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
            PaginationStyle::offset_limit("offset", "limit", 25, 200),
        );
        let tokens = generate_request_struct(&endpoint);
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
            PaginationStyle::cursor("after", Some("limit"), 20),
        );
        let tokens = generate_request_struct(&endpoint);
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
    let code = format_generated_code(&generate_request_struct(&endpoint)).unwrap();

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
    let code = format_generated_code(&generate_request_struct(&endpoint)).unwrap();

    assert!(code.contains("pub body: Vec<u8>,"), "{code}");
    assert!(code.contains("pub fn new(name: impl Into<String>, body: Vec<u8>) -> Self"), "{code}");
    assert!(
        code.contains(r#"content_type: "application/octet-stream".to_string()"#),
        "{code}"
    );
    // `Vec<u8>` converts into the runtime's byte buffer without copying.
    assert!(code.contains("bytes: self.body.into(),"), "{code}");
    assert!(validate_generated_code(&generate_request_struct(&endpoint)).is_ok());
}

// ---- header and cookie parameters (R25, R26) ----

/// `code` with all whitespace, and any trailing comma before `)` or `]`,
/// removed, so assertions survive line wrapping.
fn squash(code: &str) -> String {
    let flat: String = code.split_whitespace().collect();
    flat.replace(",)", ")").replace(",]", "]")
}

fn named_param(name: &str, param_type: QueryParamType) -> schematic_define::ParamDef {
    schematic_define::ParamDef {
        name: name.to_string(),
        required: true,
        description: None,
        param_type,
        explode: false,
        style: schematic_define::ParamStyle::Simple,
    }
}

fn endpoint_with_named_params(
    headers: Vec<schematic_define::ParamDef>,
    cookies: Vec<schematic_define::ParamDef>,
) -> Endpoint {
    Endpoint {
        headers: vec![("X-Mode".to_string(), "static".to_string())],
        params: Some(schematic_define::EndpointParams {
            header: headers,
            cookie: cookies,
            ..Default::default()
        }),
        ..make_endpoint("Search", RestMethod::Get, "/search", None)
    }
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
    let code = format_generated_code(&generate_request_struct(&endpoint)).unwrap();
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
    assert!(code.contains("let query: Vec<(String, String)> = Vec::new()"), "{code}");
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
    let code = format_generated_code(&generate_request_struct(&endpoint)).unwrap();
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
    let endpoint = Endpoint {
        path: "/search/{scope}".to_string(),
        ..endpoint_with_named_params(
            vec![named_param("X-Trace-Id", QueryParamType::String)],
            vec![named_param("session", QueryParamType::String)],
        )
    };
    let code = format_generated_code(&generate_request_struct(&endpoint)).unwrap();

    assert!(code.contains("x_trace_id: None"), "{code}");
    assert!(code.contains("session: None"), "{code}");
    assert!(code.contains("impl From<&str> for SearchRequest"), "{code}");
    assert!(validate_generated_code(&generate_request_struct(&endpoint)).is_ok());
}

#[test]
fn endpoints_without_named_parameters_keep_their_inline_headers() {
    let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
    let code = format_generated_code(&generate_request_struct(&endpoint)).unwrap();

    assert!(code.contains("let headers: Vec<(String, String)> = vec![];"), "{code}");
    assert!(!code.contains("let mut headers"), "{code}");
}
