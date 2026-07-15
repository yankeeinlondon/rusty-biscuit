//! Path parameter field generation and constructor methods.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::codegen::request_structs::shared::{
    QueryParamInfo, param_field_name, param_serde_rename,
};

/// Generates field declarations for path parameters.
pub(super) fn generate_path_param_fields(path_params: &[&str]) -> TokenStream {
    let fields = path_params.iter().map(|param| {
        let field_name = format_ident!("{}", param_field_name(param));
        let rename = param_serde_rename(param);
        let doc = format!(" Path parameter: {}", param);
        quote! {
            #[doc = #doc]
            #rename
            pub #field_name: String,
        }
    });

    quote! { #(#fields)* }
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
pub(super) fn generate_new_method(
    path_params: &[&str],
    query_params: &[QueryParamInfo],
    has_body: bool,
    body_type: Option<&str>,
) -> TokenStream {
    let params: Vec<_> = path_params
        .iter()
        .map(|p| {
            let name = format_ident!("{}", param_field_name(p));
            quote! { #name: impl Into<String> }
        })
        .collect();

    let path_field_inits: Vec<_> = path_params
        .iter()
        .map(|p| {
            let name = format_ident!("{}", param_field_name(p));
            quote! { #name: #name.into() }
        })
        .collect();

    // Initialize query params to None
    let query_field_inits: Vec<_> = query_params
        .iter()
        .map(|qp| {
            let name = format_ident!("{}", param_field_name(&qp.name));
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
