//! Body-specific request struct generation.
//!
//! Handles typed body fields and `From<BodyType>` implementations for
//! request structs that carry a JSON payload.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{ApiRequest, Endpoint};

use crate::codegen::request_structs::shared::{QueryParamInfo, param_field_name};

/// Generates the body field if the endpoint has a request schema.
pub fn generate_body_field(endpoint: &Endpoint) -> TokenStream {
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

/// Generates the Default implementation for structs with a body.
///
/// Returns empty TokenStream since Default is now always derived.
pub fn generate_default_impl(
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
/// ```text
/// let req: CreateCompletionRequest = body.into();
/// ```
///
/// ## Returns
///
/// - For structs with body and no path params: `impl From<BodyType> for StructName`
/// - Otherwise: empty `TokenStream`
pub fn generate_from_body_impl(
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
                let name = format_ident!("{}", param_field_name(&qp.name));
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
