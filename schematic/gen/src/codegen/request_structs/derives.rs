//! Derive attribute generation for request structs.

use proc_macro2::TokenStream;
use quote::quote;

/// Generates derive attributes for the request struct.
///
/// Always derives `Debug, Clone, Serialize, Deserialize`, and `Default` when
/// `default` is set. A request with a typed path field or a required query
/// field passes `false`: a defaulted value would bypass `new()` and send a
/// placeholder the caller never chose.
pub(super) fn generate_derives(default: bool) -> TokenStream {
    if default {
        quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] }
    } else {
        quote! { #[derive(Debug, Clone, Serialize, Deserialize)] }
    }
}
