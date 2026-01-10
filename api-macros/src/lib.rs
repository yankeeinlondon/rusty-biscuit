//! Procedural macros for REST API definition and code generation.
//!
//! This crate provides two main macros for defining type-safe REST APIs:
//!
//! - [`RestApi`] - A derive macro for API client structs
//! - [`endpoints`] - An attribute macro for defining API endpoints
//!
//! ## Examples
//!
//! ```ignore
//! use api_macros::{RestApi, endpoints};
//!
//! #[derive(RestApi)]
//! #[api(base_url = "https://api.example.com")]
//! #[api(auth = bearer)]
//! pub struct ExampleApi;
//!
//! #[endpoints(api = ExampleApi)]
//! impl ExampleApi {
//!     #[endpoint(method = Get, path = "/models")]
//!     #[response(json)]
//!     pub async fn get_models(&self) -> Result<ModelsResponse, ApiError>;
//! }
//! ```

use proc_macro::TokenStream;

mod codegen;
mod derive_api;
mod endpoints;
mod parse;

/// Derive macro for REST API client structs.
///
/// This macro generates the client infrastructure for making API calls.
/// It should be applied to a unit struct that represents an API.
///
/// ## Attributes
///
/// - `#[api(base_url = "...")]` - The base URL for all API requests (required)
/// - `#[api(auth = bearer|header_key|query_param)]` - Authentication method (optional)
/// - `#[api(docs = "...")]` - Documentation URL (optional)
/// - `#[api(openapi = "...")]` - OpenAPI specification URL (optional)
///
/// ## Examples
///
/// ```ignore
/// #[derive(RestApi)]
/// #[api(base_url = "https://api.example.com")]
/// #[api(auth = bearer)]
/// #[api(docs = "https://docs.example.com")]
/// pub struct MyApi;
/// ```
#[proc_macro_derive(RestApi, attributes(api))]
pub fn derive_rest_api(input: TokenStream) -> TokenStream {
    derive_api::derive_rest_api_impl(input.into()).into()
}

/// Attribute macro for defining API endpoints.
///
/// This macro transforms method signatures in an impl block into
/// executable API endpoint methods.
///
/// ## Attributes
///
/// On the impl block:
/// - `#[endpoints(api = TypeName)]` - Associates endpoints with an API struct
///
/// On each method:
/// - `#[endpoint(method = Get|Post|Put|Patch|Delete, path = "...")]` - HTTP method and path
/// - `#[request(json|xml|form)]` - Request body format (optional)
/// - `#[response(json|xml|yaml|plain_text|html|csv|binary)]` - Response format
///
/// ## Path Parameters
///
/// Path parameters use curly braces: `/users/{id}` will extract `id` from method arguments.
///
/// ## Examples
///
/// ```ignore
/// #[endpoints(api = MyApi)]
/// impl MyApi {
///     /// Get all models
///     #[endpoint(method = Get, path = "/models")]
///     #[response(json)]
///     pub async fn get_models(&self) -> Result<Vec<Model>, ApiError>;
///
///     /// Create a new user
///     #[endpoint(method = Post, path = "/users")]
///     #[request(json)]
///     #[response(json)]
///     pub async fn create_user(&self, input: CreateUserInput) -> Result<User, ApiError>;
///
///     /// Get user by ID
///     #[endpoint(method = Get, path = "/users/{id}")]
///     #[response(json)]
///     pub async fn get_user(&self, id: String) -> Result<User, ApiError>;
/// }
/// ```
#[proc_macro_attribute]
pub fn endpoints(attr: TokenStream, item: TokenStream) -> TokenStream {
    endpoints::endpoints_impl(attr.into(), item.into()).into()
}
