//! Convenient re-exports for working with schematic definitions.
//!
//! This prelude provides all the core types needed to define REST and WebSocket APIs.
//!
//! ## Examples
//!
//! ### REST API
//!
//! ```
//! use schematic_define::prelude::*;
//!
//! let api = RestApi {
//!     name: "MyAPI".to_string(),
//!     description: "My API".to_string(),
//!     base_url: "https://api.example.com".to_string(),
//!     docs_url: None,
//!     auth: AuthStrategy::BearerToken { header: None },
//!     env_auth: vec!["API_KEY".to_string()],
//!     env_username: None,
//!     headers: vec![],
//!     endpoints: vec![],
//! };
//! ```
//!
//! ### WebSocket API
//!
//! ```
//! use schematic_define::prelude::*;
//!
//! let ws_api = WebSocketApi {
//!     name: "MyStreamAPI".to_string(),
//!     description: "Streaming API".to_string(),
//!     base_url: "wss://stream.example.com".to_string(),
//!     docs_url: None,
//!     auth: AuthStrategy::BearerToken { header: None },
//!     env_auth: vec!["STREAM_KEY".to_string()],
//!     endpoints: vec![],
//! };
//! ```

pub use crate::auth::{AuthStrategy, UpdateStrategy};
pub use crate::request::{ApiRequest, FormField, FormFieldKind};
pub use crate::response::ApiResponse;
pub use crate::schema::{Schema, SchemaObject};
pub use crate::types::{Endpoint, RestApi, RestMethod};
pub use crate::websocket::{
    ConnectionLifecycle, ConnectionParam, MessageDirection, MessageSchema, ParamType, WebSocketApi,
    WebSocketEndpoint,
};
