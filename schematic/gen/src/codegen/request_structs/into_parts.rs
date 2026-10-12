//! `into_parts` method and header initialization generation.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{ApiRequest, Endpoint};

use crate::codegen::request_structs::named_params::generate_headers_assembly;
use crate::codegen::request_structs::shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, generate_path_format,
};

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
        Some(ApiRequest::Text { content_type }) => quote! {
            crate::shared::RequestBody::Raw {
                content_type: #content_type.to_string(),
                bytes: self.body.into_bytes(),
            }
        },
        Some(ApiRequest::Binary { content_type }) => quote! {
            crate::shared::RequestBody::Raw {
                content_type: #content_type.to_string(),
                bytes: self.body,
            }
        },
        _ => quote! { crate::shared::RequestBody::Empty },
    };

    let cookie_params = extract_cookie_params(&endpoint.params);
    let (headers_statements, headers_init) = generate_headers_assembly(
        &endpoint.headers,
        &extract_header_params(&endpoint.params),
        &cookie_params,
    );
    // Header and cookie parameters travel in the headers element; only cookie
    // parameters add a failure, so the doc line is emitted only for them.
    let cookie_error_doc = if cookie_params.is_empty() {
        quote! {}
    } else {
        quote! {
            ///
            /// Returns `SchematicError::Header` if the endpoint's static
            /// `Cookie` header, which set cookie parameters join, cannot be
            /// parsed.
        }
    };

    quote! {
        /// Converts the request into (method, path, body, headers, query) parts.
        ///
        /// ## Returns
        ///
        /// A tuple of:
        /// - HTTP method as a static string (e.g., "GET", "POST")
        /// - Path with path parameters substituted; declared query parameters
        ///   are not part of it
        /// - The request body as a `RequestBody`
        /// - Endpoint-specific headers as key-value pairs
        /// - Declared query parameters as unencoded key-value pairs
        ///
        /// ## Errors
        ///
        /// Returns `SchematicError::SerializationError` if a JSON request body
        /// fails to serialize.
        #cookie_error_doc
        pub fn into_parts(self) -> Result<RequestParts, SchematicError> {
            #path_format
            #headers_statements
            Ok((#method_str, path, #body_expr, #headers_init, query))
        }
    }
}
