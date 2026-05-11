//! Derive attribute generation for request structs.

use proc_macro2::TokenStream;
use quote::quote;

/// Generates derive attributes for the request struct.
///
/// Always derives `Debug, Clone, Default, Serialize, Deserialize`.
pub(super) fn generate_derives(_has_body: bool) -> TokenStream {
    // Always derive Default - all field types (String, body types) implement Default
    quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] }
}
