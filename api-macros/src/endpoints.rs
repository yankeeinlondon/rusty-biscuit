//! Implementation of the `#[endpoints]` attribute macro.
//!
//! This module handles parsing impl blocks with endpoint method definitions
//! and generates the actual HTTP client method implementations.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Error, ImplItem, ItemImpl, Result};

use crate::parse::{EndpointConfig, EndpointsInput};

/// Main implementation for the `#[endpoints]` attribute macro.
pub fn endpoints_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    match endpoints_inner(attr, item) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn endpoints_inner(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    // Parse the attribute arguments
    let input: EndpointsInput = parse2(attr)?;
    let _api_type = &input.api_type;

    // Parse the impl block
    let impl_block: ItemImpl = parse2(item.clone())?;

    // For now, just validate endpoints and return the original impl block
    // Full implementation will be added in Phase 5
    for item in &impl_block.items {
        if let ImplItem::Fn(method) = item {
            if let Some(_config) = EndpointConfig::from_attrs(&method.attrs)? {
                // Endpoint found and validated
                // Full code generation will be implemented in Phase 5
            }
        }
    }

    // Return the original impl block unchanged for now
    // Phase 5 will transform this into actual endpoint implementations
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_endpoints_parses_basic_impl() {
        let attr = quote! { api = TestApi };
        let item = quote! {
            impl TestApi {
                #[endpoint(method = Get, path = "/test")]
                #[response(json)]
                pub async fn test_endpoint(&self) -> Result<String, ApiError>;
            }
        };

        let result = endpoints_impl(attr, item);
        // Should not produce a compile error
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_endpoints_requires_api_attribute() {
        let attr = quote! {};
        let item = quote! {
            impl TestApi {}
        };

        let result = endpoints_impl(attr, item);
        // Should produce a compile error about missing api attribute
        assert!(result.to_string().contains("compile_error"));
    }
}
