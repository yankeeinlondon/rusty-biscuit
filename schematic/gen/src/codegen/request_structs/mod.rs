//! Request struct generation for API endpoints.
//!
//! Generates Rust structs that encapsulate path parameters, query parameters,
//! and request bodies for each API endpoint, with methods to convert them into
//! HTTP request parts.

mod body;
mod derives;
mod endpoint_spec;
mod into_parts;
mod path_params;
mod query_params;
mod shared;
mod single;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::Endpoint;

use crate::parser::extract_path_params;

use shared::{QueryParamInfo, extract_query_params, param_field_name};

// Re-export for sibling codegen modules and tests
pub use shared::{format_generated_code, validate_generated_code};

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
/// ```text
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
    let path_param_fields = path_params::generate_path_param_fields(&path_params);
    let query_param_fields = query_params::generate_query_param_fields(&query_params);
    let body_field = body::generate_body_field(endpoint);

    // Generate derives (Default only if no body or body type implements Default)
    let derives = derives::generate_derives(has_body);

    // Generate Default impl if we have a body (manual impl needed)
    let default_impl =
        body::generate_default_impl(&struct_name, &path_params, &query_params, has_body);

    // Generate new() constructor method body for type-safe construction
    let new_method =
        path_params::generate_new_method(&path_params, &query_params, has_body, body_type_name);

    // Generate builder methods for query params
    let query_builder_methods = query_params::generate_query_builder_methods(&query_params);

    // Generate From<Body> impl for body-only structs
    let from_body_impl = body::generate_from_body_impl(
        &struct_name,
        &path_params,
        &query_params,
        has_body,
        body_type_name,
    );

    // Generate From<&str> and From<String> impls for single-param no-body structs
    let from_string_impls =
        single::generate_from_string_impls(&struct_name, &path_params, &query_params, has_body);

    // Generate EndpointSpec implementation for type-safe hook registration
    let endpoint_spec_impl =
        endpoint_spec::generate_endpoint_spec_impl(&struct_name, &endpoint.id, &endpoint.response);

    // Generate into_parts method
    let into_parts =
        into_parts::generate_into_parts(endpoint, &path_params, &query_params, &method_str);

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
    lines.push(" ```text".to_string());

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
            let snake_name = param_field_name(&qp.name);
            lines.push(format!("     .with_{}(/* value */)", snake_name));
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
            lines.push(format!(
                "     .with_{}(/* value */)",
                param_field_name(&qp.name)
            ));
        }
        lines.push(";".to_string());
    } else {
        // No required fields - use default()
        lines.push(format!(" use schematic_schema::{mod_path}::{struct_name};"));
        lines.push(String::new());
        lines.push(format!(" let request = {struct_name}::default()"));

        // Add query param builder calls
        for qp in query_params {
            lines.push(format!(
                "     .with_{}(/* value */)",
                param_field_name(&qp.name)
            ));
        }
        lines.push(";".to_string());
    }

    lines.push(" ```".to_string());

    lines
}
