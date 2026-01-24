//! Request struct generation for API endpoints.
//!
//! Generates Rust structs that encapsulate path parameters and request bodies
//! for each API endpoint, with methods to convert them into HTTP request parts.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::Endpoint;

use crate::parser::extract_path_params;

/// Default suffix for request struct names.
const DEFAULT_REQUEST_SUFFIX: &str = "Request";

/// Generates a request struct for the given endpoint.
///
/// The generated struct includes:
/// - String fields for each path parameter (e.g., `{model}` becomes `pub model: String`)
/// - A `body` field if the endpoint has a request schema
/// - `Default` trait implementation
/// - `into_parts()` method that returns `Result<RequestParts, SchematicError>`
///
/// ## Examples
///
/// For a GET endpoint with path parameters:
/// ```ignore
/// // Input endpoint:
/// Endpoint {
///     id: "RetrieveModel",
///     path: "/models/{model}",
///     method: RestMethod::Get,
///     request: None,
///     headers: vec![],
///     ...
/// }
///
/// // Generated struct:
/// #[derive(Debug, Clone, Default)]
/// pub struct RetrieveModelRequest {
///     pub model: String,
/// }
///
/// impl RetrieveModelRequest {
///     pub fn into_parts(self) -> Result<RequestParts, SchematicError> {
///         let path = format!("/models/{}", self.model);
///         Ok(("GET", path, None, vec![]))
///     }
/// }
/// ```
pub fn generate_request_struct(endpoint: &Endpoint) -> TokenStream {
    generate_request_struct_with_suffix(endpoint, DEFAULT_REQUEST_SUFFIX)
}

/// Generates a request struct for the given endpoint with a custom suffix.
///
/// This variant allows specifying a custom suffix instead of the default "Request".
/// For example, using "Params" would generate `ListModelsParams` instead of `ListModelsRequest`.
///
/// ## Arguments
///
/// * `endpoint` - The endpoint to generate a request struct for
/// * `suffix` - The suffix to append to the endpoint ID (e.g., "Request", "Params")
pub fn generate_request_struct_with_suffix(endpoint: &Endpoint, suffix: &str) -> TokenStream {
    generate_request_struct_with_options(endpoint, suffix, None)
}

/// Generates a request struct for the given endpoint with full configuration.
///
/// This variant allows specifying both the suffix and the module path for use imports.
///
/// ## Arguments
///
/// * `endpoint` - The endpoint to generate a request struct for
/// * `suffix` - The suffix to append to the endpoint ID (e.g., "Request", "Params")
/// * `module_path` - Optional module path for use in examples (e.g., "openai")
pub fn generate_request_struct_with_options(
    endpoint: &Endpoint,
    suffix: &str,
    module_path: Option<&str>,
) -> TokenStream {
    use schematic_define::ApiRequest;

    let struct_name = format_ident!("{}{}", endpoint.id, suffix);
    let struct_name_str = format!("{}{}", endpoint.id, suffix);
    let path_params = extract_path_params(&endpoint.path);
    // Only JSON requests have a typed body field
    let has_body = matches!(&endpoint.request, Some(ApiRequest::Json(_)));
    let method_str = endpoint.method.to_string();

    // Extract body type name for new() constructor
    let body_type_name = match &endpoint.request {
        Some(ApiRequest::Json(schema)) => Some(schema.type_name.as_str()),
        _ => None,
    };

    // Generate struct fields
    let param_fields = generate_param_fields(&path_params);
    let body_field = generate_body_field(endpoint);

    // Generate derives (Default only if no body or body type implements Default)
    let derives = generate_derives(has_body);

    // Generate Default impl if we have a body (manual impl needed)
    let default_impl = generate_default_impl(&struct_name, &path_params, has_body);

    // Generate new() constructor for type-safe construction
    let new_impl = generate_new_impl(&struct_name, &path_params, has_body, body_type_name);

    // Generate into_parts method
    let into_parts = generate_into_parts(endpoint, &path_params, &method_str);

    // Generate doc comments with example section
    let doc_lines = generate_doc_comment_with_example(
        &endpoint.id,
        &struct_name_str,
        &path_params,
        has_body,
        body_type_name,
        module_path,
    );

    // Combine all fields
    let all_fields = if has_body {
        quote! {
            #param_fields
            #body_field
        }
    } else {
        param_fields
    };

    quote! {
        #(#[doc = #doc_lines])*
        #derives
        pub struct #struct_name {
            #all_fields
        }

        #default_impl

        #new_impl

        impl #struct_name {
            #into_parts
        }
    }
}

/// Generates doc comment lines with an example section.
///
/// The example shows how to instantiate the request struct:
/// - For structs with no required fields: uses `Default::default()`
/// - For structs with path params only: uses `new()` with params
/// - For structs with body: uses `new()` with body (and optional params)
///
/// ## Arguments
///
/// * `endpoint_id` - The endpoint identifier (e.g., "CreateChatCompletion")
/// * `struct_name` - The full struct name (e.g., "CreateChatCompletionRequest")
/// * `path_params` - List of path parameter names
/// * `has_body` - Whether the struct has a body field
/// * `body_type` - The body type name, if any
/// * `module_path` - Optional module path for imports (e.g., "openai")
///
/// ## Returns
///
/// A vector of doc comment lines (each with leading space for proper `///` formatting).
fn generate_doc_comment_with_example(
    endpoint_id: &str,
    struct_name: &str,
    path_params: &[&str],
    has_body: bool,
    body_type: Option<&str>,
    module_path: Option<&str>,
) -> Vec<String> {
    let mut lines = vec![format!(" Request for `{}` endpoint.", endpoint_id)];

    // Add blank line before example section
    lines.push(String::new());
    lines.push(" ## Example".to_string());
    lines.push(String::new());
    lines.push(" ```ignore".to_string());

    // Build the module path for use statement
    let mod_path = module_path.unwrap_or("api");

    // Generate appropriate example based on struct configuration
    if has_body {
        let body_ty = body_type.unwrap_or("Body");

        // Import both the request struct and body type
        lines.push(format!(
            " use schematic_schema::{mod_path}::{{{struct_name}, {body_ty}}};"
        ));
        lines.push(String::new());

        // Show body construction with ..Default::default() pattern
        lines.push(format!(" let body = {body_ty} {{"));
        lines.push("     // ... set required fields ...".to_string());
        lines.push("     ..Default::default()".to_string());
        lines.push(" };".to_string());

        // Show request construction
        if path_params.is_empty() {
            lines.push(format!(" let request = {struct_name}::new(body);"));
        } else {
            // Include path params in new() call
            let param_args: Vec<String> = path_params
                .iter()
                .map(|p| format!("\"{p}_value\""))
                .collect();
            let args = param_args.join(", ");
            lines.push(format!(" let request = {struct_name}::new({args}, body);"));
        }
    } else if !path_params.is_empty() {
        // Path params only - use new()
        lines.push(format!(" use schematic_schema::{mod_path}::{struct_name};"));
        lines.push(String::new());

        let param_args: Vec<String> = path_params
            .iter()
            .map(|p| format!("\"{p}_value\""))
            .collect();
        let args = param_args.join(", ");
        lines.push(format!(" let request = {struct_name}::new({args});"));
    } else {
        // No required fields - use default()
        lines.push(format!(" use schematic_schema::{mod_path}::{struct_name};"));
        lines.push(String::new());
        lines.push(format!(" let request = {struct_name}::default();"));
    }

    lines.push(" ```".to_string());

    lines
}

/// Generates field declarations for path parameters.
fn generate_param_fields(path_params: &[&str]) -> TokenStream {
    let fields = path_params.iter().map(|param| {
        let field_name = format_ident!("{}", param);
        // Leading space for proper /// formatting
        let doc = format!(" Path parameter: {}", param);
        quote! {
            #[doc = #doc]
            pub #field_name: String,
        }
    });

    quote! { #(#fields)* }
}

/// Generates the body field if the endpoint has a request schema.
fn generate_body_field(endpoint: &Endpoint) -> TokenStream {
    use schematic_define::ApiRequest;

    match &endpoint.request {
        Some(ApiRequest::Json(schema)) => {
            let type_name = format_ident!("{}", schema.type_name);
            quote! {
                /// Request body
                pub body: #type_name,
            }
        }
        // FormData, UrlEncoded, Text, Binary don't have a typed body field
        // The generated code will handle these differently
        Some(ApiRequest::FormData { .. })
        | Some(ApiRequest::UrlEncoded { .. })
        | Some(ApiRequest::Text { .. })
        | Some(ApiRequest::Binary { .. }) => quote! {},
        None => quote! {},
    }
}

/// Generates derive attributes for the struct.
fn generate_derives(has_body: bool) -> TokenStream {
    if has_body {
        // With body, we don't derive Default (we implement it manually)
        quote! { #[derive(Debug, Clone, Serialize, Deserialize)] }
    } else {
        // Without body, we can derive Default
        quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] }
    }
}

/// Generates the Default implementation for structs with a body.
fn generate_default_impl(
    struct_name: &proc_macro2::Ident,
    path_params: &[&str],
    has_body: bool,
) -> TokenStream {
    if !has_body {
        // Default is derived, no manual impl needed
        return quote! {};
    }

    let param_defaults = path_params.iter().map(|param| {
        let field_name = format_ident!("{}", param);
        quote! { #field_name: String::new(), }
    });

    quote! {
        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#param_defaults)*
                    body: Default::default(),
                }
            }
        }
    }
}

/// Generates a `new()` constructor for the request struct.
///
/// The constructor requires all path parameters and the body (if present),
/// providing compile-time enforcement of required fields.
///
/// ## Returns
///
/// - For structs with path params or body: `impl` block with `new()` method
/// - For empty structs (no params, no body): empty `TokenStream` (use `Default`)
fn generate_new_impl(
    struct_name: &proc_macro2::Ident,
    path_params: &[&str],
    has_body: bool,
    body_type: Option<&str>,
) -> TokenStream {
    let params: Vec<_> = path_params
        .iter()
        .map(|p| {
            let name = format_ident!("{}", p);
            quote! { #name: impl Into<String> }
        })
        .collect();

    let field_inits: Vec<_> = path_params
        .iter()
        .map(|p| {
            let name = format_ident!("{}", p);
            quote! { #name: #name.into() }
        })
        .collect();

    if has_body {
        let body_ty = format_ident!("{}", body_type.unwrap());
        quote! {
            impl #struct_name {
                /// Creates a new request with the required path parameters and body.
                pub fn new(#(#params,)* body: #body_ty) -> Self {
                    Self {
                        #(#field_inits,)*
                        body,
                    }
                }
            }
        }
    } else if !path_params.is_empty() {
        quote! {
            impl #struct_name {
                /// Creates a new request with the required path parameters.
                pub fn new(#(#params),*) -> Self {
                    Self {
                        #(#field_inits,)*
                    }
                }
            }
        }
    } else {
        // No params, no body - new() is just Default
        quote! {}
    }
}

/// Generates the into_parts method.
fn generate_into_parts(endpoint: &Endpoint, path_params: &[&str], method_str: &str) -> TokenStream {
    use schematic_define::ApiRequest;

    let path_format = generate_path_format(&endpoint.path, path_params);
    // Only JSON requests have a typed body field
    let has_json_body = matches!(&endpoint.request, Some(ApiRequest::Json(_)));

    let body_expr = if has_json_body {
        quote! {
            Some(serde_json::to_string(&self.body).map_err(|e| {
                SchematicError::SerializationError(e.to_string())
            })?)
        }
    } else {
        quote! { None }
    };

    // Generate headers initialization
    let headers_init = generate_endpoint_headers_init(&endpoint.headers);

    quote! {
        /// Converts the request into (method, path, body, headers) parts.
        ///
        /// ## Returns
        ///
        /// A tuple of:
        /// - HTTP method as a static string (e.g., "GET", "POST")
        /// - Fully substituted path string
        /// - Optional JSON body string
        /// - Endpoint-specific headers as key-value pairs
        ///
        /// ## Errors
        ///
        /// Returns `SchematicError::SerializationError` if the request body
        /// fails to serialize to JSON.
        pub fn into_parts(self) -> Result<RequestParts, SchematicError> {
            #path_format
            Ok((#method_str, path, #body_expr, #headers_init))
        }
    }
}

/// Generates the initialization code for endpoint-specific headers.
fn generate_endpoint_headers_init(headers: &[(String, String)]) -> TokenStream {
    if headers.is_empty() {
        quote! { vec![] }
    } else {
        let header_pairs = headers.iter().map(|(k, v)| {
            quote! { (#k.to_string(), #v.to_string()) }
        });
        quote! { vec![#(#header_pairs),*] }
    }
}

/// Generates the path format expression.
fn generate_path_format(path: &str, path_params: &[&str]) -> TokenStream {
    if path_params.is_empty() {
        let path_literal = path;
        quote! { let path = #path_literal.to_string(); }
    } else {
        // Build format string and arguments
        let format_str = build_format_string(path, path_params);
        let format_args = path_params.iter().map(|param| {
            let field_name = format_ident!("{}", param);
            quote! { self.#field_name }
        });

        quote! { let path = format!(#format_str, #(#format_args),*); }
    }
}

/// Builds a format string by replacing {param} with {}.
fn build_format_string(path: &str, path_params: &[&str]) -> String {
    let mut result = path.to_string();
    for param in path_params {
        let placeholder = format!("{{{}}}", param);
        result = result.replace(&placeholder, "{}");
    }
    result
}

/// Validates that the generated code is syntactically correct.
///
/// ## Errors
///
/// Returns an error string if the generated code fails to parse.
pub fn validate_generated_code(tokens: &TokenStream) -> Result<(), String> {
    syn::parse2::<syn::File>(tokens.clone()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Formats generated code using prettyplease.
///
/// ## Errors
///
/// Returns an error string if the code fails to parse.
pub fn format_generated_code(tokens: &TokenStream) -> Result<String, String> {
    let file = syn::parse2::<syn::File>(tokens.clone()).map_err(|e| e.to_string())?;
    Ok(prettyplease::unparse(&file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiRequest, ApiResponse, RestMethod};

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
        }
    }

    #[test]
    fn generate_get_no_params() {
        let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
        let tokens = generate_request_struct(&endpoint);

        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("pub struct ListModelsRequest"));
        assert!(code.contains("#[derive(Debug, Clone, Default, Serialize, Deserialize)]"));
        assert!(code.contains("into_parts"));
        assert!(code.contains("Result<"));
        assert!(code.contains("SchematicError"));
        assert!(code.contains(r#""GET""#));
        assert!(code.contains(r#""/models".to_string()"#));
    }

    #[test]
    fn generate_get_with_path_param() {
        let endpoint = make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None);
        let tokens = generate_request_struct(&endpoint);

        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("pub struct RetrieveModelRequest"));
        assert!(code.contains("pub model: String"));
        assert!(code.contains(r#"format!("/models/{}", self.model)"#));
        assert!(code.contains(r#""GET""#));
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
        assert!(
            code.contains(r#"format!("/threads/{}/messages/{}", self.thread_id, self.message_id)"#)
        );
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
        assert!(code.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
        assert!(code.contains("pub body: CreateCompletionRequest"));
        assert!(code.contains("impl Default for CreateCompletionRequest"));
        assert!(code.contains(r#""POST""#));
        assert!(code.contains("serde_json::to_string(&self.body)"));
        assert!(code.contains(".map_err"));
        assert!(code.contains("SerializationError"));
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
        assert!(code.contains("impl Default for CreateMessageRequest"));
        assert!(code.contains("thread_id: String::new()"));
    }

    #[test]
    fn generate_delete_with_path_param() {
        let endpoint = make_endpoint("DeleteModel", RestMethod::Delete, "/models/{model}", None);
        let tokens = generate_request_struct(&endpoint);

        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("pub struct DeleteModelRequest"));
        assert!(code.contains("pub model: String"));
        assert!(code.contains(r#""DELETE""#));
        assert!(code.contains("None")); // No body for DELETE
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
        assert!(code.contains(r#""PATCH""#));
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
                code.contains(&format!(r#""{}""#, expected_str)),
                "Expected method {} not found in generated code",
                expected_str
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
            code.contains(
                "pub fn new(thread_id: impl Into<String>, body: CreateMessageBody) -> Self"
            ),
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
            code.contains("```ignore"),
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
        let tokens = generate_request_struct_with_options(&endpoint, "Request", Some("openai"));
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
}
