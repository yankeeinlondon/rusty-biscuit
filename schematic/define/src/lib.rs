//! Schematic Definition Library
//!
//! This crate provides types (primitives) for defining REST APIs in a declarative way.
//! These definitions are consumed by the `schematic-gen` binary to generate
//! strongly-typed Rust client code.
//!
//! ## Core Types
//!
//! - [`RestApi`] - A complete REST API definition with base URL, auth, and endpoints
//! - [`Endpoint`] - A single API endpoint with method, path, and schemas
//! - [`RestMethod`] - HTTP methods (GET, POST, PUT, etc.)
//! - [`AuthStrategy`] - Authentication strategies (Bearer, API Key, Basic, None)
//! - [`UpdateStrategy`] - Strategy for updating auth in API variants (NoChange, ChangeTo)
//! - [`ApiResponse`] - Response type definitions (JSON, Text, Binary, Empty)
//! - [`Schema`] - Type information for request/response bodies
//!
//! ## Examples
//!
//! Define a simple API with bearer token authentication:
//!
//! ```
//! use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};
//!
//! let api = RestApi {
//!     name: "OpenAI".to_string(),
//!     description: "OpenAI API".to_string(),
//!     base_url: "https://api.openai.com/v1".to_string(),
//!     docs_url: Some("https://platform.openai.com/docs/api-reference".to_string()),
//!     auth: AuthStrategy::BearerToken { header: None },
//!     env_auth: vec!["OPENAI_API_KEY".to_string()],
//!     env_username: None,
//!     headers: vec![],
//!     endpoints: vec![
//!         Endpoint {
//!             id: "ListModels".to_string(),
//!             method: RestMethod::Get,
//!             path: "/models".to_string(),
//!             description: "List available models".to_string(),
//!             request: None,
//!             response: ApiResponse::json_type("ListModelsResponse"),
//!             headers: vec![],
//!         },
//!     ],
//! };
//!
//! assert_eq!(api.name, "OpenAI");
//! assert_eq!(api.endpoints.len(), 1);
//! ```
//!
//! ## API Definitions
//!
//! Actual API definitions (like OpenAI) are in the separate `schematic-definitions` crate,
//! which uses these primitives to define real-world APIs.

pub mod auth;
pub mod prelude;
pub mod response;
pub mod schema;
pub mod types;

// Re-export main types at crate root
pub use auth::{AuthStrategy, UpdateStrategy};
pub use response::ApiResponse;
pub use schema::{Schema, SchemaObject};
pub use types::{Endpoint, RestApi, RestMethod};
