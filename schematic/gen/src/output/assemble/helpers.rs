//! Shared helpers for the assemble submodules.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

/// Converts a PascalCase string to snake_case.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Returns the module path for the given API.
///
/// Uses `api.module_path` if set, otherwise attempts to infer from the API name.
/// Falls back to `api.name.to_lowercase()` if inference returns None.
///
/// ## Resolution Order
///
/// 1. Explicit `module_path` (highest priority)
/// 2. Inferred from CamelCase name (e.g., "OllamaNative" -> "ollama")
/// 3. Lowercase API name (fallback)
pub fn get_module_path(api: &RestApi) -> String {
    api.module_path.clone().unwrap_or_else(|| {
        crate::inference::infer_module_path(&api.name).unwrap_or_else(|| api.name.to_lowercase())
    })
}

/// Returns the request suffix for the given API.
///
/// Uses `api.request_suffix` if set, otherwise defaults to `"Request"`.
pub fn get_request_suffix(api: &RestApi) -> String {
    api.request_suffix
        .clone()
        .unwrap_or_else(|| "Request".to_string())
}

pub(crate) fn definitions_module_path(flat_module_path: &str) -> &str {
    match flat_module_path {
        "unfolded_circle_core_rest" => "unfolded_circle::core_rest",
        _ => flat_module_path,
    }
}

pub(crate) fn build_definitions_import(module_path: &str) -> TokenStream {
    let segments: Vec<_> = definitions_module_path(module_path)
        .split("::")
        .map(|s| format_ident!("{}", s))
        .collect();
    quote! {
        pub use schematic_definitions::#(#segments)::*::*;
    }
}
