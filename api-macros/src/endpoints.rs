//! Implementation of the `#[endpoints]` attribute macro.
//!
//! This module handles parsing impl blocks with endpoint method definitions
//! and generates the actual HTTP client method implementations.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Error, FnArg, ImplItem, ImplItemFn, ItemImpl, Pat, PatType, Result, ReturnType, Type, TypePath,
    parse2,
};

use crate::codegen::{http_method_ident, response_format_type, to_screaming_snake_case};
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
    let mut impl_block: ItemImpl = parse2(item)?;

    // Transform endpoint methods
    let mut transformed_items = Vec::new();

    for item in &impl_block.items {
        match item {
            ImplItem::Fn(method) => {
                if let Some(config) = EndpointConfig::from_attrs(&method.attrs)? {
                    // Transform this method into an endpoint implementation
                    let transformed = transform_endpoint_method(method, &config)?;
                    transformed_items.push(ImplItem::Fn(transformed));
                } else {
                    // Not an endpoint - keep as-is
                    transformed_items.push(item.clone());
                }
            }
            _ => {
                // Keep non-function items as-is
                transformed_items.push(item.clone());
            }
        }
    }

    // Replace the items in the impl block
    impl_block.items = transformed_items;

    Ok(quote! { #impl_block })
}

/// Transforms an endpoint method signature into a full implementation.
///
/// Takes a method like:
/// ```ignore
/// #[endpoint(method = Get, path = "/users/{id}")]
/// #[response(json)]
/// pub async fn get_user(&self, id: String) -> Result<User, ApiError>;
/// ```
///
/// And generates:
/// ```ignore
/// pub async fn get_user(&self, id: String) -> Result<User, api::ApiError> {
///     static ENDPOINT_GET_USER: api::Endpoint<api::response::JsonFormat<User>> =
///         api::Endpoint::builder()
///             .id("get_user")
///             .method(api::RestMethod::Get)
///             .path("/users/{id}")
///             .build();
///
///     self.client.execute_with_params(&ENDPOINT_GET_USER, &[("id", &id)]).await
/// }
/// ```
fn transform_endpoint_method(method: &ImplItemFn, config: &EndpointConfig) -> Result<ImplItemFn> {
    let method_name = &method.sig.ident;
    let _vis = &method.vis;
    let asyncness = &method.sig.asyncness;
    let inputs = &method.sig.inputs;
    let output = &method.sig.output;

    // Ensure the method is async
    if asyncness.is_none() {
        return Err(Error::new_spanned(
            &method.sig,
            "endpoint methods must be async",
        ));
    }

    // Extract the response type from Result<T, ApiError>
    let response_type = extract_response_type(output)?;

    // Generate static endpoint constant name
    let endpoint_const_name = format_ident!(
        "ENDPOINT_{}",
        to_screaming_snake_case(&method_name.to_string())
    );

    // Generate the endpoint builder
    let http_method = http_method_ident(config.method);
    let path = &config.path;
    let format_type = response_format_type(config.response_format, &response_type);

    let endpoint_def = quote! {
        static #endpoint_const_name: api::Endpoint<#format_type> = {
            match api::Endpoint::builder()
                .id(stringify!(#method_name))
                .method(api::RestMethod::#http_method)
                .path(#path)
                .build()
            {
                endpoint => endpoint,
            }
        };
    };

    // Extract path parameters from the config
    let path_params = config.path_params();

    // Extract method parameters (skip &self)
    let _method_params: Vec<&PatType> = inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                // Skip self parameter
                if let Pat::Ident(ident) = &*pat_type.pat
                    && ident.ident != "self"
                {
                    return Some(pat_type);
                }
            }
            None
        })
        .collect();

    // Generate the client call
    let client_call = if path_params.is_empty() {
        // No path parameters - use simple execute
        quote! {
            self.client.execute(&#endpoint_const_name).await
        }
    } else {
        // Has path parameters - build params array
        let param_pairs: Vec<TokenStream> = path_params
            .iter()
            .map(|param_name| {
                // Find the matching method parameter
                let param_ident = format_ident!("{}", param_name);

                // Convert to &str for the tuple
                quote! { (#param_name, #param_ident.as_ref()) }
            })
            .collect();

        quote! {
            self.client.execute_with_params(&#endpoint_const_name, &[#(#param_pairs),*]).await
        }
    };

    // Build the complete method body
    let body = quote! {
        {
            #endpoint_def
            #client_call
        }
    };

    // Create the transformed method
    let mut transformed = method.clone();

    // Remove the endpoint and response attributes (keep doc comments)
    transformed.attrs.retain(|attr| {
        !attr.path().is_ident("endpoint")
            && !attr.path().is_ident("response")
            && !attr.path().is_ident("request")
    });

    // Replace the body
    transformed.block = syn::parse2(body)?;

    // Ensure return type uses api::ApiError
    transformed.sig.output = output.clone();

    Ok(transformed)
}

/// Extracts the response type T from Result<T, ApiError>.
fn extract_response_type(output: &ReturnType) -> Result<TokenStream> {
    match output {
        ReturnType::Default => Err(Error::new(
            Span::call_site(),
            "endpoint methods must return Result<T, ApiError>",
        )),
        ReturnType::Type(_, ty) => {
            // Expecting Type::Path with Result<T, ApiError>
            if let Type::Path(TypePath { path, .. }) = &**ty
                && let Some(segment) = path.segments.last()
                && segment.ident == "Result"
            {
                // Extract the first generic argument (the T in Result<T, E>)
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(syn::GenericArgument::Type(response_ty)) = args.args.first()
                {
                    return Ok(quote! { #response_ty });
                }
            }

            Err(Error::new_spanned(
                ty,
                "endpoint methods must return Result<T, ApiError>",
            ))
        }
    }
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
