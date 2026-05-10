//! Schematic Definition Library
//!
//! This crate provides types (primitives) for defining REST APIs in a declarative way.
//! These definitions are consumed by the `schematic-gen` binary to generate
//! strongly-typed Rust client code.
//!
//! ## Core Types
//!
//! ### REST API Types
//!
//! - [`RestApi`] - A complete REST API definition with base URL, auth, and endpoints
//! - [`Endpoint`] - A single API endpoint with method, path, and schemas
//! - [`RestMethod`] - HTTP methods (GET, POST, PUT, etc.)
//! - [`AuthStrategy`] - Legacy single-strategy authentication model
//! - [`AuthPolicy`] - Explicit auth methods plus environment fallback policy
//! - [`UpdateStrategy`] - Strategy for updating auth in API variants (NoChange, ChangeTo)
//! - [`ApiResponse`] - Response type definitions (JSON, Text, Binary, Empty)
//! - [`ApiRequest`] - Request body type definitions (JSON, FormData, UrlEncoded, Text, Binary)
//! - [`FormField`] - Form field definitions for multipart and URL-encoded requests
//! - [`FormFieldKind`] - Form field type classification (Text, File, Files, Json)
//! - [`Schema`] - Type information for request/response bodies
//!
//! ### Header and Authentication Types
//!
//! - [`Headers`] - Fluent builder for HTTP headers with auth support
//! - [`SensitiveString`] - Secure wrapper for passwords/tokens (redacts Debug output)
//! - [`EnvList`] - Environment variable fallback chain for credentials
//! - [`ApiKeyEnv`] - API key header configuration with environment source
//! - [`EnvMapping`] - Complete environment variable mapping for auth credentials
//! - [`HeaderError`] - Errors from header validation and credential resolution
//!
//! ### WebSocket API Types
//!
//! - [`WebSocketApi`] - Complete WebSocket API definition with base URL, auth, and endpoints
//! - [`WebSocketEndpoint`] - Single WebSocket endpoint with path, parameters, and message schemas
//! - [`ConnectionParam`] - Query/path parameter definition for WebSocket connections
//! - [`ParamType`] - Parameter types (String, Integer, Boolean, Float)
//! - [`ConnectionLifecycle`] - Open, close, and keepalive message schemas
//! - [`MessageSchema`] - Single message type with direction and schema
//! - [`MessageDirection`] - Message flow direction (Client, Server, Bidirectional)
//!
//! ### Model Definition Types (for API schema import)
//!
//! - [`ModelCatalog`] - Collection of model definitions with optional module path
//! - [`ModelDef`] - Union of model types (struct, enum, or alias)
//! - [`StructDef`] - Structure definition with fields
//! - [`EnumDef`] - Enumeration definition with variants
//! - [`TypeAlias`] - Type alias definition
//! - [`FieldDef`] - Field definition for structs
//! - [`EnumVariant`] - Variant definition for enums
//! - [`TypeRef`] - Type reference (primitives, arrays, named types, combinators)
//! - [`PrimitiveType`] - Basic primitive types
//!
//! ### Parameter Definition Types (for API endpoint import)
//!
//! - [`EndpointParams`] - Collection of endpoint parameters (query, header, cookie)
//! - [`ParamDef`] - Single parameter definition
//! - [`QueryParamType`] - Parameter value type
//! - [`ParamStyle`] - Parameter serialization style
//! - [`PaginationStyle`] - Common pagination request patterns
//! - [`PaginationResponse`] - How APIs signal pagination state in responses
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
//!     auth_policy: None,
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
//!             params: None,
//!             oauth_scopes: None,
//!         },
//!     ],
//!     module_path: None,
//!     request_suffix: None,
//!     version: None,
//!     env_mapping: None,
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
//!
//! ## Body Type Patterns
//!
//! Request body types in API definitions should follow the builder pattern
//! for ergonomic construction:
//!
//! ```text
//! // Core constructor with required fields
//! CreateMessageBody::new("claude-sonnet-4-5-20250514", messages, 1024)
//!     .with_system("You are a helpful assistant")
//!     .with_temperature(0.7)
//!     .with_tools(tools)
//! ```
//!
//! ### Recommended Methods
//!
//! - `new()` - Constructor requiring all mandatory fields
//! - `with_*()` - Builder methods for optional fields (return `Self` for chaining)
//! - `Default` - Implement when all fields have sensible defaults
//!
//! ### Pattern Benefits
//!
//! - **Discoverability**: IDE autocomplete reveals all optional fields
//! - **Readability**: Configuration intent is clear at the call site
//! - **Flexibility**: Add new optional fields without breaking existing code
//!
//! See `schematic_definitions::anthropic` for a comprehensive example.

pub mod auth;
pub mod headers;
pub mod models;
pub mod oauth;
pub mod pagination;
pub mod params;
pub mod prelude;
pub mod request;
pub mod response;
pub mod schema;
pub mod types;
pub mod websocket;

#[cfg(feature = "openapi")]
pub mod openapi;

// Re-export main types at crate root
pub use auth::{
    ApiKeyLocation, AuthMethod, AuthPolicy, AuthStrategy, EnvAuthStrategy, UpdateStrategy,
};
pub use headers::{ApiKeyEnv, EnvList, EnvMapping, HeaderError, Headers, SensitiveString};
pub use models::{
    EnumDef, EnumVariant, FieldDef, ModelCatalog, ModelDef, PrimitiveType, StructDef, TypeAlias,
    TypeRef,
};
pub use oauth::{OAuth2ClientAuthMethod, OAuth2Config, OAuth2GrantType, PkceRequirement};
pub use pagination::{PaginationResponse, PaginationStyle};
pub use params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
pub use request::{ApiRequest, FormField, FormFieldKind};
pub use response::ApiResponse;
pub use schema::{Schema, SchemaObject};
pub use types::{Endpoint, RestApi, RestMethod};
pub use websocket::{
    AuthFlowHints, ConnectionLifecycle, ConnectionParam, CorrelationHints, FrameFormat,
    HeartbeatHints, MessageDirection, MessageSchema, ParamType, RequestIdType, WebSocketApi,
    WebSocketEndpoint, WebSocketEndpointHints, WebSocketRuntimeHints,
};

/// Core API definition types.
///
/// This module groups the fundamental types for defining REST API structures,
/// authentication, requests, and responses.
pub mod core {
    pub use crate::auth::*;
    pub use crate::request::*;
    pub use crate::response::*;
    pub use crate::schema::*;
    pub use crate::types::*;
}

/// Transport-layer types for HTTP headers, parameters, and WebSocket APIs.
///
/// This module groups types related to the transport layer of API communication.
pub mod transport {
    pub use crate::headers::*;
    pub use crate::pagination::*;
    pub use crate::params::*;
    pub use crate::websocket::*;
}

/// Model definition types for API schema import.
///
/// This module groups types used to define data models (structs, enums, type aliases)
/// when importing API specifications.
pub mod model {
    pub use crate::models::*;
}
