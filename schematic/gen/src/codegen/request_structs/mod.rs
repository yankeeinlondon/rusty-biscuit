//! Request struct generation for API endpoints.
//!
//! Generates Rust structs that encapsulate path parameters, query parameters,
//! and request bodies for each API endpoint, with methods to convert them into
//! HTTP request parts.

mod body;
mod derives;
pub(crate) mod endpoint_spec;
mod form;
mod into_parts;
mod named_params;
mod path_params;
mod query_params;
pub(crate) mod shared;
mod single;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::Endpoint;

use crate::parser::extract_path_params;

use named_params::ParamLocation;
use shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, extract_query_params,
    param_field_name,
};

// Re-export for sibling codegen modules and tests
pub use shared::{format_generated_code, validate_generated_code};

/// Default suffix for request struct names.
const DEFAULT_REQUEST_SUFFIX: &str = "Request";

/// The API marker named by request structs generated without an API, for
/// inspecting one endpoint's output in isolation.
pub const PLACEHOLDER_API_MARKER: &str = "Api";

/// Generates a request struct for the given endpoint.
///
/// The generated struct includes:
/// - String fields for each path parameter (e.g., `{model}` becomes `pub model: String`)
/// - A `body` field if the endpoint sends a body: its JSON type, a generated
///   form struct, `String` for a text body, or `Vec<u8>` for a binary body
/// - `Default` trait implementation
/// - a `schematic_runtime::EndpointSpec` implementation naming its API
///   marker, success type, codec, and endpoint id, whose `into_parts`
///   assembles the request
///
/// This form names the API marker [`PLACEHOLDER_API_MARKER`]; generated
/// modules use [`generate_request_struct_with_options`] with the API's own
/// name.
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
/// #[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// pub struct RetrieveModelRequest {
///     pub model: String,
/// }
///
/// impl schematic_runtime::EndpointSpec for RetrieveModelRequest {
///     type Api = Api;
///     type Success = Model;
///     type Failure = schematic_runtime::NoDeclaredError;
///     type Decoder = schematic_runtime::Json<Model>;
///     const ENDPOINT_ID: &'static str = "RetrieveModel";
///     const METHOD: schematic_runtime::Method = schematic_runtime::Method::GET;
///
///     fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument> {
///         let path = format!("/models/{}", urlencoding::encode(&self.model.to_string()));
///         let query: Vec<(String, String)> = Vec::new();
///         Ok(schematic_runtime::RequestParts { method: <Self as schematic_runtime::EndpointSpec>::METHOD, path, query, headers: vec![], body: schematic_runtime::RequestBody::Empty })
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
/// Like [`generate_request_struct`], it names the API marker
/// [`PLACEHOLDER_API_MARKER`].
pub fn generate_request_struct_with_suffix(endpoint: &Endpoint, suffix: &str) -> TokenStream {
    generate_request_struct_with_options(endpoint, suffix, None, PLACEHOLDER_API_MARKER)
}

/// Generates a request struct for the given endpoint with full configuration.
///
/// `suffix` is appended to the endpoint id to name the struct, `module_path`
/// (for example `openai`) appears in the doc example, and `api_marker` is the
/// API type the request's `EndpointSpec::Api` names, which scopes the request
/// to that API's client.
pub fn generate_request_struct_with_options(
    endpoint: &Endpoint,
    suffix: &str,
    module_path: Option<&str>,
    api_marker: &str,
) -> TokenStream {
    let struct_name = format_ident!("{}{}", endpoint.id, suffix);
    let struct_name_str = format!("{}{}", endpoint.id, suffix);
    let path_params = extract_path_params(&endpoint.path);
    let query_params = extract_query_params(&endpoint.params);
    let header_params = extract_header_params(&endpoint.params);
    let cookie_params = extract_cookie_params(&endpoint.params);
    // Every optional field starts as `None` in the generated constructors and
    // `From` impls, whichever location it is sent in.
    let optional_params: Vec<QueryParamInfo> = query_params
        .iter()
        .chain(&header_params)
        .chain(&cookie_params)
        .cloned()
        .collect();

    // Form endpoints get a synthesized `{EndpointId}Form` body type, so from here
    // down they follow exactly the same path as JSON bodies.
    let body_type_name = request_body_type_name(endpoint);
    let body_type_name = body_type_name.as_deref();
    let has_body = body_type_name.is_some();

    let path_param_fields = path_params::generate_path_param_fields(&path_params);
    let query_param_fields = query_params::generate_query_param_fields(&query_params);
    let header_param_fields =
        named_params::generate_named_param_fields(&header_params, ParamLocation::Header);
    let cookie_param_fields =
        named_params::generate_named_param_fields(&cookie_params, ParamLocation::Cookie);
    let body_field = body::generate_body_field(endpoint);

    let derives = derives::generate_derives(has_body);

    let default_impl =
        body::generate_default_impl(&struct_name, &path_params, &query_params, has_body);

    // Generate new() constructor method body for type-safe construction
    let new_method =
        path_params::generate_new_method(&path_params, &optional_params, has_body, body_type_name);

    // Generate builder methods for query, header, and cookie params
    let query_builder_methods = query_params::generate_query_builder_methods(&query_params);
    let header_builder_methods =
        named_params::generate_named_builder_methods(&header_params, ParamLocation::Header);
    let cookie_builder_methods =
        named_params::generate_named_builder_methods(&cookie_params, ParamLocation::Cookie);

    // Generate From<Body> impl for body-only structs
    let from_body_impl = body::generate_from_body_impl(
        &struct_name,
        &path_params,
        &optional_params,
        has_body,
        body_type_name,
    );

    // Generate From<&str> and From<String> impls for single-param no-body structs
    let from_string_impls =
        single::generate_from_string_impls(&struct_name, &path_params, &optional_params, has_body);

    let into_parts_body = into_parts::generate_into_parts_body(endpoint, &path_params, &query_params);
    let endpoint_spec_impl =
        endpoint_spec::generate_endpoint_spec_impl(&struct_name, api_marker, endpoint, into_parts_body);

    let doc_lines = generate_doc_comment_with_example(
        &endpoint.id,
        &struct_name_str,
        &path_params,
        &optional_params,
        has_body,
        body_type_name,
        module_path,
    );

    let all_fields = if has_body {
        quote! {
            #path_param_fields
            #query_param_fields
            #header_param_fields
            #cookie_param_fields
            #body_field
        }
    } else {
        quote! {
            #path_param_fields
            #query_param_fields
            #header_param_fields
            #cookie_param_fields
        }
    };

    let form_struct = form::generate_form_struct(endpoint);

    let inherent_methods = quote! {
        #new_method
        #query_builder_methods
        #header_builder_methods
        #cookie_builder_methods
    };
    let inherent_impl = if inherent_methods.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #struct_name {
                #inherent_methods
            }
        }
    };

    quote! {
        #form_struct

        #(#[doc = #doc_lines])*
        #derives
        pub struct #struct_name {
            #all_fields
        }

        #default_impl

        #inherent_impl

        #from_body_impl

        #from_string_impls

        #endpoint_spec_impl
    }
}

/// The type of the request's `body` field, or `None` when it sends no body:
/// the JSON type, a generated `{EndpointId}Form`, `String` for a text body,
/// or `Vec<u8>` for a binary body.
pub(crate) fn request_body_type_name(endpoint: &Endpoint) -> Option<String> {
    use schematic_define::ApiRequest;

    match &endpoint.request {
        Some(ApiRequest::Json(schema)) => Some(schema.type_name.clone()),
        Some(ApiRequest::Text { .. }) => Some("String".to_string()),
        Some(ApiRequest::Binary { .. }) => Some("Vec<u8>".to_string()),
        _ => form::form_body_type_name(endpoint),
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

        // Raw text and binary bodies are std types: nothing to import.
        match body_ty {
            "String" | "Vec<u8>" => {
                lines.push(format!(" use schematic_schema::{mod_path}::{struct_name};"));
                lines.push(String::new());
                lines.push(format!(" let body: {body_ty} = Default::default();"));
            }
            _ => {
                lines.push(format!(
                    " use schematic_schema::{mod_path}::{{{struct_name}, {body_ty}}};"
                ));
                lines.push(String::new());

                // Show body construction with ..Default::default() pattern
                lines.push(format!(" let body = {body_ty} {{"));
                lines.push("     // ... set required fields ...".to_string());
                lines.push("     ..Default::default()".to_string());
                lines.push(" };".to_string());
            }
        }

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
