//! Header and cookie parameter fields, builders, and serialization.
//!
//! Both kinds reuse the query parameter type mapping and become `Option<T>`
//! fields with `with_*` builders; `required` is not enforced. They are sent
//! in the `headers` field of `RequestParts`, never in the query:
//!
//! - a set header parameter replaces any endpoint header with the same name
//!   (case-insensitive); an unset one leaves it alone. Array elements are
//!   joined with `,`.
//! - cookie parameters are folded, with the endpoint's static `Cookie`
//!   header, into one `Cookie` header by the generated
//!   `combine_cookie_headers`. Values go through `encode_cookie_value`;
//!   array elements are encoded one by one and joined with `,`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::QueryParamType;

use crate::codegen::request_structs::shared::{
    QueryParamInfo, query_param_type_to_rust_type,
};

/// Where a named parameter travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParamLocation {
    Header,
    Cookie,
}

impl ParamLocation {
    fn label(self) -> &'static str {
        match self {
            Self::Header => "Header",
            Self::Cookie => "Cookie",
        }
    }
}

/// Generates `Option<T>` field declarations for header or cookie parameters.
pub(super) fn generate_named_param_fields(
    params: &[QueryParamInfo],
    location: ParamLocation,
) -> TokenStream {
    let fields = params.iter().map(|param| {
        let field_name = format_ident!("{}", param.field_name());
        let rename = param.serde_rename();
        let rust_type = query_param_type_to_rust_type(&param.param_type);
        let doc = format!(
            " {} parameter `{}`{}",
            location.label(),
            param.name,
            param
                .description
                .as_deref()
                .map(|description| format!(": {description}"))
                .unwrap_or_default()
        );

        let deprecated = param.deprecation_doc();
        let rules = param.rules_doc();

        quote! {
            #[doc = #doc]
            #deprecated
            #rules
            #rename
            pub #field_name: Option<#rust_type>,
        }
    });

    quote! { #(#fields)* }
}

/// Generates `with_*` builder methods for header or cookie parameters.
pub(super) fn generate_named_builder_methods(
    params: &[QueryParamInfo],
    location: ParamLocation,
) -> TokenStream {
    let methods = params.iter().map(|param| {
        let field = param.field_name();
        let method_name = format_ident!("with_{}", field);
        let field_name = format_ident!("{}", field);
        let rust_type = query_param_type_to_rust_type(&param.param_type);
        let doc = format!(
            " Sets the `{}` {} parameter.",
            param.name,
            location.label().to_lowercase()
        );
        let deprecated = param.deprecation_doc();

        quote! {
            #[doc = #doc]
            #deprecated
            pub fn #method_name(mut self, value: #rust_type) -> Self {
                self.#field_name = Some(value);
                self
            }
        }
    });

    quote! { #(#methods)* }
}

/// Generates the expression or statements that produce the `headers` field
/// of `into_parts`'s `RequestParts`.
///
/// With no header or cookie parameters, and at most one static `Cookie`
/// header, this is the plain list of the endpoint's static headers.
/// Otherwise it binds `headers`, merging every `Cookie` header and set cookie
/// parameter into one, and the returned expression is that binding.
pub(super) fn generate_headers_assembly(
    static_headers: &[(String, String)],
    header_params: &[QueryParamInfo],
    cookie_params: &[QueryParamInfo],
) -> (TokenStream, TokenStream) {
    let static_pairs = static_headers.iter().map(|(name, value)| {
        quote! { (#name.to_string(), #value.to_string()) }
    });
    let static_list = if static_headers.is_empty() {
        quote! { vec![] }
    } else {
        quote! { vec![#(#static_pairs),*] }
    };

    // An API-level `Cookie` is folded in ahead of the endpoint's own, so the
    // static list itself can hold two `Cookie` headers to merge.
    let static_cookies = static_headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        .count();
    if header_params.is_empty() && cookie_params.is_empty() && static_cookies < 2 {
        return (quote! {}, static_list);
    }

    let header_setters = header_params.iter().map(|param| {
        let field_name = format_ident!("{}", param.field_name());
        let wire_name = &param.name;
        let value = header_value_expr(&param.param_type);
        quote! {
            if let Some(ref value) = self.#field_name {
                let value = #value;
                headers.retain(|(name, _)| !name.eq_ignore_ascii_case(#wire_name));
                headers.push((#wire_name.to_string(), value));
            }
        }
    });

    let cookies = if cookie_params.is_empty() && static_cookies < 2 {
        quote! {}
    } else if cookie_params.is_empty() {
        quote! {
            crate::shared::combine_cookie_headers(&mut headers, &[])?;
        }
    } else {
        let cookie_pushes = cookie_params.iter().map(|param| {
            let field_name = format_ident!("{}", param.field_name());
            let wire_name = &param.name;
            let value = cookie_value_expr(&param.param_type);
            quote! {
                if let Some(ref value) = self.#field_name {
                    cookies.push((#wire_name.to_string(), #value));
                }
            }
        });
        quote! {
            let mut cookies: Vec<(String, String)> = Vec::new();
            #(#cookie_pushes)*
            crate::shared::combine_cookie_headers(&mut headers, &cookies)?;
        }
    };

    let statements = quote! {
        let mut headers: Vec<(String, String)> = #static_list;
        #(#header_setters)*
        #cookies
    };
    (statements, quote! { headers })
}

/// The header value for a bound `value`: scalars as written, arrays joined
/// with `,`.
fn header_value_expr(param_type: &QueryParamType) -> TokenStream {
    match param_type {
        QueryParamType::Array(_) => quote! {
            value.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(",")
        },
        _ => quote! { value.to_string() },
    }
}

/// The encoded cookie value for a bound `value`: each array element is
/// encoded on its own, so a `,` inside an element cannot read as a separator.
fn cookie_value_expr(param_type: &QueryParamType) -> TokenStream {
    match param_type {
        QueryParamType::Array(_) => quote! {
            value
                .iter()
                .map(|item| crate::shared::encode_cookie_value(&item.to_string()))
                .collect::<Vec<_>>()
                .join(",")
        },
        _ => quote! { crate::shared::encode_cookie_value(&value.to_string()) },
    }
}
