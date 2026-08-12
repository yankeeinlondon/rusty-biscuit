//! Shared helpers for request struct generation.
//!
//! Common utilities used across all request struct shape generators.

#![allow(dead_code)]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::openapi::import::naming::sanitize_rust_field_ident;
use schematic_define::{EndpointParams, QueryParamType};

/// Renders a schema type name as Rust type tokens.
///
/// Names arriving from an imported spec are not always bare identifiers: a
/// schema with no `$ref` maps to the path `serde_json::Value`, and generic
/// spellings such as `Vec<Model>` also occur. Both are valid input to
/// [`syn::parse_str`] but panic `format_ident!`, so every site that turns a
/// `Schema::type_name` into tokens must go through here.
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
#[derive(Debug, Clone)]
pub struct QueryParamInfo {
    pub name: String,
    pub description: Option<String>,
    pub param_type: QueryParamType,
}

/// Extracts query parameters from an endpoint's params field.
pub fn extract_query_params(params: &Option<EndpointParams>) -> Vec<QueryParamInfo> {
    match params {
        Some(EndpointParams { query, .. }) => query
            .iter()
            .map(|p| QueryParamInfo {
                name: p.name.clone(),
                description: p.description.clone(),
                param_type: p.param_type.clone(),
            })
            .collect(),
        None => Vec::new(),
    }
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

/// Generates the path format expression with query parameter handling.
///
/// Uses a collector pattern for query parameters:
/// 1. Collects all params into a `Vec<(&str, String)>`
/// 2. URL-encodes values using `urlencoding::encode()`
/// 3. Constructs query string with single `?`/`&` check
pub fn generate_path_format(
    path: &str,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
) -> TokenStream {
    let mut_tok = if !query_params.is_empty() {
        quote! { mut }
    } else {
        quote! {}
    };

    let base_path = if path_params.is_empty() {
        let path_literal = path;
        quote! { let #mut_tok path = #path_literal.to_string(); }
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

        quote! { let #mut_tok path = format!(#format_str, #(#format_args),*); }
    };

    // Generate query parameter collection and appending
    let query_appending = if query_params.is_empty() {
        quote! {}
    } else {
        // Generate collection statements for each param
        let query_param_collectors = query_params.iter().map(|qp| {
            let field_name = format_ident!("{}", param_field_name(&qp.name));
            let param_name = &qp.name;

            match &qp.param_type {
                QueryParamType::Array(_) => {
                    // Arrays add multiple entries for the same key
                    quote! {
                        if let Some(ref values) = self.#field_name {
                            for value in values {
                                query_pairs.push((#param_name, value.to_string()));
                            }
                        }
                    }
                }
                _ => {
                    quote! {
                        if let Some(ref value) = self.#field_name {
                            query_pairs.push((#param_name, value.to_string()));
                        }
                    }
                }
            }
        });

        quote! {
            let mut query_pairs: Vec<(&str, String)> = Vec::new();
            #(#query_param_collectors)*
            if !query_pairs.is_empty() {
                let query_string: String = query_pairs
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                if path.contains('?') {
                    path.push_str(&format!("&{}", query_string));
                } else {
                    path.push_str(&format!("?{}", query_string));
                }
            }
        }
    };

    quote! {
        #base_path
        #query_appending
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
