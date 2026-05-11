//! HTTP client method generation for REST APIs.
//!
//! Generates request methods that execute HTTP requests using reqwest,
//! with proper authentication handling based on the API's AuthStrategy.
//!
//! ## Response Types
//!
//! Different methods are generated based on endpoint response types:
//! - `request<T>()` - For JSON responses (deserializes to type T)
//! - `request_bytes()` - For binary responses (returns `bytes::Bytes`)
//! - `request_text()` - For text responses (returns `String`)
//! - `request_empty()` - For empty responses (returns `()`)

mod helpers;
mod methods;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

/// Generates all request methods for the API struct.
///
/// Analyzes which response types the API uses and generates the appropriate
/// request methods. Also generates convenience methods for non-JSON endpoints.
///
/// ## Generated Methods
///
/// - `request<T>()` - Generated if any endpoint returns JSON
/// - `request_bytes()` - Generated if any endpoint returns Binary
/// - `request_text()` - Generated if any endpoint returns Text
/// - `request_empty()` - Generated if any endpoint returns Empty
///
/// ## Examples
///
/// For an API with mixed response types:
/// ```text
/// impl ElevenLabs {
///     // JSON responses
///     pub async fn request<T: serde::de::DeserializeOwned>(...) -> Result<T, SchematicError>
///
///     // Binary responses (audio files)
///     pub async fn request_bytes(...) -> Result<bytes::Bytes, SchematicError>
///
///     // Convenience method for CreateSpeech endpoint
///     pub async fn create_speech(req: CreateSpeechRequest) -> Result<bytes::Bytes, SchematicError>
/// }
/// ```
pub fn generate_request_method(api: &RestApi) -> TokenStream {
    generate_request_method_with_suffix(api, "Request")
}

/// Generates all request methods for the API struct with a custom request suffix.
///
/// This is the same as `generate_request_method` but allows specifying a custom
/// suffix for request struct names (e.g., "BasicRequest" or "BearerRequest").
pub fn generate_request_method_with_suffix(api: &RestApi, request_suffix: &str) -> TokenStream {
    let struct_name = format_ident!("{}", api.name);
    let request_enum = format_ident!("{}Request", api.name);

    // Check which response types the API uses
    let has_json = api.endpoints.iter().any(|e| e.response.is_json());
    let has_binary = api.endpoints.iter().any(|e| e.response.is_binary());
    let has_text = api.endpoints.iter().any(|e| e.response.is_text());
    let has_empty = api.endpoints.iter().any(|e| e.response.is_empty());

    let auth_setup = helpers::generate_auth_setup(api);
    let auth_helpers = helpers::generate_auth_helper_methods();

    // Generate shared helper method
    let build_request_method =
        methods::generate_build_request_method(&struct_name, &request_enum, &auth_setup);

    // Generate merge_headers helper
    let merge_headers_method = methods::generate_merge_headers_method();

    // Generate response-specific methods
    let json_method = if has_json {
        methods::generate_json_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    let bytes_method = if has_binary {
        methods::generate_bytes_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    let text_method = if has_text {
        methods::generate_text_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    let empty_method = if has_empty {
        methods::generate_empty_request_method(&struct_name, &request_enum)
    } else {
        quote! {}
    };

    // Generate convenience methods for non-JSON endpoints
    let convenience_methods = helpers::generate_convenience_methods(api, request_suffix);

    quote! {
        impl #struct_name {
            #auth_helpers
            #build_request_method
            #merge_headers_method
            #json_method
            #bytes_method
            #text_method
            #empty_method
            #convenience_methods
        }
    }
}
