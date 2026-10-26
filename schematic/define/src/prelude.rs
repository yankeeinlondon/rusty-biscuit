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
//! let ctx = DefinitionContext::types_in("crate::types");
//! let api = RestApi::builder(&ctx, "MyAPI", "https://api.example.com")
//!     .describes("My API")
//!     .bearer_env(["API_KEY"])
//!     .build().unwrap();
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
//!     auth_policy: None,
//!     env_auth: vec!["STREAM_KEY".to_string()],
//!     env_mapping: None,
//!     headers: vec![],
//!     version: None,
//!     endpoints: vec![],
//!     runtime: None,
//! };
//! ```

pub use crate::auth::{
    AuthMethod, AuthPolicy, AuthStrategy, EndpointAuth, EnvAuthStrategy, UpdateStrategy,
};
pub use crate::metadata::{
    Backoff, MetadataError, RateLimit, ResetUnit, RetryGuidance, StatusSelector,
};
pub use crate::cross_origin::CrossOrigin;
pub use crate::oauth::{
    AuthorizationCodeGrant, ClientCredentialsGrant, DeviceCodeGrant, OAuth2ClientAuthMethod,
    OAuth2Config, OAuth2ConfigError, OAuth2GrantType, PkceRequirement,
};
pub use crate::pagination::{
    Carry, Collection, CursorLeg, CursorSource, FollowUp, InputParam, KeepAlive, LinkSource,
    MoreSignal, PageSize, Partition,
    Session, SessionStep, Strategy, ValueSource,
};
pub use crate::params::EndpointParams;
pub use crate::request::{ApiRequest, FormField, FormFieldKind};
pub use crate::response::{ResponseShape, StreamEnd, StreamFraming, SuccessShape};
pub use crate::schema::{Schema, SchemaObject};
pub use crate::types::{Endpoint, ResolvedEndpointAuth, RestApi, RestMethod};
pub use crate::websocket::{
    AuthFlowHints, ConnectionLifecycle, CorrelationHints, EndpointRole, FrameFormat,
    HeartbeatHints, MessageDirection, MessageSchema, MessageSelector, RequestIdType,
    WebSocketApi, WebSocketEndpoint, WebSocketEndpointHints, WebSocketRuntimeHints,
};

pub use crate::{DefinitionContext, DefinitionError, Registration, TypeDesc, Success, ErrorSet, EndpointBuilder, RestApiBuilder};
pub use crate::{AddressRule, ErrorShape, GraphQlOperation, StatusMatch};
