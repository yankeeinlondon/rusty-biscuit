//! Code generation utilities for REST API macros.
//!
//! This module provides helpers for generating Rust code from parsed
//! API definitions. It handles token stream construction, identifier
//! generation, and code formatting.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use crate::parse::{ApiConfig, AuthMethod, EndpointConfig, HttpMethod, ResponseFormat};

/// Generates the identifier for a response format type.
///
/// Maps our internal representation to the actual type names
/// that will be used in the generated code.
pub fn response_format_type(format: ResponseFormat, response_type: &TokenStream) -> TokenStream {
    match format {
        ResponseFormat::Json => quote! { api::response::JsonFormat<#response_type> },
        ResponseFormat::Xml => quote! { api::response::XmlFormat<#response_type> },
        ResponseFormat::Yaml => quote! { api::response::YamlFormat<#response_type> },
        ResponseFormat::PlainText => quote! { api::response::PlainTextFormat },
        ResponseFormat::Html => quote! { api::response::HtmlFormat },
        ResponseFormat::Csv => quote! { api::response::CsvFormat },
        ResponseFormat::Binary => quote! { api::response::BinaryFormat },
    }
}

/// Generates the identifier for an HTTP method.
pub fn http_method_ident(method: HttpMethod) -> Ident {
    match method {
        HttpMethod::Get => format_ident!("Get"),
        HttpMethod::Post => format_ident!("Post"),
        HttpMethod::Put => format_ident!("Put"),
        HttpMethod::Patch => format_ident!("Patch"),
        HttpMethod::Delete => format_ident!("Delete"),
        HttpMethod::Head => format_ident!("Head"),
        HttpMethod::Options => format_ident!("Options"),
    }
}

/// Generates code to substitute path parameters in a URL path.
///
/// For a path like `/users/{id}/posts/{post_id}`, generates:
/// ```ignore
/// let path = format!("/users/{}/posts/{}", id, post_id);
/// ```
#[allow(dead_code)]
pub fn path_substitution(path: &str, params: &[String]) -> TokenStream {
    if params.is_empty() {
        let path_lit = path;
        return quote! { #path_lit.to_string() };
    }

    // Replace {param} with {} for format! macro
    let mut format_string = path.to_string();
    for param in params {
        format_string = format_string.replace(&format!("{{{}}}", param), "{}");
    }

    let param_idents: Vec<Ident> = params
        .iter()
        .map(|p| Ident::new(p, Span::call_site()))
        .collect();

    quote! {
        format!(#format_string, #(#param_idents),*)
    }
}

/// Generates an endpoint static definition.
///
/// This creates a static `Endpoint` struct for compile-time endpoint information.
#[allow(dead_code)]
pub fn endpoint_static(
    name: &Ident,
    config: &EndpointConfig,
    response_type: &TokenStream,
) -> TokenStream {
    let method = http_method_ident(config.method);
    let path = &config.path;
    let format_type = response_format_type(config.response_format, response_type);

    quote! {
        static #name: api::Endpoint<#format_type> = api::Endpoint {
            id: stringify!(#name),
            method: api::RestMethod::#method,
            path: #path,
            description: None,
            _format: ::std::marker::PhantomData,
        };
    }
}

/// Formats generated code using prettyplease.
///
/// This ensures the generated code is readable and well-formatted.
#[allow(dead_code)]
pub fn format_tokens(tokens: TokenStream) -> String {
    let file = syn::parse_file(&tokens.to_string());
    match file {
        Ok(file) => prettyplease::unparse(&file),
        Err(_) => tokens.to_string(),
    }
}

/// Generates a snake_case identifier from a string.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_lowercase = false;

    for ch in s.chars() {
        if ch.is_uppercase() {
            if prev_lowercase {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_lowercase = false;
        } else {
            result.push(ch);
            prev_lowercase = ch.is_lowercase();
        }
    }

    result
}

/// Generates a SCREAMING_SNAKE_CASE identifier from a string.
pub fn to_screaming_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

/// Generates the API client struct implementation.
///
/// This creates a struct with a `client: api::ApiClient` field and
/// a `new()` constructor that accepts an API key.
///
/// ## Examples
///
/// ```ignore
/// #[derive(RestApi)]
/// #[api(base_url = "https://api.example.com", auth = bearer)]
/// pub struct ExampleApi;
/// ```
///
/// Generates:
///
/// ```ignore
/// pub struct ExampleApi {
///     client: api::ApiClient,
/// }
///
/// impl ExampleApi {
///     pub fn new(api_key: impl Into<String>) -> Result<Self, api::ApiError> {
///         let base_url = url::Url::parse("https://api.example.com")
///             .map_err(|e| api::ConfigError::InvalidUrl(e))?;
///
///         let client = api::ApiClient::builder(base_url)
///             .auth(api::ApiAuthMethod::BearerToken, api_key)
///             .build()?;
///
///         Ok(Self { client })
///     }
///
///     pub fn base_url(&self) -> &url::Url {
///         self.client.base_url()
///     }
/// }
/// ```
pub fn generate_api_client(name: &Ident, config: &ApiConfig) -> TokenStream {
    let base_url = config.base_url.as_ref().expect("base_url validated");
    let auth_method = generate_auth_method(&config.auth);

    // Decide whether to accept an api_key parameter based on auth method
    let (new_params, auth_setup) = if matches!(config.auth, Some(AuthMethod::None) | None) {
        // No auth - no api_key parameter needed
        (quote! {}, quote! {
            let client = api::ApiClient::builder(base_url)
                .build()?;
        })
    } else {
        // Auth required - accept api_key parameter
        (quote! { api_key: impl Into<String> }, quote! {
            let client = api::ApiClient::builder(base_url)
                .auth(#auth_method, api_key)
                .build()?;
        })
    };

    quote! {
        impl #name {
            /// Creates a new API client.
            #[allow(dead_code)]
            pub fn new(#new_params) -> Result<Self, api::ApiError> {
                let base_url = url::Url::parse(#base_url)
                    .map_err(api::ConfigError::InvalidUrl)?;

                #auth_setup

                Ok(Self { client })
            }

            /// Returns the base URL for this API.
            #[allow(dead_code)]
            pub fn base_url(&self) -> &url::Url {
                self.client.base_url()
            }
        }
    }
}

/// Generates the authentication method enum variant.
fn generate_auth_method(auth: &Option<AuthMethod>) -> TokenStream {
    match auth {
        Some(AuthMethod::Bearer) => quote! { api::ApiAuthMethod::BearerToken },
        Some(AuthMethod::HeaderKey) => quote! {
            api::ApiAuthMethod::ApiKey("X-API-Key".to_string())
        },
        Some(AuthMethod::QueryParam) => quote! {
            api::ApiAuthMethod::QueryParam("key".to_string())
        },
        Some(AuthMethod::None) | None => quote! { api::ApiAuthMethod::None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_substitution_no_params() {
        let result = path_substitution("/users", &[]);
        assert!(result.to_string().contains("\"/users\""));
    }

    #[test]
    fn test_path_substitution_with_params() {
        let result = path_substitution("/users/{id}", &["id".to_string()]);
        let result_str = result.to_string();
        // quote! tokenizes `format!` as `format !` (with a space)
        assert!(result_str.contains("format"), "Expected 'format' in: {}", result_str);
        assert!(result_str.contains("id"), "Expected 'id' in: {}", result_str);
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("GetModels"), "get_models");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_to_screaming_snake_case() {
        assert_eq!(to_screaming_snake_case("HelloWorld"), "HELLO_WORLD");
        assert_eq!(to_screaming_snake_case("GetModels"), "GET_MODELS");
    }

    #[test]
    fn test_http_method_ident() {
        assert_eq!(http_method_ident(HttpMethod::Get).to_string(), "Get");
        assert_eq!(http_method_ident(HttpMethod::Post).to_string(), "Post");
        assert_eq!(http_method_ident(HttpMethod::Delete).to_string(), "Delete");
    }
}
