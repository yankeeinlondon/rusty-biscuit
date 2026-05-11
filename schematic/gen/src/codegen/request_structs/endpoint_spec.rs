//! EndpointSpec trait implementation generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::ApiResponse;

/// Generates the EndpointSpec implementation for type-safe hook registration.
///
/// The trait associates request types with their response types and endpoint IDs,
/// enabling the variant builder's `mutate_response` method to verify types at compile time.
pub(super) fn generate_endpoint_spec_impl(
    struct_name: &proc_macro2::Ident,
    endpoint_id: &str,
    response: &ApiResponse,
) -> TokenStream {
    // Determine the response type based on ApiResponse variant
    let response_type = match response {
        ApiResponse::Json(schema) => {
            // Parse the type name as a proper type (handles Vec<T>, Option<T>, etc.)
            let type_name = &schema.type_name;
            // Use syn to parse the type expression
            match syn::parse_str::<syn::Type>(type_name) {
                Ok(ty) => quote! { #ty },
                Err(_) => {
                    // Fallback: treat as simple identifier (shouldn't happen with valid schemas)
                    let ident = format_ident!("{}", type_name);
                    quote! { #ident }
                }
            }
        }
        ApiResponse::Text => quote! { String },
        ApiResponse::Binary => quote! { bytes::Bytes },
        ApiResponse::Empty => quote! { () },
        // Handle future variants (non_exhaustive)
        _ => quote! { () },
    };

    quote! {
        impl crate::shared::EndpointSpec for #struct_name {
            type Response = #response_type;
            const ENDPOINT_ID: &'static str = #endpoint_id;
        }
    }
}
