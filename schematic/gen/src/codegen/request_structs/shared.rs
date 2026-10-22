//! Shared helpers for request struct generation.
//!
//! Common utilities used across all request struct shape generators.

#![allow(dead_code)]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::openapi::import::naming::sanitize_rust_field_ident;
use schematic_define::{EndpointParams, ParamDef, QueryParamType, RestApi, TypeRef};

/// The Rust spelling of a JSON body reference, resolved through the API's
/// own [`RestApi::schemas`].
///
/// A reference that does not resolve is spelled as a `compile_error!` type
/// naming it, so generated code that bypassed validation fails to compile at
/// that type instead of silently naming some other type.
pub fn json_type_name(api: &RestApi, reference: &TypeRef) -> String {
    api.rust_type(reference).unwrap_or_else(|| {
        let message = format!("unresolved schema reference {reference:?} in API {}", api.name);
        format!("::core::compile_error!({message:?})")
    })
}

/// Renders a Rust type spelling as tokens.
///
/// Spellings are not always bare identifiers: component paths are
/// qualified (`crate::types::Model`), free-form JSON is
/// `::serde_json::Value`, and wrappers are generic (`::std::vec::Vec<..>`).
/// All are valid input to [`syn::parse_str`] but panic `format_ident!`, so
/// every site that turns a type spelling into tokens must go through here.
pub fn type_name_to_tokens(type_name: &str) -> TokenStream {
    match syn::parse_str::<syn::Type>(type_name) {
        Ok(ty) => quote! { #ty },
        // Unparseable names are still emitted as an identifier so the generated
        // file fails to compile at the offending type rather than aborting the
        // whole run with a `quote` panic.
        Err(_) => {
            let ident = format_ident!("{}", sanitize_rust_field_ident(type_name));
            quote! { #ident }
        }
    }
}

/// Returns the Rust struct-field identifier for a parameter's wire name.
///
/// Wire names taken from a spec (`{user-id}`, `$top`, `user.name`, `2fa`) are
/// not always valid Rust identifiers; routing them through
/// [`sanitize_rust_field_ident`] guarantees a usable field name and keeps the
/// path-building, field-declaration, and constructor sites in agreement.
pub fn param_field_name(wire_name: &str) -> String {
    sanitize_rust_field_ident(wire_name)
}

/// Emits `#[serde(rename = "<wire>")]` when the sanitized field name differs
/// from the parameter's wire name, so the on-the-wire spelling is preserved.
pub fn param_serde_rename(wire_name: &str) -> TokenStream {
    let field = param_field_name(wire_name);
    if field == wire_name {
        quote! {}
    } else {
        quote! { #[serde(rename = #wire_name)] }
    }
}

/// Converts a PascalCase or camelCase string to snake_case.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Information about a query parameter for code generation.
///
/// Header and cookie parameters use it too: they share the query type
/// mapping and the `Option<T>` field and `with_*` builder shape.
#[derive(Debug, Clone)]
pub struct QueryParamInfo {
    pub name: String,
    /// Explicit Rust spelling; the query/header/cookie key stays `name`.
    pub rust_name: Option<String>,
    pub description: Option<String>,
    pub param_type: QueryParamType,
    /// Documentation only: the field and setter docs say "Deprecated.".
    pub deprecated: bool,
}

impl QueryParamInfo {
    pub fn field_name(&self) -> String {
        self.rust_name.clone().unwrap_or_else(|| param_field_name(&self.name))
    }

    /// The doc attributes a deprecated parameter's field and setter append
    /// after their summary; none otherwise.
    pub fn deprecation_doc(&self) -> TokenStream {
        if self.deprecated {
            let line = format!(" {}", super::endpoint_docs::DEPRECATED_LINE);
            quote! { #[doc = ""] #[doc = #line] }
        } else {
            quote! {}
        }
    }

    pub fn serde_rename(&self) -> TokenStream {
        let wire = &self.name;
        if self.field_name() == *wire { quote! {} } else { quote! { #[serde(rename = #wire)] } }
    }
}

/// Extracts query parameters from an endpoint's params field.
pub fn extract_query_params(params: &Option<EndpointParams>) -> Vec<QueryParamInfo> {
    params.as_ref().map(|p| param_infos(&p.query)).unwrap_or_default()
}

/// Extracts header parameters from an endpoint's params field.
pub fn extract_header_params(params: &Option<EndpointParams>) -> Vec<QueryParamInfo> {
    params.as_ref().map(|p| param_infos(&p.header)).unwrap_or_default()
}

/// Extracts cookie parameters from an endpoint's params field.
pub fn extract_cookie_params(params: &Option<EndpointParams>) -> Vec<QueryParamInfo> {
    params.as_ref().map(|p| param_infos(&p.cookie)).unwrap_or_default()
}

fn param_infos(params: &[ParamDef]) -> Vec<QueryParamInfo> {
    params
        .iter()
        .map(|p| QueryParamInfo {
            name: p.name.clone(),
            rust_name: p.rust_name.clone(),
            description: p.description.clone(),
            param_type: p.param_type.clone(),
            deprecated: p.deprecated,
        })
        .collect()
}

/// Maps QueryParamType to a Rust type token stream.
pub fn query_param_type_to_rust_type(param_type: &QueryParamType) -> TokenStream {
    match param_type {
        QueryParamType::String => quote! { String },
        QueryParamType::Integer => quote! { i64 },
        QueryParamType::Number => quote! { f64 },
        QueryParamType::Boolean => quote! { bool },
        QueryParamType::Array(inner) => {
            let inner_type = query_param_type_to_rust_type(inner);
            quote! { Vec<#inner_type> }
        }
        QueryParamType::Enum(_) => quote! { String },
        QueryParamType::Json => quote! { serde_json::Value },
        _ => quote! { String },
    }
}

/// Generates the statements that bind `path` and `query` inside `into_parts`.
///
/// `path` is the endpoint path with path parameters substituted (and
/// percent-encoded unless written `{+name}`); any query written into the path
/// template stays in it untouched. `query` holds the declared query parameters
/// as unencoded `(name, value)` pairs in declaration order, one pair per array
/// element. The runtime client percent-encodes them once, when it builds the
/// URL.
pub fn generate_path_format(
    path: &str,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
) -> TokenStream {
    let base_path = if path_params.is_empty() {
        let path_literal = path;
        quote! { let path = #path_literal.to_string(); }
    } else {
        // Build format string and arguments. The format string keeps each
        // placeholder's position by wire name; the interpolated value is the
        // sanitized field, percent-encoded so a value containing `/ ? # %`
        // cannot break out of its path segment or forge the request.
        //
        // A param written `{+name}` in the source path opts out of encoding
        // (RFC 6570 reserved expansion): its value is emitted raw, so a
        // slash-bearing repository id or file path keeps its separators.
        let format_str = build_format_string(path, path_params);
        let format_args = path_params.iter().map(|param| {
            let field_name = format_ident!("{}", param_field_name(param));
            if path.contains(&format!("{{+{}}}", param)) {
                // No `&`: `format!` takes its arguments by reference already, and
                // a redundant one trips `clippy::needless_borrows_for_generic_args`.
                quote! { self.#field_name }
            } else {
                quote! { urlencoding::encode(&self.#field_name.to_string()) }
            }
        });

        quote! { let path = format!(#format_str, #(#format_args),*); }
    };

    let query = generate_query_binding(query_params);

    quote! {
        #base_path
        #query
    }
}

/// Generates the statement that binds `query` inside `into_parts`: the
/// declared query parameters as unencoded `(name, value)` pairs in
/// declaration order, one pair per array element.
pub fn generate_query_binding(query_params: &[QueryParamInfo]) -> TokenStream {
    if query_params.is_empty() {
        quote! { let query: Vec<(String, String)> = Vec::new(); }
    } else {
        let query_param_collectors = query_params.iter().map(|qp| {
            let field_name = format_ident!("{}", qp.field_name());
            let param_name = &qp.name;

            match &qp.param_type {
                QueryParamType::Array(_) => quote! {
                    if let Some(ref values) = self.#field_name {
                        for value in values {
                            query.push((#param_name.to_string(), value.to_string()));
                        }
                    }
                },
                _ => quote! {
                    if let Some(ref value) = self.#field_name {
                        query.push((#param_name.to_string(), value.to_string()));
                    }
                },
            }
        });

        quote! {
            let mut query: Vec<(String, String)> = Vec::new();
            #(#query_param_collectors)*
        }
    }
}

/// Builds a format string by replacing `{param}` and `{+param}` with `{}`.
pub fn build_format_string(path: &str, path_params: &[&str]) -> String {
    let mut result = path.to_string();
    for param in path_params {
        result = result.replace(&format!("{{+{}}}", param), "{}");
        result = result.replace(&format!("{{{}}}", param), "{}");
    }
    result
}

/// Validates that the generated code is syntactically correct.
///
/// ## Errors
///
/// Returns an error string if the generated code fails to parse.
pub fn validate_generated_code(tokens: &TokenStream) -> Result<(), String> {
    syn::parse2::<syn::File>(tokens.clone()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Formats generated code using prettyplease.
///
/// ## Errors
///
/// Returns an error string if the code fails to parse.
pub fn format_generated_code(tokens: &TokenStream) -> Result<String, String> {
    let file = syn::parse2::<syn::File>(tokens.clone()).map_err(|e| e.to_string())?;
    Ok(prettyplease::unparse(&file))
}
