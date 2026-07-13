//! Query parameter field generation and builder methods.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::codegen::request_structs::shared::{
    QueryParamInfo, param_field_name, param_serde_rename, query_param_type_to_rust_type,
};

/// Generates field declarations for query parameters.
///
/// Query parameters are optional (wrapped in `Option<T>`) since they typically
/// have default values on the server side.
pub(super) fn generate_query_param_fields(query_params: &[QueryParamInfo]) -> TokenStream {
    let fields = query_params.iter().map(|qp| {
        let field_name = format_ident!("{}", param_field_name(&qp.name));
        let rename = param_serde_rename(&qp.name);
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
            #rename
            pub #field_name: Option<#rust_type>,
        }
    });

    quote! { #(#fields)* }
}

/// Generates builder-style `with_*` methods for query parameters.
///
/// Each query parameter gets a `with_name(value)` method that returns `Self`
/// for method chaining.
pub(super) fn generate_query_builder_methods(query_params: &[QueryParamInfo]) -> TokenStream {
    let methods = query_params.iter().map(|qp| {
        let field = param_field_name(&qp.name);
        let method_name = format_ident!("with_{}", field);
        let field_name = format_ident!("{}", field);
        let rust_type = query_param_type_to_rust_type(&qp.param_type);
        let doc = format!(" Sets the `{}` query parameter.", field);

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
