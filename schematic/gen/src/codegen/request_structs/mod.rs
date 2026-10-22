//! Request struct generation for API endpoints.
//!
//! Generates Rust structs that encapsulate path parameters, query parameters,
//! and request bodies for each API endpoint, with methods to convert them into
//! HTTP request parts.

mod body;
mod derives;
pub(crate) mod endpoint_docs;
pub(crate) mod endpoint_spec;
mod form;
mod into_parts;
mod named_params;
pub(crate) mod param_enums;
pub(crate) mod path_params;
mod query_params;
pub(crate) mod shared;
mod single;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{Endpoint, RestApi};

use crate::codegen::pagination::OrdinaryDefault;
use crate::parser::extract_path_params;

use named_params::ParamLocation;
use path_params::PathField;
use shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, extract_query_params,
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
/// - one field per path parameter: a `String` per placeholder when the
///   endpoint declares no `params.path` (e.g., `{model}` becomes
///   `pub model: String`), otherwise one field per distinct placeholder typed
///   `String`, `i64`, `f64`, `bool`, or a generated enum, with `new()` taking
///   them in first-occurrence template order
/// - A `body` field if the endpoint sends a body: its JSON type, a generated
///   form struct, `String` for a text body, or `Vec<u8>` for a binary body
/// - a derived `Default`, unless a typed path field makes `new()` the only
///   way to build it
/// - docs stating deprecation, an endpoint's own authentication, and its
///   effective rate-limit headers and retry guidance; none of these changes
///   the generated code
/// - a `schematic_runtime::EndpointSpec` implementation naming its API
///   marker, success type, codec, and endpoint id, whose `into_parts`
///   assembles the request
///
/// This form names the API marker [`PLACEHOLDER_API_MARKER`]; generated
/// modules use [`generate_request_struct_with_options`] with the API's own
/// name.
///
/// JSON body and response types are spelled from `endpoint`'s references,
/// resolved through `api`'s [`RestApi::schemas`].
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
pub fn generate_request_struct(api: &RestApi, endpoint: &Endpoint) -> TokenStream {
    generate_request_struct_with_suffix(api, endpoint, DEFAULT_REQUEST_SUFFIX)
}

/// Generates a request struct for the given endpoint with a custom suffix.
///
/// This variant allows specifying a custom suffix instead of the default "Request".
/// For example, using "Params" would generate `ListModelsParams` instead of `ListModelsRequest`.
/// Like [`generate_request_struct`], it names the API marker
/// [`PLACEHOLDER_API_MARKER`].
pub fn generate_request_struct_with_suffix(api: &RestApi, endpoint: &Endpoint, suffix: &str) -> TokenStream {
    generate_request_struct_with_options(api, endpoint, suffix, None, PLACEHOLDER_API_MARKER)
}

/// Generates a request struct for the given endpoint with full configuration.
///
/// `suffix` is appended to the endpoint id to name the struct, `module_path`
/// (for example `openai`) appears in the doc example, and `api_marker` is the
/// API type the request's `EndpointSpec::Api` names, which scopes the request
/// to that API's client.
pub fn generate_request_struct_with_options(
    api: &RestApi,
    endpoint: &Endpoint,
    suffix: &str,
    module_path: Option<&str>,
    api_marker: &str,
) -> TokenStream {
    generate_request_struct_with_defaults(api, endpoint, suffix, module_path, api_marker, &[])
}

/// [`generate_request_struct_with_options`] for an endpoint seen through its
/// pager view, whose `into_parts` also sends `defaults`, the required
/// parameters its pagination strategy adopted.
pub(crate) fn generate_request_struct_with_defaults(
    api: &RestApi,
    endpoint: &Endpoint,
    suffix: &str,
    module_path: Option<&str>,
    api_marker: &str,
    defaults: &[OrdinaryDefault],
) -> TokenStream {
    let struct_name = format_ident!("{}{}", endpoint.id, suffix);
    let struct_name_str = format!("{}{}", endpoint.id, suffix);
    let path_params = extract_path_params(&endpoint.path);
    let query_params = extract_query_params(&endpoint.params);
    let header_params = extract_header_params(&endpoint.params);
    let cookie_params = extract_cookie_params(&endpoint.params);
    let typed_paths = path_params::typed_path_fields(endpoint, &struct_name_str);
    let typed_paths = typed_paths.as_deref();
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
    let body_type_name = request_body_type_name(api, endpoint);
    let body_type_name = body_type_name.as_deref();
    let has_body = body_type_name.is_some();

    let path_param_fields = match typed_paths {
        Some(fields) => path_params::generate_typed_path_fields(fields),
        None => path_params::generate_path_param_fields(&path_params),
    };
    let path_enums = typed_paths
        .map(|fields| path_params::generate_path_enums(fields, &struct_name_str))
        .unwrap_or_default();
    let query_param_fields = query_params::generate_query_param_fields(&query_params);
    let header_param_fields =
        named_params::generate_named_param_fields(&header_params, ParamLocation::Header);
    let cookie_param_fields =
        named_params::generate_named_param_fields(&cookie_params, ParamLocation::Cookie);
    let body_field = body::generate_body_field(api, endpoint);

    let derives = derives::generate_derives(typed_paths.is_none());

    let default_impl =
        body::generate_default_impl(&struct_name, &path_params, &query_params, has_body);

    // Generate new() constructor method body for type-safe construction
    let path_args = match typed_paths {
        Some(fields) => path_params::typed_path_args(fields),
        None => path_params::untyped_path_args(&path_params),
    };
    let new_method =
        path_params::generate_new_method(&path_args, &optional_params, has_body, body_type_name);

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

    // Generate From<&str> and From<String> impls for single-param no-body
    // structs. A typed path value is only taken by `new()`.
    let from_string_impls = if typed_paths.is_some() {
        quote! {}
    } else {
        single::generate_from_string_impls(&struct_name, &path_params, &optional_params, has_body)
    };

    let into_parts_body = into_parts::generate_into_parts_body(
        endpoint,
        &path_params,
        typed_paths,
        &query_params,
        defaults,
    );
    let endpoint_spec_impl =
        endpoint_spec::generate_endpoint_spec_impl(api, &struct_name, api_marker, endpoint, into_parts_body);

    let example = match typed_paths {
        Some(fields) => typed_path_example(fields),
        None => PathExample {
            args: path_params.iter().map(|p| format!("\"{p}_value\"")).collect(),
            imports: Vec::new(),
        },
    };
    let mut doc_lines = vec![format!(" Request for `{}` endpoint.", endpoint.id)];
    doc_lines.extend(
        endpoint_docs::request_doc_sections(api, endpoint)
            .iter()
            .map(|line| endpoint_docs::doc_line(line)),
    );
    doc_lines.extend(generate_doc_example(
        &struct_name_str,
        &example,
        &optional_params,
        has_body,
        body_type_name,
        module_path,
    ));

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

    let form_struct = form::generate_form_struct(api, endpoint);

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

        #path_enums

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
pub(crate) fn request_body_type_name(api: &RestApi, endpoint: &Endpoint) -> Option<String> {
    use schematic_define::ApiRequest;

    match &endpoint.request {
        Some(ApiRequest::Json(reference)) => Some(shared::json_type_name(api, reference)),
        Some(ApiRequest::Text { .. }) => Some("String".to_string()),
        Some(ApiRequest::Binary { .. }) => Some("Vec<u8>".to_string()),
        _ => form::form_body_type_name(endpoint),
    }
}

/// The path arguments a doc example passes to `new()`, and the types they
/// name that the example's `use` line imports besides the request.
struct PathExample {
    args: Vec<String>,
    imports: Vec<String>,
}

fn typed_path_example(fields: &[PathField]) -> PathExample {
    let imports = fields
        .iter()
        .filter_map(|field| match &field.value {
            path_params::PathValue::Enum { type_name, .. } => Some(type_name.clone()),
            _ => None,
        })
        .collect();
    PathExample { args: fields.iter().map(PathField::example_arg).collect(), imports }
}

/// Generates the doc lines of the example section, starting with the blank
/// line that separates it from the summary.
///
/// The example shows how to instantiate the request struct:
/// - For structs with no required fields: uses `Default::default()`
/// - For structs with path params only: uses `new()` with params
/// - For structs with body: uses `new()` with body (and optional params)
/// - Query params use builder-style `with_*` methods
fn generate_doc_example(
    struct_name: &str,
    path: &PathExample,
    query_params: &[QueryParamInfo],
    has_body: bool,
    body_type: Option<&str>,
    module_path: Option<&str>,
) -> Vec<String> {
    let path_params = &path.args;
    let mut lines = vec![
        String::new(),
        " ## Example".to_string(),
        String::new(),
        " ```text".to_string(),
    ];

    // Build the module path for use statement
    let mod_path = module_path.unwrap_or("api");
    let request_import = if path.imports.is_empty() {
        format!(" use schematic_schema::{mod_path}::{struct_name};")
    } else {
        format!(" use schematic_schema::{mod_path}::{{{struct_name}, {}}};", path.imports.join(", "))
    };

    // Generate appropriate example based on struct configuration
    if has_body {
        let body_ty = body_type.unwrap_or("Body");

        // Raw text and binary bodies are std types: nothing to import.
        match body_ty {
            "String" | "Vec<u8>" => {
                lines.push(request_import.clone());
                lines.push(String::new());
                lines.push(format!(" let body: {body_ty} = Default::default();"));
            }
            _ if body_ty.starts_with("::") => {
                lines.push(request_import.clone());
                lines.push(String::new());
                lines.push(format!(" let body: {body_ty} = Default::default();"));
            }
            _ => {
                let mut imports = vec![struct_name.to_string()];
                imports.extend(path.imports.iter().cloned());
                imports.push(body_ty.to_string());
                lines.push(format!(" use schematic_schema::{mod_path}::{{{}}};", imports.join(", ")));
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
            let args = path_params.join(", ");
            lines.push(format!(" let request = {struct_name}::new({args}, body)"));
        }

        // Add query param builder calls
        for qp in query_params {
            let snake_name = qp.field_name();
            lines.push(format!("     .with_{}(/* value */)", snake_name));
        }
        lines.push(";".to_string());
    } else if !path_params.is_empty() {
        // Path params only - use new()
        lines.push(request_import);
        lines.push(String::new());

        let args = path_params.join(", ");
        lines.push(format!(" let request = {struct_name}::new({args})"));

        // Add query param builder calls
        for qp in query_params {
            lines.push(format!(
                "     .with_{}(/* value */)",
                qp.field_name()
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
                qp.field_name()
            ));
        }
        lines.push(";".to_string());
    }

    lines.push(" ```".to_string());

    lines
}
