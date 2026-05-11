//! Shared helpers for request struct generation.
//!
//! Common utilities used across all request struct shape generators.

#![allow(dead_code)]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{EndpointParams, QueryParamType};

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
        // Build format string and arguments
        let format_str = build_format_string(path, path_params);
        let format_args = path_params.iter().map(|param| {
            let field_name = format_ident!("{}", param);
            quote! { self.#field_name }
        });

        quote! { let #mut_tok path = format!(#format_str, #(#format_args),*); }
    };

    // Generate query parameter collection and appending
    let query_appending = if query_params.is_empty() {
        quote! {}
    } else {
        // Generate collection statements for each param
        let query_param_collectors = query_params.iter().map(|qp| {
            let field_name = format_ident!("{}", to_snake_case(&qp.name));
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
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
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

/// Builds a format string by replacing {param} with {}.
pub fn build_format_string(path: &str, path_params: &[&str]) -> String {
    let mut result = path.to_string();
    for param in path_params {
        let placeholder = format!("{{{}}}", param);
        result = result.replace(&placeholder, "{}");
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
