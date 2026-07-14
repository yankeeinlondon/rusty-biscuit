//! Single-parameter request struct ergonomics.
//!
//! Generates `From<&str>` and `From<String>` impls for request structs
//! with exactly one path parameter, enabling ergonomic usage like:
//! ```text
//! let req: RetrieveModelRequest = "gpt-4".into();
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::codegen::request_structs::shared::{QueryParamInfo, param_field_name};

/// Generates `From<&str>` and `From<String>` impls for single-param no-body request structs.
///
/// This allows ergonomic conversion from string types to the request struct:
/// ```text
/// let req: RetrieveModelRequest = "gpt-4".into();
/// let req = RetrieveModelRequest::from("gpt-4");
/// ```
///
/// ## Returns
///
/// - For structs with exactly one path param and no body/query params: `impl From<&str>` and `impl From<String>`
/// - Otherwise: empty `TokenStream`
pub fn generate_from_string_impls(
    struct_name: &proc_macro2::Ident,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
    has_body: bool,
) -> TokenStream {
    // Only generate From impls for single-param no-body no-query-param structs
    if path_params.len() == 1 && !has_body && query_params.is_empty() {
        let field_name = format_ident!("{}", param_field_name(path_params[0]));
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
        let field_name = format_ident!("{}", param_field_name(path_params[0]));
        let query_field_inits: Vec<_> = query_params
            .iter()
            .map(|qp| {
                let name = format_ident!("{}", param_field_name(&qp.name));
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
