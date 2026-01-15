//! Request struct generation for API endpoints.
//!
//! Generates Rust structs that encapsulate path parameters and request bodies
//! for each API endpoint, with methods to convert them into HTTP request parts.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::Endpoint;

use crate::parser::extract_path_params;

/// Generates a request struct for the given endpoint.
///
/// The generated struct includes:
/// - String fields for each path parameter (e.g., `{model}` becomes `pub model: String`)
/// - A `body` field if the endpoint has a request schema
/// - `Default` trait implementation
/// - `into_parts()` method that returns `(&'static str, String, Option<String>, Vec<(String, String)>)`
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
///     pub fn into_parts(self) -> Result<(&'static str, String, Option<String>, Vec<(String, String)>), SchematicError> {
///         let path = format!("/models/{}", self.model);
///         Ok(("GET", path, None, vec![]))
///     }
/// }
/// ```
pub fn generate_request_struct(endpoint: &Endpoint) -> TokenStream {
    let struct_name = format_ident!("{}Request", endpoint.id);
    let path_params = extract_path_params(&endpoint.path);
    let has_body = endpoint.request.is_some();
    let method_str = endpoint.method.to_string();

    // Generate struct fields
    let param_fields = generate_param_fields(&path_params);
    let body_field = generate_body_field(endpoint);

    // Generate derives (Default only if no body or body type implements Default)
    let derives = generate_derives(has_body);

    // Generate Default impl if we have a body (manual impl needed)
    let default_impl = generate_default_impl(&struct_name, &path_params, has_body);

    // Generate into_parts method
    let into_parts = generate_into_parts(endpoint, &path_params, &method_str);

    // Generate doc comment
    let doc_comment = format!("Request for {} endpoint.", endpoint.id);

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
        #[doc = #doc_comment]
        #derives
        pub struct #struct_name {
            #all_fields
        }

        #default_impl

        impl #struct_name {
            #into_parts
        }
    }
}

/// Generates field declarations for path parameters.
fn generate_param_fields(path_params: &[&str]) -> TokenStream {
    let fields = path_params.iter().map(|param| {
        let field_name = format_ident!("{}", param);
        let doc = format!("Path parameter: {}", param);
        quote! {
            #[doc = #doc]
            pub #field_name: String,
        }
    });

    quote! { #(#fields)* }
}

/// Generates the body field if the endpoint has a request schema.
fn generate_body_field(endpoint: &Endpoint) -> TokenStream {
    match &endpoint.request {
        Some(schema) => {
            let type_name = format_ident!("{}", schema.type_name);
            quote! {
                /// Request body
                pub body: #type_name,
            }
        }
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

/// Generates the into_parts method.
fn generate_into_parts(endpoint: &Endpoint, path_params: &[&str], method_str: &str) -> TokenStream {
    let path_format = generate_path_format(&endpoint.path, path_params);
    let has_body = endpoint.request.is_some();

    let body_expr = if has_body {
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
        pub fn into_parts(self) -> Result<(&'static str, String, Option<String>, Vec<(String, String)>), SchematicError> {
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
    use schematic_define::{ApiResponse, RestMethod, Schema};

    fn make_endpoint(
        id: &str,
        method: RestMethod,
        path: &str,
        request: Option<Schema>,
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
            Some(Schema::new("CreateCompletionRequest")),
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
            Some(Schema::new("CreateMessageRequest")),
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
            Some(Schema::new("UpdateThreadRequest")),
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
            Some(Schema::new("CreateIssueRequest")),
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
}
