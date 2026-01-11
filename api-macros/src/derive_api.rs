//! Implementation of the `#[derive(RestApi)]` macro.
//!
//! This module handles parsing the input struct and its attributes,
//! then generates the API client infrastructure.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Data, DeriveInput, Error, Fields, Result};

use crate::codegen::generate_api_client;
use crate::parse::ApiConfig;

/// Main implementation for the `#[derive(RestApi)]` macro.
pub fn derive_rest_api_impl(input: TokenStream) -> TokenStream {
    match derive_rest_api_inner(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn derive_rest_api_inner(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;
    let config = ApiConfig::from_attrs(&input.attrs)?;

    // Validate configuration
    config.validate()?;

    let name = &input.ident;
    let vis = &input.vis;

    // Ensure the input is a unit struct
    match &input.data {
        Data::Struct(data_struct) => {
            if !matches!(data_struct.fields, Fields::Unit) {
                return Err(Error::new_spanned(
                    &data_struct.fields,
                    "RestApi can only be derived on unit structs (e.g., `struct ApiName;`)",
                ));
            }
        }
        Data::Enum(_) => {
            return Err(Error::new_spanned(
                input.ident,
                "RestApi cannot be derived on enums",
            ));
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input.ident,
                "RestApi cannot be derived on unions",
            ));
        }
    }

    // Generate a NEW struct definition with the client field
    // This will "shadow" the unit struct definition
    let struct_def = quote! {
        #vis struct #name {
            client: api::ApiClient,
        }
    };

    // Generate the client implementation
    let client_impl = generate_api_client(name, &config);

    // Combine both
    let expanded = quote! {
        #struct_def
        #client_impl
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_derive_parses_basic_struct() {
        let input = quote! {
            #[api(base_url = "https://api.example.com")]
            pub struct TestApi;
        };

        let result = derive_rest_api_impl(input);
        // Should not produce a compile error
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_derive_requires_base_url() {
        let input = quote! {
            pub struct TestApi;
        };

        let result = derive_rest_api_impl(input);
        // Should produce a compile error about missing base_url
        assert!(result.to_string().contains("compile_error"));
    }
}
