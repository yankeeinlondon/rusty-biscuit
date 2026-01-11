//! Type-safe REST API client library.
//!
//! The `api` crate provides primitives for defining REST APIs with type-safe
//! callable structs generated from definitions.
//!
//! ## Features
//!
//! - **Type-safe endpoints**: Define endpoints with typed request/response bodies
//! - **Multiple response formats**: JSON, YAML, XML (with optional XSD validation),
//!   HTML, CSV, PlainText, Binary
//! - **Async-first HTTP client**: Built on `reqwest` with `tokio`
//! - **Layered error handling**: Structured errors for different failure modes
//! - **OpenAPI generation**: Auto-generate OpenAPI 3.x specs from definitions
//!
//! ## Example
//!
//! ```rust,ignore
//! use api::{Endpoint, RestMethod};
//! use api::response::JsonFormat;
//!
//! #[derive(serde::Deserialize)]
//! struct User { id: u64, name: String }
//!
//! let get_user: Endpoint<JsonFormat<User>> = Endpoint::builder()
//!     .id("get_user")
//!     .method(RestMethod::Get)
//!     .path("/users/{id}")
//!     .description("Retrieve a user by ID")
//!     .build();
//! ```

pub mod auth;
pub mod client;
pub mod endpoint;
pub mod endpoint_id;
pub mod error;
pub mod method;
pub mod openapi;
pub mod response;

// Re-exports for convenience
pub use auth::ApiAuthMethod;
pub use client::{ApiClient, ApiClientBuilder};
pub use endpoint::{Endpoint, EndpointBuilder};
pub use endpoint_id::{EndpointId, EndpointIdError};
pub use error::{ApiError, AuthError, ClientError, ConfigError, ValidationError};
pub use method::RestMethod;
pub use response::{ApiResponseValue, ResponseFormat};
