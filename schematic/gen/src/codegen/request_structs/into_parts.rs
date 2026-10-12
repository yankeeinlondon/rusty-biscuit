//! The body of a request type's `EndpointSpec::into_parts`.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{ApiRequest, Endpoint};

use crate::codegen::request_structs::named_params::generate_headers_assembly;
use crate::codegen::request_structs::shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, generate_path_format,
};

/// Generates the statements that assemble `schematic_runtime::RequestParts`.
///
/// The parts carry no credential; the runtime attaches one per hop. A JSON
/// body that fails to serialize, and a static `Cookie` header that cannot be
/// merged with cookie parameters, are `InvalidArgument`s reported before any
/// network work.
pub(super) fn generate_into_parts_body(
    endpoint: &Endpoint,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
) -> TokenStream {
    let path_format = generate_path_format(&endpoint.path, path_params, query_params);

    let body_expr = match &endpoint.request {
        Some(ApiRequest::Json(_)) => quote! {
            schematic_runtime::RequestBody::Json(
                serde_json::to_string(&self.body).map_err(|_| {
                    schematic_runtime::InvalidArgument::new("the request body cannot be serialized as JSON")
                })?
            )
        },
        Some(ApiRequest::FormData { .. }) => quote! {
            schematic_runtime::RequestBody::Multipart(self.body.into_form_parts())
        },
        Some(ApiRequest::UrlEncoded { .. }) => quote! {
            schematic_runtime::RequestBody::UrlEncoded(self.body.into_form_pairs())
        },
        Some(ApiRequest::Text { content_type }) => quote! {
            schematic_runtime::RequestBody::Raw {
                content_type: #content_type.to_string(),
                bytes: self.body.into_bytes().into(),
            }
        },
        Some(ApiRequest::Binary { content_type }) => quote! {
            schematic_runtime::RequestBody::Raw {
                content_type: #content_type.to_string(),
                bytes: self.body.into(),
            }
        },
        _ => quote! { schematic_runtime::RequestBody::Empty },
    };

    let (headers_statements, headers_init) = generate_headers_assembly(
        &endpoint.headers,
        &extract_header_params(&endpoint.params),
        &extract_cookie_params(&endpoint.params),
    );
    // The assembly binds `headers` itself when parameters add to it;
    // otherwise its expression is the static list, bound here.
    let headers_binding = if headers_statements.is_empty() {
        quote! { let headers: Vec<(String, String)> = #headers_init; }
    } else {
        headers_statements
    };

    quote! {
        #path_format
        #headers_binding
        Ok(schematic_runtime::RequestParts {
            method: <Self as schematic_runtime::EndpointSpec>::METHOD,
            path,
            query,
            headers,
            body: #body_expr,
        })
    }
}
