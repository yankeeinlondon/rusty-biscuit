use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

/// Converts a CamelCase identifier to snake_case.
pub(crate) fn to_snake_case(s: &str) -> String {
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

/// Generates the expression that resolves a request's credentials.
///
/// It evaluates to the API-level headers and any API-key parameter, through
/// the shared `resolve_request_auth` (see `shared_auth.rs`).
pub(crate) fn generate_auth_setup(_api: &RestApi) -> TokenStream {
    quote! {
        crate::shared::resolve_request_auth(
            &self.auth_policy,
            &self.headers,
            &self.env_auth,
            self.api_key_param.as_ref(),
        )?
    }
}

/// Generates convenience methods for non-JSON endpoints.
///
/// For each Binary, Text, or Empty endpoint, generates a named method
/// that provides compile-time type safety and better ergonomics.
///
/// ## Examples
///
/// For a Binary endpoint with id "CreateSpeech":
/// ```text
/// pub async fn create_speech(&self, req: CreateSpeechRequest) -> Result<bytes::Bytes, SchematicError> {
///     self.request_bytes(req).await
/// }
/// ```
pub fn generate_convenience_methods(api: &RestApi, request_suffix: &str) -> TokenStream {
    let methods: Vec<TokenStream> = api
        .endpoints
        .iter()
        .filter(|ep| !ep.response.is_json())
        .map(|ep| {
            let method_name = format_ident!("{}", to_snake_case(&ep.id));
            let request_struct = format_ident!("{}{}", ep.id, request_suffix);
            let doc = format!(" Convenience method for the `{}` endpoint.", ep.id);
            let desc_doc = format!(" {}", ep.description);

            if ep.response.is_binary() {
                quote! {
                    #[doc = #doc]
                    ///
                    #[doc = #desc_doc]
                    #[must_use = "this returns a Future that must be awaited"]
                    pub async fn #method_name(
                        &self,
                        request: #request_struct,
                    ) -> Result<bytes::Bytes, SchematicError> {
                        self.request_bytes(request).await
                    }
                }
            } else if ep.response.is_text() {
                quote! {
                    #[doc = #doc]
                    ///
                    #[doc = #desc_doc]
                    #[must_use = "this returns a Future that must be awaited"]
                    pub async fn #method_name(
                        &self,
                        request: #request_struct,
                    ) -> Result<String, SchematicError> {
                        self.request_text(request).await
                    }
                }
            } else if ep.response.is_empty() {
                quote! {
                    #[doc = #doc]
                    ///
                    #[doc = #desc_doc]
                    #[must_use = "this returns a Future that must be awaited"]
                    pub async fn #method_name(
                        &self,
                        request: #request_struct,
                    ) -> Result<(), SchematicError> {
                        self.request_empty(request).await
                    }
                }
            } else {
                quote! {}
            }
        })
        .collect();

    quote! { #(#methods)* }
}
