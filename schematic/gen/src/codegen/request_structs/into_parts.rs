//! The body of a request type's `EndpointSpec::into_parts`.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::pagination::ParamLocation;
use schematic_define::{ApiRequest, Endpoint};

use crate::codegen::pagination::OrdinaryDefault;

use crate::codegen::request_structs::named_params::generate_headers_assembly;
use crate::codegen::request_structs::path_params::{PathField, generate_typed_path_binding};
use crate::codegen::request_structs::shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, generate_path_format,
    generate_query_binding,
};

/// Generates the statements that assemble `schematic_runtime::RequestParts`.
///
/// The parts carry no credential; the runtime attaches one per hop. A JSON
/// body that fails to serialize, and a static `Cookie` header that cannot be
/// merged with cookie parameters, are `InvalidArgument`s reported before any
/// network work.
///
/// `typed_paths` are the endpoint's declared path fields; `None` substitutes
/// the untyped `path_params` instead. A non-finite typed number is an
/// `InvalidArgument` too.
///
/// `defaults` are adopted parameters the endpoint declares required: they
/// are not request fields, so their pager default is appended here.
pub(super) fn generate_into_parts_body(
    endpoint: &Endpoint,
    path_params: &[&str],
    typed_paths: Option<&[PathField]>,
    query_params: &[QueryParamInfo],
    defaults: &[OrdinaryDefault],
) -> TokenStream {
    let path_format = match typed_paths {
        Some(fields) => {
            let path = generate_typed_path_binding(&endpoint.path, fields);
            let query = generate_query_binding(query_params);
            quote! { #path #query }
        }
        None => generate_path_format(&endpoint.path, path_params, query_params),
    };

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

    let defaults_statements = generate_defaults(defaults);

    quote! {
        #path_format
        #headers_binding
        #defaults_statements
        Ok(schematic_runtime::RequestParts {
            method: <Self as schematic_runtime::EndpointSpec>::METHOD,
            path,
            query,
            headers,
            body: #body_expr,
        })
    }
}

/// Appends each required adopted parameter's default to the assembled query
/// or headers.
fn generate_defaults(defaults: &[OrdinaryDefault]) -> TokenStream {
    let pushes = |location: ParamLocation, value_type: TokenStream| {
        defaults
            .iter()
            .filter(move |default| default.location == location)
            .map(|OrdinaryDefault { name, value, .. }| quote! { (#name.to_owned(), #value_type::from(#value)) })
            .collect::<Vec<_>>()
    };
    let query = pushes(ParamLocation::Query, quote! { schematic_runtime::QueryValue });
    let headers = pushes(ParamLocation::Header, quote! { String });
    let mut statements = TokenStream::new();
    if !query.is_empty() {
        statements.extend(quote! { let mut query = query; query.extend([#(#query),*]); });
    }
    if !headers.is_empty() {
        statements.extend(quote! { let mut headers = headers; headers.extend([#(#headers),*]); });
    }
    statements
}
