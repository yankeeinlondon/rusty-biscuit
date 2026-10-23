//! Shared helpers for the assemble submodules.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

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

/// The `schematic_definitions` module path of the registered REST module
/// `flat_module_path`; an unregistered module is its own path.
pub(crate) fn definitions_module_path(flat_module_path: &str) -> &str {
    schematic_definitions::registry::registrations()
        .iter()
        .filter(|entry| entry.rest().is_some())
        .find(|entry| entry.module == flat_module_path)
        .map_or(flat_module_path, |entry| entry.definitions_path)
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
