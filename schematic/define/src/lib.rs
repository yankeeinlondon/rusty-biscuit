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
//! - [`CrossOrigin`] - Policy for following redirects and next-page links to other origins
//! - [`UpdateStrategy`] - Strategy for updating auth in API variants (NoChange, ChangeTo); no generated code reads it
//! - [`ApiResponse`] - Response type definitions (JSON, Text, Binary, Empty)
//! - [`ApiRequest`] - Request body type definitions (JSON, FormData, UrlEncoded, Text, Binary)
//! - [`FormField`] - Form field definitions for multipart and URL-encoded requests
//! - [`FormFieldKind`] - Form field type classification (Text, File, Files, Json)
//! - [`SchemaComponent`] - A named schema carried by [`RestApi::schemas`]; JSON bodies reference it through [`TypeRef`]
//! - [`Schema`] - Type information for WebSocket message bodies
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
//!
//! List pagination is authored per API through the [`pagination`] module.
//!
//! ## Examples
//!
//! Define a simple API with bearer token authentication:
//!
//! ```
//! use schematic_define::{DefinitionContext, RestApi, Endpoint};
//!
//! let mut ctx = DefinitionContext::types_in("crate::types");
//! let endpoint = Endpoint::get("ListModels", "/models")
//!     .describes("List available models")
//!     .returns_json::<serde_json::Value>(&mut ctx)
//!     .build().unwrap();
//! let api = RestApi::builder(&ctx, "OpenAI", "https://api.openai.com/v1")
//!     .describes("OpenAI API")
//!     .bearer_env(["OPENAI_API_KEY"])
//!     .docs_url("https://platform.openai.com/docs/api-reference")
//!     .endpoints([endpoint])
//!     .build().unwrap();
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

#![doc = include_str!("../../docs/authoring-examples.md")]

pub mod auth;
pub mod components;
pub mod cross_origin;
pub mod headers;
pub mod models;
pub mod oauth;
pub mod pagination;
pub mod params;
pub mod prelude;
pub mod request;
pub mod response;
pub mod schema;
mod serde_presence;
pub mod types;
pub mod registration;
pub mod builders;
pub mod metadata;
pub mod websocket;

#[cfg(feature = "openapi")]
pub mod openapi;

// Re-export main types at crate root
pub use auth::{
    ApiKeyLocation, AuthMethod, AuthPolicy, AuthStrategy, EndpointAuth, EnvAuthStrategy,
    UpdateStrategy,
};
pub use metadata::{Backoff, MetadataError, RateLimit, ResetUnit, RetryGuidance, StatusSelector};
pub use components::SchemaComponent;
pub use cross_origin::CrossOrigin;
pub use headers::{ApiKeyEnv, EnvList, EnvMapping, HeaderError, Headers, SensitiveString};
pub use models::{
    EnumDef, EnumVariant, FieldDef, ModelCatalog, ModelDef, PrimitiveType, StructDef, TypeAlias,
    TypeRef,
};
pub use oauth::{
    AuthorizationCodeGrant, ClientCredentialsGrant, DeviceCodeGrant, OAuth2ClientAuthMethod,
    OAuth2Config, OAuth2ConfigError, OAuth2GrantType, PkceRequirement,
};
pub use params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
pub use request::{ApiRequest, FormField, FormFieldKind};
pub use response::ApiResponse;
pub use schema::{Schema, SchemaObject};
pub use types::{Endpoint, ResolvedEndpointAuth, RestApi, RestMethod};
pub use registration::{DefinitionContext, DefinitionError, MapKey, Registration, RustPath, TypeDesc, TypeRegistration};
pub use websocket::{
    AuthFlowHints, ConnectionLifecycle, ConnectionParam, CorrelationHints, FrameFormat,
    HeartbeatHints, MessageDirection, MessageSchema, ParamType, RequestIdType, WebSocketApi,
    WebSocketEndpoint, WebSocketEndpointHints, WebSocketRuntimeHints,
};

pub use builders::{EndpointBuilder, RestApiBuilder, Success, ErrorSet};
