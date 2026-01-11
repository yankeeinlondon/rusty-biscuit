//! Custom parsing for API macro definitions.
//!
//! This module contains the data structures and parsing logic for
//! extracting API configuration from macro attributes.

use proc_macro2::Span;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Error, Ident, Result, Token,
};

/// Parsed API-level configuration from `#[api(...)]` attributes.
#[derive(Debug, Default)]
pub struct ApiConfig {
    /// Base URL for the API (required)
    pub base_url: Option<String>,
    /// Authentication method
    pub auth: Option<AuthMethod>,
    /// Documentation URL
    pub docs_url: Option<String>,
    /// OpenAPI specification URL
    pub openapi_url: Option<String>,
    /// Span for error reporting
    pub span: Option<Span>,
}

impl ApiConfig {
    /// Parse API configuration from a list of attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut config = ApiConfig::default();

        for attr in attrs {
            if attr.path().is_ident("api") {
                config.parse_api_attr(attr)?;
            }
        }

        Ok(config)
    }

    fn parse_api_attr(&mut self, attr: &Attribute) -> Result<()> {
        self.span = Some(attr.span());

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("base_url") {
                let value: LitStr = meta.value()?.parse()?;
                self.base_url = Some(value.value());
            } else if meta.path.is_ident("auth") {
                let value: Ident = meta.value()?.parse()?;
                self.auth = Some(AuthMethod::from_ident(&value)?);
            } else if meta.path.is_ident("docs") {
                let value: LitStr = meta.value()?.parse()?;
                self.docs_url = Some(value.value());
            } else if meta.path.is_ident("openapi") {
                let value: LitStr = meta.value()?.parse()?;
                self.openapi_url = Some(value.value());
            } else {
                return Err(meta.error(format!(
                    "unknown api attribute: `{}`",
                    meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                )));
            }
            Ok(())
        })
    }

    /// Validate that required fields are present.
    pub fn validate(&self) -> Result<()> {
        if self.base_url.is_none() {
            return Err(Error::new(
                self.span.unwrap_or_else(Span::call_site),
                "missing required `base_url` attribute: #[api(base_url = \"...\")]",
            ));
        }
        Ok(())
    }
}

use syn::LitStr;

/// Authentication method for the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// Bearer token authentication (Authorization: Bearer <token>)
    Bearer,
    /// API key in a custom header
    HeaderKey,
    /// API key as a query parameter
    QueryParam,
    /// No authentication required
    None,
}

impl AuthMethod {
    /// Parse from an identifier.
    pub fn from_ident(ident: &Ident) -> Result<Self> {
        match ident.to_string().as_str() {
            "bearer" => Ok(AuthMethod::Bearer),
            "header_key" => Ok(AuthMethod::HeaderKey),
            "query_param" => Ok(AuthMethod::QueryParam),
            "none" => Ok(AuthMethod::None),
            other => Err(Error::new(
                ident.span(),
                format!(
                    "unknown auth method: `{}`. Expected one of: bearer, header_key, query_param, none",
                    other
                ),
            )),
        }
    }
}

/// HTTP method for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    /// Parse from an identifier.
    pub fn from_ident(ident: &Ident) -> Result<Self> {
        match ident.to_string().as_str() {
            "Get" | "GET" | "get" => Ok(HttpMethod::Get),
            "Post" | "POST" | "post" => Ok(HttpMethod::Post),
            "Put" | "PUT" | "put" => Ok(HttpMethod::Put),
            "Patch" | "PATCH" | "patch" => Ok(HttpMethod::Patch),
            "Delete" | "DELETE" | "delete" => Ok(HttpMethod::Delete),
            "Head" | "HEAD" | "head" => Ok(HttpMethod::Head),
            "Options" | "OPTIONS" | "options" => Ok(HttpMethod::Options),
            other => Err(Error::new(
                ident.span(),
                format!(
                    "unknown HTTP method: `{}`. Expected one of: Get, Post, Put, Patch, Delete, Head, Options",
                    other
                ),
            )),
        }
    }

    /// Returns true if this method typically has a request body.
    #[allow(dead_code)]
    pub fn has_body(&self) -> bool {
        matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
    }
}

/// Response format for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseFormat {
    #[default]
    Json,
    Xml,
    Yaml,
    PlainText,
    Html,
    Csv,
    Binary,
}

impl ResponseFormat {
    /// Parse from an identifier.
    pub fn from_ident(ident: &Ident) -> Result<Self> {
        match ident.to_string().as_str() {
            "json" | "Json" | "JSON" => Ok(ResponseFormat::Json),
            "xml" | "Xml" | "XML" => Ok(ResponseFormat::Xml),
            "yaml" | "Yaml" | "YAML" => Ok(ResponseFormat::Yaml),
            "plain_text" | "PlainText" | "text" => Ok(ResponseFormat::PlainText),
            "html" | "Html" | "HTML" => Ok(ResponseFormat::Html),
            "csv" | "Csv" | "CSV" => Ok(ResponseFormat::Csv),
            "binary" | "Binary" | "bytes" => Ok(ResponseFormat::Binary),
            other => Err(Error::new(
                ident.span(),
                format!(
                    "unknown response format: `{}`. Expected one of: json, xml, yaml, plain_text, html, csv, binary",
                    other
                ),
            )),
        }
    }
}

/// Request body format for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestFormat {
    #[default]
    Json,
    Xml,
    Form,
}

impl RequestFormat {
    /// Parse from an identifier.
    pub fn from_ident(ident: &Ident) -> Result<Self> {
        match ident.to_string().as_str() {
            "json" | "Json" | "JSON" => Ok(RequestFormat::Json),
            "xml" | "Xml" | "XML" => Ok(RequestFormat::Xml),
            "form" | "Form" | "FORM" => Ok(RequestFormat::Form),
            other => Err(Error::new(
                ident.span(),
                format!(
                    "unknown request format: `{}`. Expected one of: json, xml, form",
                    other
                ),
            )),
        }
    }
}

/// Parsed endpoint configuration from method attributes.
#[derive(Debug)]
pub struct EndpointConfig {
    /// HTTP method
    pub method: HttpMethod,
    /// URL path (may contain path parameters like `{id}`)
    pub path: String,
    /// Response format
    pub response_format: ResponseFormat,
    /// Request body format (if applicable)
    #[allow(dead_code)]
    pub request_format: Option<RequestFormat>,
    /// Span for error reporting
    #[allow(dead_code)]
    pub span: Span,
}

impl EndpointConfig {
    /// Parse endpoint configuration from method attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Option<Self>> {
        let mut method: Option<HttpMethod> = None;
        let mut path: Option<String> = None;
        let mut response_format: Option<ResponseFormat> = None;
        let mut request_format: Option<RequestFormat> = None;
        let mut span: Option<Span> = None;

        for attr in attrs {
            if attr.path().is_ident("endpoint") {
                span = Some(attr.span());
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("method") {
                        let value: Ident = meta.value()?.parse()?;
                        method = Some(HttpMethod::from_ident(&value)?);
                    } else if meta.path.is_ident("path") {
                        let value: LitStr = meta.value()?.parse()?;
                        path = Some(value.value());
                    } else {
                        return Err(meta.error(format!(
                            "unknown endpoint attribute: `{}`",
                            meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                        )));
                    }
                    Ok(())
                })?;
            } else if attr.path().is_ident("response") {
                attr.parse_nested_meta(|meta| {
                    if let Some(ident) = meta.path.get_ident() {
                        response_format = Some(ResponseFormat::from_ident(ident)?);
                    }
                    Ok(())
                })?;
            } else if attr.path().is_ident("request") {
                attr.parse_nested_meta(|meta| {
                    if let Some(ident) = meta.path.get_ident() {
                        request_format = Some(RequestFormat::from_ident(ident)?);
                    }
                    Ok(())
                })?;
            }
        }

        // If no endpoint attribute, return None (not an endpoint method)
        let span = match span {
            Some(s) => s,
            None => return Ok(None),
        };

        let method = method.ok_or_else(|| {
            Error::new(span, "missing `method` in endpoint attribute")
        })?;

        let path = path.ok_or_else(|| {
            Error::new(span, "missing `path` in endpoint attribute")
        })?;

        Ok(Some(EndpointConfig {
            method,
            path,
            response_format: response_format.unwrap_or_default(),
            request_format,
            span,
        }))
    }

    /// Extract path parameters from the path string.
    ///
    /// For example, `/users/{id}/posts/{post_id}` returns `["id", "post_id"]`.
    pub fn path_params(&self) -> Vec<String> {
        let mut params = Vec::new();
        let mut in_param = false;
        let mut current_param = String::new();

        for ch in self.path.chars() {
            match ch {
                '{' => {
                    in_param = true;
                    current_param.clear();
                }
                '}' => {
                    if in_param && !current_param.is_empty() {
                        params.push(current_param.clone());
                    }
                    in_param = false;
                }
                _ if in_param => {
                    current_param.push(ch);
                }
                _ => {}
            }
        }

        params
    }
}

/// Input for the endpoints attribute macro.
#[derive(Debug)]
pub struct EndpointsInput {
    /// The API type this impl block is for
    pub api_type: Ident,
}

impl Parse for EndpointsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut api_type: Option<Ident> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "api" {
                let _: Token![=] = input.parse()?;
                api_type = Some(input.parse()?);
            } else {
                return Err(Error::new(
                    ident.span(),
                    format!("unknown attribute: `{}`. Expected `api`", ident),
                ));
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let api_type = api_type.ok_or_else(|| {
            Error::new(input.span(), "missing `api` attribute: #[endpoints(api = TypeName)]")
        })?;

        Ok(EndpointsInput { api_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_params_extraction() {
        let config = EndpointConfig {
            method: HttpMethod::Get,
            path: "/users/{id}/posts/{post_id}".to_string(),
            response_format: ResponseFormat::Json,
            request_format: None,
            span: Span::call_site(),
        };

        let params = config.path_params();
        assert_eq!(params, vec!["id", "post_id"]);
    }

    #[test]
    fn test_path_params_no_params() {
        let config = EndpointConfig {
            method: HttpMethod::Get,
            path: "/users/all".to_string(),
            response_format: ResponseFormat::Json,
            request_format: None,
            span: Span::call_site(),
        };

        let params = config.path_params();
        assert!(params.is_empty());
    }

    #[test]
    fn test_http_method_has_body() {
        assert!(!HttpMethod::Get.has_body());
        assert!(HttpMethod::Post.has_body());
        assert!(HttpMethod::Put.has_body());
        assert!(HttpMethod::Patch.has_body());
        assert!(!HttpMethod::Delete.has_body());
    }
}
