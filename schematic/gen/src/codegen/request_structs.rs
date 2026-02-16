//! Request struct generation for API endpoints.
//!
//! Generates Rust structs that encapsulate path parameters, query parameters,
//! and request bodies for each API endpoint, with methods to convert them into
//! HTTP request parts.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{ApiResponse, Endpoint, EndpointParams, QueryParamType};

use crate::parser::extract_path_params;

/// Default suffix for request struct names.
const DEFAULT_REQUEST_SUFFIX: &str = "Request";

/// Extracts query parameters from an endpoint's params field.
fn extract_query_params(params: &Option<EndpointParams>) -> Vec<QueryParamInfo> {
    match params {
        Some(EndpointParams { query, .. }) => query
            .iter()
            .map(|p| QueryParamInfo {
                name: p.name.clone(),
                description: p.description.clone(),
                param_type: p.param_type.clone(),
            })
            .collect(),
        None => Vec::new(),
    }
}

/// Information about a query parameter for code generation.
#[derive(Debug, Clone)]
struct QueryParamInfo {
    name: String,
    description: Option<String>,
    param_type: QueryParamType,
}

/// Maps QueryParamType to a Rust type token stream.
fn query_param_type_to_rust_type(param_type: &QueryParamType) -> TokenStream {
    match param_type {
        QueryParamType::String => quote! { String },
        QueryParamType::Integer => quote! { i64 },
        QueryParamType::Number => quote! { f64 },
        QueryParamType::Boolean => quote! { bool },
        QueryParamType::Array(inner) => {
            let inner_type = query_param_type_to_rust_type(inner);
            quote! { Vec<#inner_type> }
        }
        QueryParamType::Enum(_) => quote! { String },
        QueryParamType::Json => quote! { serde_json::Value },
        _ => quote! { String },
    }
}

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
    let query_params = extract_query_params(&endpoint.params);
    // Only JSON requests have a typed body field
    let has_body = matches!(&endpoint.request, Some(ApiRequest::Json(_)));
    let method_str = endpoint.method.to_string();

    // Extract body type name for new() constructor
    let body_type_name = match &endpoint.request {
        Some(ApiRequest::Json(schema)) => Some(schema.type_name.as_str()),
        _ => None,
    };

    // Generate struct fields
    let path_param_fields = generate_path_param_fields(&path_params);
    let query_param_fields = generate_query_param_fields(&query_params);
    let body_field = generate_body_field(endpoint);

    // Generate derives (Default only if no body or body type implements Default)
    let derives = generate_derives(has_body);

    // Generate Default impl if we have a body (manual impl needed)
    let default_impl = generate_default_impl(&struct_name, &path_params, &query_params, has_body);

    // Generate new() constructor method body for type-safe construction
    let new_method = generate_new_method(&path_params, &query_params, has_body, body_type_name);

    // Generate builder methods for query params
    let query_builder_methods = generate_query_builder_methods(&query_params);

    // Generate From<Body> impl for body-only structs
    let from_body_impl = generate_from_body_impl(
        &struct_name,
        &path_params,
        &query_params,
        has_body,
        body_type_name,
    );

    // Generate From<&str> and From<String> impls for single-param no-body structs
    let from_string_impls =
        generate_from_string_impls(&struct_name, &path_params, &query_params, has_body);

    // Generate EndpointSpec implementation for type-safe hook registration
    let endpoint_spec_impl =
        generate_endpoint_spec_impl(&struct_name, &endpoint.id, &endpoint.response);

    // Generate into_parts method
    let into_parts = generate_into_parts(endpoint, &path_params, &query_params, &method_str);

    // Generate doc comments with example section
    let doc_lines = generate_doc_comment_with_example(
        &endpoint.id,
        &struct_name_str,
        &path_params,
        &query_params,
        has_body,
        body_type_name,
        module_path,
    );

    // Combine all fields
    let all_fields = if has_body {
        quote! {
            #path_param_fields
            #query_param_fields
            #body_field
        }
    } else {
        quote! {
            #path_param_fields
            #query_param_fields
        }
    };

    quote! {
        #(#[doc = #doc_lines])*
        #derives
        pub struct #struct_name {
            #all_fields
        }

        #default_impl

        impl #struct_name {
            #new_method
            #query_builder_methods
            #into_parts
        }

        #from_body_impl

        #from_string_impls

        #endpoint_spec_impl
    }
}

/// Generates doc comment lines with an example section.
///
/// The example shows how to instantiate the request struct:
/// - For structs with no required fields: uses `Default::default()`
/// - For structs with path params only: uses `new()` with params
/// - For structs with body: uses `new()` with body (and optional params)
/// - Query params use builder-style `with_*` methods
///
/// ## Arguments
///
/// * `endpoint_id` - The endpoint identifier (e.g., "CreateChatCompletion")
/// * `struct_name` - The full struct name (e.g., "CreateChatCompletionRequest")
/// * `path_params` - List of path parameter names
/// * `query_params` - List of query parameter info
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
    query_params: &[QueryParamInfo],
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
            lines.push(format!(" let request = {struct_name}::new(body)"));
        } else {
            // Include path params in new() call
            let param_args: Vec<String> = path_params
                .iter()
                .map(|p| format!("\"{p}_value\""))
                .collect();
            let args = param_args.join(", ");
            lines.push(format!(" let request = {struct_name}::new({args}, body)"));
        }

        // Add query param builder calls
        for qp in query_params {
            let setter_name = format_ident!("{}", qp.name);
            lines.push(format!("     .with_{}(/* value */)", setter_name));
        }
        lines.push(";".to_string());
    } else if !path_params.is_empty() {
        // Path params only - use new()
        lines.push(format!(" use schematic_schema::{mod_path}::{struct_name};"));
        lines.push(String::new());

        let param_args: Vec<String> = path_params
            .iter()
            .map(|p| format!("\"{p}_value\""))
            .collect();
        let args = param_args.join(", ");
        lines.push(format!(" let request = {struct_name}::new({args})"));

        // Add query param builder calls
        for qp in query_params {
            lines.push(format!("     .with_{}(/* value */)", qp.name));
        }
        lines.push(";".to_string());
    } else {
        // No required fields - use default()
        lines.push(format!(" use schematic_schema::{mod_path}::{struct_name};"));
        lines.push(String::new());
        lines.push(format!(" let request = {struct_name}::default()"));

        // Add query param builder calls
        for qp in query_params {
            lines.push(format!("     .with_{}(/* value */)", qp.name));
        }
        lines.push(";".to_string());
    }

    lines.push(" ```".to_string());

    lines
}

/// Generates field declarations for path parameters.
fn generate_path_param_fields(path_params: &[&str]) -> TokenStream {
    let fields = path_params.iter().map(|param| {
        let field_name = format_ident!("{}", param);
        let doc = format!(" Path parameter: {}", param);
        quote! {
            #[doc = #doc]
            pub #field_name: String,
        }
    });

    quote! { #(#fields)* }
}

/// Generates field declarations for query parameters.
///
/// Query parameters are optional (wrapped in `Option<T>`) since they typically
/// have default values on the server side.
fn generate_query_param_fields(query_params: &[QueryParamInfo]) -> TokenStream {
    let fields = query_params.iter().map(|qp| {
        let field_name = format_ident!("{}", qp.name);
        let rust_type = query_param_type_to_rust_type(&qp.param_type);
        let doc = qp
            .description
            .as_ref()
            .map(|d| {
                let doc_str = format!(" Query parameter: {}", d);
                quote! { #[doc = #doc_str] }
            })
            .unwrap_or_else(|| {
                let doc_str = format!(" Query parameter: {}", qp.name);
                quote! { #[doc = #doc_str] }
            });

        quote! {
            #doc
            pub #field_name: Option<#rust_type>,
        }
    });

    quote! { #(#fields)* }
}

/// Generates builder-style `with_*` methods for query parameters.
///
/// Each query parameter gets a `with_name(value)` method that returns `Self`
/// for method chaining.
fn generate_query_builder_methods(query_params: &[QueryParamInfo]) -> TokenStream {
    let methods = query_params.iter().map(|qp| {
        let method_name = format_ident!("with_{}", qp.name);
        let field_name = format_ident!("{}", qp.name);
        let rust_type = query_param_type_to_rust_type(&qp.param_type);
        let doc = format!(" Sets the `{}` query parameter.", qp.name);

        quote! {
            #[doc = #doc]
            pub fn #method_name(mut self, value: #rust_type) -> Self {
                self.#field_name = Some(value);
                self
            }
        }
    });

    quote! { #(#methods)* }
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
        // Handle future variants (non_exhaustive)
        Some(_) => quote! {},
        None => quote! {},
    }
}

/// Generates derive attributes for the struct.
fn generate_derives(_has_body: bool) -> TokenStream {
    // Always derive Default - all field types (String, body types) implement Default
    quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] }
}

/// Generates the Default implementation for structs with a body.
///
/// Returns empty TokenStream since Default is now always derived.
fn generate_default_impl(
    _struct_name: &proc_macro2::Ident,
    _path_params: &[&str],
    _query_params: &[QueryParamInfo],
    _has_body: bool,
) -> TokenStream {
    // Default is derived for all structs, no manual impl needed
    quote! {}
}

/// Generates a `From<BodyType>` impl for body-only request structs.
///
/// This allows ergonomic conversion from the body type to the request struct:
/// ```ignore
/// let req: CreateCompletionRequest = body.into();
/// ```
///
/// ## Returns
///
/// - For structs with body and no path params: `impl From<BodyType> for StructName`
/// - Otherwise: empty `TokenStream`
fn generate_from_body_impl(
    struct_name: &proc_macro2::Ident,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
    has_body: bool,
    body_type: Option<&str>,
) -> TokenStream {
    // Only generate From impl for body-only structs (no path params)
    if has_body && path_params.is_empty() {
        let body_ty = format_ident!("{}", body_type.unwrap());

        // Initialize query params to None
        let query_field_inits: Vec<_> = query_params
            .iter()
            .map(|qp| {
                let name = format_ident!("{}", qp.name);
                quote! { #name: None }
            })
            .collect();

        quote! {
            impl From<#body_ty> for #struct_name {
                fn from(body: #body_ty) -> Self {
                    Self {
                        #(#query_field_inits,)*
                        body,
                    }
                }
            }
        }
    } else {
        quote! {}
    }
}

/// Generates `From<&str>` and `From<String>` impls for single-param no-body request structs.
///
/// This allows ergonomic conversion from string types to the request struct:
/// ```ignore
/// let req: RetrieveModelRequest = "gpt-4".into();
/// let req = RetrieveModelRequest::from("gpt-4");
/// ```
///
/// ## Returns
///
/// - For structs with exactly one path param and no body/query params: `impl From<&str>` and `impl From<String>`
/// - Otherwise: empty `TokenStream`
fn generate_from_string_impls(
    struct_name: &proc_macro2::Ident,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
    has_body: bool,
) -> TokenStream {
    // Only generate From impls for single-param no-body no-query-param structs
    if path_params.len() == 1 && !has_body && query_params.is_empty() {
        let field_name = format_ident!("{}", path_params[0]);
        quote! {
            impl From<&str> for #struct_name {
                fn from(param: &str) -> Self {
                    Self { #field_name: param.to_string() }
                }
            }

            impl From<String> for #struct_name {
                fn from(param: String) -> Self {
                    Self { #field_name: param }
                }
            }
        }
    } else if path_params.len() == 1 && !has_body && !query_params.is_empty() {
        // Single path param with query params - still generate From but initialize query params
        let field_name = format_ident!("{}", path_params[0]);
        let query_field_inits: Vec<_> = query_params
            .iter()
            .map(|qp| {
                let name = format_ident!("{}", qp.name);
                quote! { #name: None }
            })
            .collect();

        quote! {
            impl From<&str> for #struct_name {
                fn from(param: &str) -> Self {
                    Self {
                        #field_name: param.to_string(),
                        #(#query_field_inits,)*
                    }
                }
            }

            impl From<String> for #struct_name {
                fn from(param: String) -> Self {
                    Self {
                        #field_name: param,
                        #(#query_field_inits,)*
                    }
                }
            }
        }
    } else {
        quote! {}
    }
}

/// Generates a `new()` constructor method body for the request struct.
///
/// The constructor requires all path parameters and the body (if present),
/// providing compile-time enforcement of required fields.
///
/// ## Returns
///
/// - For structs with path params or body: method body for `new()` (no impl wrapper)
/// - For empty structs (no params, no body): empty `TokenStream` (use `Default`)
fn generate_new_method(
    path_params: &[&str],
    query_params: &[QueryParamInfo],
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

    let path_field_inits: Vec<_> = path_params
        .iter()
        .map(|p| {
            let name = format_ident!("{}", p);
            quote! { #name: #name.into() }
        })
        .collect();

    // Initialize query params to None
    let query_field_inits: Vec<_> = query_params
        .iter()
        .map(|qp| {
            let name = format_ident!("{}", qp.name);
            quote! { #name: None }
        })
        .collect();

    if has_body {
        let body_ty = format_ident!("{}", body_type.unwrap());
        quote! {
            /// Creates a new request with the required path parameters and body.
            pub fn new(#(#params,)* body: #body_ty) -> Self {
                Self {
                    #(#path_field_inits,)*
                    #(#query_field_inits,)*
                    body,
                }
            }
        }
    } else if !path_params.is_empty() {
        quote! {
            /// Creates a new request with the required path parameters.
            pub fn new(#(#params),*) -> Self {
                Self {
                    #(#path_field_inits,)*
                    #(#query_field_inits,)*
                }
            }
        }
    } else if !query_params.is_empty() {
        // No path params but has query params - need to initialize them
        quote! {
            /// Creates a new request with default values.
            pub fn new() -> Self {
                Self {
                    #(#query_field_inits,)*
                }
            }
        }
    } else {
        // No params, no body - new() is just Default
        quote! {}
    }
}

/// Generates the into_parts method.
fn generate_into_parts(
    endpoint: &Endpoint,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
    method_str: &str,
) -> TokenStream {
    use schematic_define::ApiRequest;

    let path_format = generate_path_format(&endpoint.path, path_params, query_params);
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
        /// - Fully substituted path string with query parameters
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

/// Generates the EndpointSpec implementation for type-safe hook registration.
///
/// The trait associates request types with their response types and endpoint IDs,
/// enabling the variant builder's `mutate_response` method to verify types at compile time.
fn generate_endpoint_spec_impl(
    struct_name: &proc_macro2::Ident,
    endpoint_id: &str,
    response: &ApiResponse,
) -> TokenStream {
    // Determine the response type based on ApiResponse variant
    let response_type = match response {
        ApiResponse::Json(schema) => {
            // Parse the type name as a proper type (handles Vec<T>, Option<T>, etc.)
            let type_name = &schema.type_name;
            // Use syn to parse the type expression
            match syn::parse_str::<syn::Type>(type_name) {
                Ok(ty) => quote! { #ty },
                Err(_) => {
                    // Fallback: treat as simple identifier (shouldn't happen with valid schemas)
                    let ident = format_ident!("{}", type_name);
                    quote! { #ident }
                }
            }
        }
        ApiResponse::Text => quote! { String },
        ApiResponse::Binary => quote! { bytes::Bytes },
        ApiResponse::Empty => quote! { () },
        // Handle future variants (non_exhaustive)
        _ => quote! { () },
    };

    quote! {
        impl crate::shared::EndpointSpec for #struct_name {
            type Response = #response_type;
            const ENDPOINT_ID: &'static str = #endpoint_id;
        }
    }
}

/// Generates the path format expression with query parameter handling.
fn generate_path_format(
    path: &str,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
) -> TokenStream {
    let mut_tok = if !query_params.is_empty() {
        quote! { mut }
    } else {
        quote! {}
    };

    let base_path = if path_params.is_empty() {
        let path_literal = path;
        quote! { let #mut_tok path = #path_literal.to_string(); }
    } else {
        // Build format string and arguments
        let format_str = build_format_string(path, path_params);
        let format_args = path_params.iter().map(|param| {
            let field_name = format_ident!("{}", param);
            quote! { self.#field_name }
        });

        quote! { let #mut_tok path = format!(#format_str, #(#format_args),*); }
    };

    // Generate query parameter appending
    let query_appending = if query_params.is_empty() {
        quote! {}
    } else {
        let query_param_code = query_params.iter().map(|qp| {
            let field_name = format_ident!("{}", qp.name);
            let param_name = &qp.name;

            match &qp.param_type {
                QueryParamType::Array(_) => {
                    quote! {
                        if let Some(ref values) = self.#field_name {
                            for value in values {
                                if !path.contains('?') {
                                    path.push_str(&format!("?{}={}", #param_name, value));
                                } else {
                                    path.push_str(&format!("&{}={}", #param_name, value));
                                }
                            }
                        }
                    }
                }
                _ => {
                    quote! {
                        if let Some(ref value) = self.#field_name {
                            if !path.contains('?') {
                                path.push_str(&format!("?{}={}", #param_name, value));
                            } else {
                                path.push_str(&format!("&{}={}", #param_name, value));
                            }
                        }
                    }
                }
            }
        });

        quote! {
            #(#query_param_code)*
        }
    };

    quote! {
        #base_path
        #query_appending
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
            params: None,
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
        assert!(code.contains("#[derive(Debug, Clone, Default, Serialize, Deserialize)]"));
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
    fn single_impl_block_with_new_and_into_parts() {
        // Test request with path param (has new() and into_parts())
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

        // Both new() and into_parts() should be in same impl block
        assert!(
            code.contains("pub fn new("),
            "Expected new() method in impl block"
        );
        assert!(
            code.contains("pub fn into_parts("),
            "Expected into_parts() method in impl block"
        );
    }

    #[test]
    fn single_impl_block_with_body_and_both_methods() {
        // Test request with body (has new() and into_parts())
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
    fn single_impl_block_for_no_param_request() {
        // Test request with no params (only into_parts(), no new())
        let endpoint = make_endpoint("ListModels", RestMethod::Get, "/models", None);
        let tokens = generate_request_struct(&endpoint);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Count occurrences of "impl ListModelsRequest" (non-From impls)
        let impl_count = code.matches("impl ListModelsRequest").count();

        assert_eq!(
            impl_count, 1,
            "Expected exactly 1 impl block for ListModelsRequest, got {}\n\nCode:\n{}",
            impl_count, code
        );

        // Should only have into_parts(), not new()
        assert!(
            !code.contains("pub fn new("),
            "Expected no new() method for no-param request"
        );
        assert!(
            code.contains("pub fn into_parts("),
            "Expected into_parts() method"
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

        // Check that query param appending is generated
        // The format uses: path.push_str(&format!("?{}={}", "page", value))
        assert!(
            code.contains(r#"path.push_str(&format!("?{}={}", "page""#),
            "Expected page query param append, got:\n{}",
            code
        );
        assert!(code.contains(r#""limit""#), "Expected limit param name");
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
        // Array params are handled specially
        assert!(code.contains("for value in values"));
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
        assert!(
            code.contains("pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self")
        );
    }
}
