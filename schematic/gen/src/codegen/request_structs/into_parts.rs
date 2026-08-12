//! `into_parts` method and header initialization generation.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{ApiRequest, Endpoint};

use crate::codegen::request_structs::shared::{QueryParamInfo, generate_path_format};

/// Generates the into_parts method.
pub(super) fn generate_into_parts(
    endpoint: &Endpoint,
    path_params: &[&str],
    query_params: &[QueryParamInfo],
    method_str: &str,
) -> TokenStream {
    let path_format = generate_path_format(&endpoint.path, path_params, query_params);

    let body_expr = match &endpoint.request {
        Some(ApiRequest::Json(_)) => quote! {
            crate::shared::RequestBody::Json(
                serde_json::to_string(&self.body).map_err(|e| {
                    SchematicError::SerializationError(e.to_string())
                })?
            )
        },
        Some(ApiRequest::FormData { .. }) => quote! {
            crate::shared::RequestBody::Multipart(self.body.into_form_parts())
        },
        Some(ApiRequest::UrlEncoded { .. }) => quote! {
            crate::shared::RequestBody::UrlEncoded(self.body.into_form_pairs())
        },
        _ => quote! { crate::shared::RequestBody::Empty },
    };

    // Generate headers initialization
    let headers_init = generate_endpoint_headers_init(&endpoint.headers);

    quote! {
        /// Converts the request into (method, path, body, headers) parts.
        ///
        /// ## Returns
        ///
        /// A tuple of:
        /// - HTTP method as a static string (e.g., "GET", "POST")
        /// - Fully substituted path string with query parameters
        /// - The request body as a `RequestBody`
        /// - Endpoint-specific headers as key-value pairs
        ///
        /// ## Errors
        ///
        /// Returns `SchematicError::SerializationError` if a JSON request body
        /// fails to serialize.
        pub fn into_parts(self) -> Result<RequestParts, SchematicError> {
            #path_format
            Ok((#method_str, path, #body_expr, #headers_init))
        }
    }
}

/// Generates the initialization code for endpoint-specific headers.
fn generate_endpoint_headers_init(headers: &[(String, String)]) -> TokenStream {
    if headers.is_empty() {
        quote! { vec![] }
    } else {
        let header_pairs = headers.iter().map(|(k, v)| {
            quote! { (#k.to_string(), #v.to_string()) }
        });
        quote! { vec![#(#header_pairs),*] }
    }
}
