//! EndpointSpec trait implementation generation.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::ApiResponse;

use crate::codegen::request_structs::shared::type_name_to_tokens;

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
        ApiResponse::Json(schema) => type_name_to_tokens(&schema.type_name),
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
