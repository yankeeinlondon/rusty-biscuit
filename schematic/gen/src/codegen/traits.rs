//! Trait generation for API clients.
//!
//! This module generates marker traits and their implementations for request structs.
//!
//! ## Generated Traits
//!
//! - [`generate_paginated_trait`] - Generates the `Paginated` marker trait
//! - [`generate_paginated_impl`] - Generates `impl Paginated` for paginated endpoints

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::Endpoint;

/// Generates the `Paginated` marker trait definition.
///
/// This trait is used to identify request types that support pagination at compile time.
/// It enables generic pagination helpers and IDE autocomplete filtering.
///
/// ## Generated Code
///
/// ```ignore
/// /// Marker trait for paginated request types.
/// ///
/// /// Request types that implement this trait support pagination parameters
/// /// and can be used with generic pagination utilities.
/// pub trait Paginated {}
/// ```
///
/// ## Examples
///
/// ```ignore
/// let tokens = generate_paginated_trait();
/// // Use in shared.rs generation
/// ```
pub fn generate_paginated_trait() -> TokenStream {
    quote! {
        /// Marker trait for paginated request types.
        ///
        /// Request types that implement this trait support pagination parameters
        /// and can be used with generic pagination utilities.
        ///
        /// ## Examples
        ///
        /// ```ignore
        /// use schematic_schema::shared::Paginated;
        ///
        /// fn fetch_all_pages<R: Paginated>(request: R) -> Vec<R::Response>
        /// where
        ///     R: EndpointSpec,
        /// {
        ///     // Implementation that handles pagination automatically
        ///     todo!()
        /// }
        /// ```
        pub trait Paginated {}
    }
}

/// Generates `impl Paginated` for an endpoint if it has pagination.
///
/// Only generates an implementation if the endpoint has pagination configured
/// via `EndpointParams::pagination`.
///
/// ## Arguments
///
/// * `endpoint` - The endpoint to check for pagination
/// * `suffix` - The suffix used for request struct names (e.g., "Request")
///
/// ## Returns
///
/// - `TokenStream` with `impl Paginated for {Endpoint}Request {}` if paginated
/// - Empty `TokenStream` if not paginated
///
/// ## Examples
///
/// ```ignore
/// let endpoint = Endpoint {
///     id: "ListItems".to_string(),
///     params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
///     // ...
/// };
///
/// let tokens = generate_paginated_impl(&endpoint, "Request");
/// // Generates: impl Paginated for ListItemsRequest {}
/// ```
pub fn generate_paginated_impl(endpoint: &Endpoint, suffix: &str) -> TokenStream {
    // Check if endpoint has pagination
    let has_pagination = endpoint
        .params
        .as_ref()
        .is_some_and(|p| p.pagination.is_some());

    if !has_pagination {
        return TokenStream::new();
    }

    let struct_name = format_ident!("{}{}", endpoint.id, suffix);

    quote! {
        impl Paginated for #struct_name {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::request_structs::format_generated_code;
    use schematic_define::params::{EndpointParams, PaginationStyle, QueryParamType};
    use schematic_define::{ApiResponse, RestMethod};

    #[test]
    fn generate_paginated_trait_produces_valid_rust() {
        let tokens = generate_paginated_trait();
        let code = format_generated_code(&tokens).expect("Should produce valid Rust code");

        assert!(code.contains("pub trait Paginated"));
        assert!(code.contains("Marker trait for paginated request types"));
    }

    #[test]
    fn generate_paginated_impl_for_github_style_pagination() {
        let endpoint = Endpoint {
            id: "ListRepositories".to_string(),
            method: RestMethod::Get,
            path: "/repos".to_string(),
            description: "List repositories".to_string(),
            request: None,
            response: ApiResponse::json_type("ListReposResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
        };

        let tokens = generate_paginated_impl(&endpoint, "Request");
        let code = format_generated_code(&tokens).expect("Should produce valid Rust code");

        assert!(code.contains("impl Paginated for ListRepositoriesRequest"));
    }

    #[test]
    fn generate_paginated_impl_for_cursor_pagination() {
        let endpoint = Endpoint {
            id: "ListEvents".to_string(),
            method: RestMethod::Get,
            path: "/events".to_string(),
            description: "List events".to_string(),
            request: None,
            response: ApiResponse::json_type("ListEventsResponse"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_pagination(PaginationStyle::cursor("after", Some("limit"), 50)),
            ),
        };

        let tokens = generate_paginated_impl(&endpoint, "Request");
        let code = format_generated_code(&tokens).expect("Should produce valid Rust code");

        assert!(code.contains("impl Paginated for ListEventsRequest"));
    }

    #[test]
    fn generate_paginated_impl_for_offset_limit_pagination() {
        let endpoint = Endpoint {
            id: "ListLogs".to_string(),
            method: RestMethod::Get,
            path: "/logs".to_string(),
            description: "List logs".to_string(),
            request: None,
            response: ApiResponse::json_type("ListLogsResponse"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_pagination(PaginationStyle::offset_limit("offset", "limit", 100, 1000)),
            ),
        };

        let tokens = generate_paginated_impl(&endpoint, "Request");
        let code = format_generated_code(&tokens).expect("Should produce valid Rust code");

        assert!(code.contains("impl Paginated for ListLogsRequest"));
    }

    #[test]
    fn skip_impl_when_endpoint_has_no_params() {
        let endpoint = Endpoint {
            id: "GetStatus".to_string(),
            method: RestMethod::Get,
            path: "/status".to_string(),
            description: "Get status".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: None,
        };

        let tokens = generate_paginated_impl(&endpoint, "Request");
        assert!(tokens.is_empty());
    }

    #[test]
    fn skip_impl_when_endpoint_has_params_but_no_pagination() {
        let endpoint = Endpoint {
            id: "Search".to_string(),
            method: RestMethod::Get,
            path: "/search".to_string(),
            description: "Search".to_string(),
            request: None,
            response: ApiResponse::json_type("SearchResponse"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_query_param("q", QueryParamType::String, true, Some("Query")),
            ),
        };

        let tokens = generate_paginated_impl(&endpoint, "Request");
        assert!(tokens.is_empty());
    }

    #[test]
    fn custom_suffix_used_in_impl() {
        let endpoint = Endpoint {
            id: "ListUsers".to_string(),
            method: RestMethod::Get,
            path: "/users".to_string(),
            description: "List users".to_string(),
            request: None,
            response: ApiResponse::json_type("ListUsersResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
        };

        let tokens = generate_paginated_impl(&endpoint, "Params");
        let code = format_generated_code(&tokens).expect("Should produce valid Rust code");

        assert!(code.contains("impl Paginated for ListUsersParams"));
    }
}
