//! WebSocket API definitions.
//!
//! This module provides types for defining WebSocket APIs in a declarative way,
//! parallel to the REST API types in [`crate::types`]. They reuse the REST
//! vocabulary for everything a handshake shares with an HTTP request:
//! [`AuthStrategy`], [`AuthPolicy`], [`EnvMapping`], constant headers, and
//! location-aware [`EndpointParams`].
//!
//! ## Core Types
//!
//! - [`WebSocketApi`] - Complete WebSocket API definition with base URL, auth, and endpoints
//! - [`WebSocketEndpoint`] - Single WebSocket endpoint with path, parameters, role, and message schemas
//! - [`EndpointRole`] - Which side the generator emits for an endpoint (client, host, or both)
//! - [`ConnectionLifecycle`] - Open, close, and keepalive message schemas
//! - [`MessageSchema`] - Single message type with direction, schema, selectors, and response link
//! - [`MessageSelector`] - Equality check on an inbound wire field
//! - [`MessageDirection`] - Client-relative message direction (Client, Server, Bidirectional)
//!
//! ## Examples
//!
//! Define a WebSocket API like ElevenLabs Text-to-Speech:
//!
//! ```
//! use schematic_define::websocket::*;
//! use schematic_define::{AuthStrategy, EndpointParams, ParamDef, QueryParamType, Schema};
//!
//! let api = WebSocketApi {
//!     name: "ElevenLabsTTS".to_string(),
//!     description: "ElevenLabs Text-to-Speech WebSocket API".to_string(),
//!     base_url: "wss://api.elevenlabs.io/v1".to_string(),
//!     docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
//!     auth: AuthStrategy::ApiKey { header: "xi-api-key".to_string(), value_prefix: None },
//!     auth_policy: None,
//!     env_auth: vec!["ELEVEN_LABS_API_KEY".to_string()],
//!     env_mapping: None,
//!     headers: vec![],
//!     version: None,
//!     endpoints: vec![WebSocketEndpoint {
//!         id: "TextToSpeech".to_string(),
//!         path: "/text-to-speech/{voice_id}/stream-input".to_string(),
//!         description: "Stream text and receive audio chunks".to_string(),
//!         params: EndpointParams {
//!             path: vec![ParamDef::path("voice_id", QueryParamType::String)],
//!             query: vec![ParamDef::new("model_id", QueryParamType::String).describes("Model to use")],
//!             ..EndpointParams::default()
//!         },
//!         lifecycle: ConnectionLifecycle {
//!             open: Some(MessageSchema::new(
//!                 "BOS",
//!                 MessageDirection::Client,
//!                 Schema::new("BeginOfStreamMessage"),
//!             )),
//!             close: None,
//!             keepalive: None,
//!         },
//!         messages: vec![
//!             MessageSchema::new("TextChunk", MessageDirection::Client, Schema::new("TextChunkMessage")),
//!             MessageSchema::new("AudioChunk", MessageDirection::Server, Schema::new("AudioChunkResponse")),
//!         ],
//!         ..WebSocketEndpoint::default()
//!     }],
//!     runtime: None,
//! };
//!
//! assert_eq!(api.endpoints[0].role, EndpointRole::Client);
//! assert_eq!(api.endpoints[0].params.path[0].name, "voice_id");
//! ```
//!
//! Handshake parameters are REST [`ParamDef`](crate::ParamDef)s; the former
//! WebSocket-only parameter types no longer resolve:
//!
//! ```compile_fail
//! use schematic_define::websocket::ConnectionParam;
//! ```
//!
//! ```compile_fail
//! use schematic_define::websocket::ParamType;
//! ```
//!
//! ```compile_fail
//! use schematic_define::ConnectionParam;
//! ```
//!
//! ```compile_fail
//! use schematic_define::ParamType;
//! ```

mod hints;
mod lifecycle;

pub use hints::{
    AuthFlowHints, CorrelationHints, FrameFormat, HeartbeatHints, RequestIdType,
    WebSocketEndpointHints, WebSocketRuntimeHints,
};
pub use lifecycle::{ConnectionLifecycle, MessageDirection, MessageSchema, MessageSelector};

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::auth::{AuthPolicy, AuthStrategy};
use crate::headers::EnvMapping;
use crate::params::EndpointParams;
use crate::types::RestApi;

/// Compiles the examples of `schematic/docs/websocket-definitions.md`.
#[cfg(doctest)]
#[doc = include_str!("../../../docs/websocket-definitions.md")]
pub struct WebSocketDefinitionsDoc;

/// Which side of an endpoint the generator emits.
///
/// Message directions stay client-relative whatever the role: `Client`
/// messages are what a generated host receives, `Server` messages what it
/// sends. Serialized as `client`, `host`, or `both`; an absent role is
/// `Client`.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::EndpointRole;
///
/// assert_eq!(EndpointRole::default(), EndpointRole::Client);
/// assert_eq!(serde_json::to_string(&EndpointRole::Both).unwrap(), r#""both""#);
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum EndpointRole {
    /// A client that dials the endpoint.
    #[default]
    Client,
    /// A host that accepts connections on the endpoint.
    Host,
    /// Both a client and a host.
    Both,
}

/// A single WebSocket endpoint definition.
///
/// Defines how to connect to a WebSocket endpoint, including the path,
/// handshake parameters, generated role, and message schemas.
///
/// ## Path and Query Parameters
///
/// Paths support template parameters using curly braces: `/stream/{channel_id}`.
/// With an empty `params.path`, each placeholder is a required string;
/// otherwise `params.path` types them and its names must equal the
/// placeholders, as for REST endpoints. Required query parameters become
/// connect arguments and optional ones are omitted when unset.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{
///     EndpointRole, MessageDirection, MessageSchema, WebSocketEndpoint,
/// };
/// use schematic_define::{EndpointParams, QueryParamType, Schema};
///
/// let endpoint = WebSocketEndpoint {
///     id: "StreamAudio".to_string(),
///     path: "/audio/{voice_id}/stream".to_string(),
///     description: "Stream audio synthesis".to_string(),
///     params: EndpointParams::default()
///         .with_query_param("quality", QueryParamType::String, false, Some("Audio quality")),
///     messages: vec![MessageSchema::new(
///         "AudioData",
///         MessageDirection::Server,
///         Schema::new("AudioDataMessage"),
///     )],
///     ..WebSocketEndpoint::default()
/// };
///
/// assert_eq!(endpoint.role, EndpointRole::Client);
/// assert!(endpoint.path.contains("{voice_id}"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSocketEndpoint {
    /// Identifier for this endpoint (becomes the struct/method name).
    ///
    /// Should be PascalCase (e.g., "TextToSpeech", "StreamAudio").
    pub id: String,
    /// Path template (e.g., "/stream/{channel_id}"), appended to the API's
    /// `base_url`. It carries no query string and no origin of its own.
    pub path: String,
    /// Human-readable description of what this endpoint does.
    pub description: String,
    /// Handshake parameters by location: typed path placeholders and query
    /// parameters. Header and cookie parameters are not generated for a
    /// handshake and are rejected at generation.
    #[serde(deserialize_with = "crate::serde_presence::map_only")]
    pub params: EndpointParams,
    /// Generated side of this endpoint; absent means [`EndpointRole::Client`].
    #[serde(default)]
    pub role: EndpointRole,
    /// Lifecycle messages (open, close, keepalive).
    pub lifecycle: ConnectionLifecycle,
    /// Regular messages that can be sent/received.
    pub messages: Vec<MessageSchema>,
    /// Optional runtime hints for code generation.
    #[serde(
        default,
        deserialize_with = "crate::serde_presence::non_null_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub runtime: Option<WebSocketEndpointHints>,
}

/// A complete WebSocket API definition.
///
/// This struct captures all the information needed to generate a typed client
/// for a WebSocket API, including the base URL, authentication strategy, and
/// all endpoint definitions. Authentication, environment mapping, and
/// constant headers carry the same meaning as on [`RestApi`].
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::WebSocketApi;
/// use schematic_define::{AuthStrategy, EnvList, EnvMapping};
///
/// let api = WebSocketApi {
///     name: "MyStreamingAPI".to_string(),
///     description: "Real-time streaming API".to_string(),
///     base_url: "wss://stream.example.com/v1".to_string(),
///     docs_url: Some("https://docs.example.com/websocket".to_string()),
///     auth: AuthStrategy::BearerToken { header: None },
///     auth_policy: None,
///     env_auth: vec!["STREAM_API_KEY".to_string()],
///     env_mapping: Some(EnvMapping {
///         bearer_token: Some(EnvList::single("STREAM_API_KEY")),
///         ..EnvMapping::default()
///     }),
///     headers: vec![("X-Client".to_string(), "schematic".to_string())],
///     version: None,
///     endpoints: vec![],
///     runtime: None,
/// };
///
/// assert_eq!(api.default_env_mapping().bearer_token.unwrap().names(), ["STREAM_API_KEY"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSocketApi {
    /// Unique identifier for this API (used for generated struct names).
    ///
    /// This becomes the generated client struct name (e.g., "ElevenLabs"
    /// generates `struct ElevenLabsWebSocket`).
    pub name: String,
    /// Human-readable description of the API.
    pub description: String,
    /// Base URL for WebSocket connections (e.g., `wss://api.example.com/v1`).
    ///
    /// Endpoint paths are appended to this URL when connecting; a base path
    /// is kept as a prefix.
    pub base_url: String,
    /// Link to API documentation (optional).
    pub docs_url: Option<String>,
    /// Handshake authentication strategy, as [`RestApi::auth`].
    pub auth: AuthStrategy,
    /// Structured authentication policy, as [`RestApi::auth_policy`]; `None`
    /// derives one from [`Self::auth`]. See [`Self::effective_auth_policy`].
    #[serde(
        default,
        deserialize_with = "crate::serde_presence::non_null_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub auth_policy: Option<AuthPolicy>,
    /// Ordered environment fallback chain, as [`RestApi::env_auth`]: used by
    /// token and key strategies, never a Basic source.
    pub env_auth: Vec<String>,
    /// Explicit environment mapping, as [`RestApi::env_mapping`]; `None`
    /// derives one from `auth` and `env_auth`. See
    /// [`Self::default_env_mapping`].
    #[serde(
        default,
        deserialize_with = "crate::serde_presence::non_null_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub env_mapping: Option<EnvMapping>,
    /// Constant handshake headers, as [`RestApi::headers`].
    pub headers: Vec<(String, String)>,
    /// Semantic version of this API definition (e.g., `"1.0.0"`).
    ///
    /// Used as the `info.version` field in exported API documents.
    /// If `None`, falls back to a default of `"1.0.0"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// All endpoints defined for this API.
    pub endpoints: Vec<WebSocketEndpoint>,
    /// Optional runtime hints for code generation.
    #[serde(
        default,
        deserialize_with = "crate::serde_presence::non_null_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub runtime: Option<WebSocketRuntimeHints>,
}

impl WebSocketApi {
    /// The authentication policy: [`Self::auth_policy`] when set, otherwise
    /// the one [`AuthPolicy::from_auth_strategy`] derives from [`Self::auth`].
    #[must_use]
    pub fn effective_auth_policy(&self) -> AuthPolicy {
        self.auth_policy
            .clone()
            .unwrap_or_else(|| AuthPolicy::from_auth_strategy(&self.auth))
    }

    /// The environment mapping: [`Self::env_mapping`] when set (it replaces
    /// the derived mapping wholesale), otherwise
    /// [`RestApi::legacy_env_mapping_for`] of `auth` and `env_auth`.
    #[must_use]
    pub fn default_env_mapping(&self) -> EnvMapping {
        self.env_mapping
            .clone()
            .unwrap_or_else(|| RestApi::legacy_env_mapping_for(&self.auth, &self.env_auth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Schema;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    // ========== EndpointRole Tests ==========

    #[test]
    fn endpoint_role_names_are_stable() {
        let names: Vec<String> = EndpointRole::iter()
            .map(|role| serde_json::to_string(&role).unwrap())
            .collect();
        assert_eq!(names, [r#""client""#, r#""host""#, r#""both""#]);
        for role in EndpointRole::iter() {
            assert_eq!(EndpointRole::from_str(&role.to_string()).unwrap(), role);
        }
    }

    #[test]
    fn shared_parameter_types_are_serializable() {
        fn assert_serde<T: Serialize + serde::de::DeserializeOwned>() {}
        assert_serde::<crate::ParamDef>();
        assert_serde::<crate::QueryParamType>();
        assert_serde::<crate::ParamStyle>();
        assert_serde::<EndpointParams>();
    }

    // ========== MessageDirection Tests ==========

    #[test]
    fn message_direction_display_lowercase() {
        assert_eq!(MessageDirection::Client.to_string(), "client");
        assert_eq!(MessageDirection::Server.to_string(), "server");
        assert_eq!(MessageDirection::Bidirectional.to_string(), "bidirectional");
    }

    #[test]
    fn message_direction_from_str_lowercase() {
        assert_eq!(
            MessageDirection::from_str("client").unwrap(),
            MessageDirection::Client
        );
        assert_eq!(
            MessageDirection::from_str("server").unwrap(),
            MessageDirection::Server
        );
        assert_eq!(
            MessageDirection::from_str("bidirectional").unwrap(),
            MessageDirection::Bidirectional
        );
    }

    #[test]
    fn message_direction_from_str_invalid() {
        assert!(MessageDirection::from_str("INVALID").is_err());
        assert!(MessageDirection::from_str("CLIENT").is_err()); // Case-sensitive
        assert!(MessageDirection::from_str("both").is_err());
    }

    #[test]
    fn message_direction_from_str_empty() {
        assert!(MessageDirection::from_str("").is_err());
    }

    #[test]
    fn message_direction_iter_all_variants() {
        let variants: Vec<_> = MessageDirection::iter().collect();
        assert_eq!(variants.len(), 3);
        assert!(variants.contains(&MessageDirection::Client));
        assert!(variants.contains(&MessageDirection::Server));
        assert!(variants.contains(&MessageDirection::Bidirectional));
    }

    #[test]
    fn message_direction_copy() {
        let original = MessageDirection::Bidirectional;
        let copied = original; // Copy, not move
        assert_eq!(original, copied);
    }

    #[test]
    fn message_direction_serde_roundtrip() {
        for direction in MessageDirection::iter() {
            let serialized = serde_json::to_string(&direction).unwrap();
            let deserialized: MessageDirection = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, direction);
        }
    }

    // ========== MessageSchema Tests ==========

    #[test]
    fn message_schema_serde_roundtrip() {
        let schema = MessageSchema::new(
            "AudioChunk",
            MessageDirection::Server,
            Schema::new("AudioChunkPayload"),
        )
        .describe("Audio data chunk")
        .matches("type", "audio")
        .matches("final", "false")
        .responds_with("Ack");

        let serialized = serde_json::to_string(&schema).unwrap();
        let deserialized: MessageSchema = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, schema);
        assert_eq!(serde_json::to_string(&deserialized).unwrap(), serialized);
    }

    #[test]
    fn message_schema_omits_undeclared_links_selectors_and_fallback() {
        let plain = MessageSchema::new("Data", MessageDirection::Server, Schema::new("Data"));
        let value = serde_json::to_value(&plain).unwrap();
        for absent in ["responds_with", "selectors", "fallback"] {
            assert!(value.get(absent).is_none(), "{absent}: {value}");
        }
        let fallback = serde_json::to_value(plain.fallback()).unwrap();
        assert_eq!(fallback["fallback"], serde_json::json!(true));
    }

    // ========== ConnectionLifecycle Tests ==========

    #[test]
    fn connection_lifecycle_default() {
        let lifecycle = ConnectionLifecycle::default();
        assert!(lifecycle.open.is_none());
        assert!(lifecycle.close.is_none());
        assert!(lifecycle.keepalive.is_none());
    }

    #[test]
    fn connection_lifecycle_serde_roundtrip() {
        let lifecycle = ConnectionLifecycle {
            open: Some(MessageSchema::new(
                "Init",
                MessageDirection::Client,
                Schema::new("InitMessage"),
            )),
            close: Some(
                MessageSchema::new("Close", MessageDirection::Client, Schema::new("CloseMessage"))
                    .describe("Graceful close"),
            ),
            keepalive: None,
        };

        let serialized = serde_json::to_string(&lifecycle).unwrap();
        let deserialized: ConnectionLifecycle = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, lifecycle);
    }

    // ========== WebSocketEndpoint Tests ==========

    #[test]
    fn websocket_endpoint_default() {
        let endpoint = WebSocketEndpoint::default();
        assert!(endpoint.id.is_empty());
        assert!(endpoint.path.is_empty());
        assert!(endpoint.description.is_empty());
        assert_eq!(endpoint.params, EndpointParams::default());
        assert_eq!(endpoint.role, EndpointRole::Client);
        assert!(endpoint.messages.is_empty());
    }

    #[test]
    fn websocket_endpoint_serde_roundtrip() {
        let endpoint = WebSocketEndpoint {
            id: "StreamAudio".to_string(),
            path: "/audio/{voice_id}".to_string(),
            description: "Audio streaming".to_string(),
            params: EndpointParams {
                path: vec![crate::ParamDef::path("voice_id", crate::QueryParamType::String)],
                ..EndpointParams::default().with_query_param(
                    "quality",
                    crate::QueryParamType::String,
                    false,
                    None,
                )
            },
            role: EndpointRole::Both,
            lifecycle: ConnectionLifecycle::default(),
            messages: vec![MessageSchema::new(
                "Data",
                MessageDirection::Server,
                Schema::new("DataPayload"),
            )],
            runtime: None,
        };

        let serialized = serde_json::to_string(&endpoint).unwrap();
        let deserialized: WebSocketEndpoint = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, endpoint);
    }

    // ========== WebSocketApi Tests ==========

    #[test]
    fn websocket_api_serde_roundtrip() {
        let api = WebSocketApi {
            name: "TestAPI".to_string(),
            description: "Test WebSocket API".to_string(),
            base_url: "wss://test.example.com".to_string(),
            docs_url: Some("https://docs.example.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: Some(AuthPolicy::from_auth_strategy(&AuthStrategy::BearerToken {
                header: None,
            })),
            env_auth: vec!["TEST_API_KEY".to_string()],
            env_mapping: Some(EnvMapping {
                bearer_token: Some(crate::EnvList::single("TEST_API_KEY")),
                ..EnvMapping::default()
            }),
            headers: vec![("X-Client".to_string(), "test".to_string())],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Echo".to_string(),
                path: "/echo".to_string(),
                description: "Echo messages".to_string(),
                params: EndpointParams::default(),
                role: EndpointRole::Host,
                lifecycle: ConnectionLifecycle::default(),
                messages: vec![MessageSchema::new(
                    "Message",
                    MessageDirection::Bidirectional,
                    Schema::new("EchoMessage"),
                )],
                runtime: None,
            }],
            runtime: None,
        };

        let serialized = serde_json::to_string(&api).unwrap();
        let deserialized: WebSocketApi = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, api);
    }

    #[test]
    fn websocket_api_with_auth_strategy() {
        let strategies = vec![
            AuthStrategy::None,
            AuthStrategy::BearerToken { header: None },
            AuthStrategy::BearerToken {
                header: Some("X-Auth".to_string()),
            },
            AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
                value_prefix: None,
            },
            AuthStrategy::Basic {
                username_env: crate::EnvList::single("WS_USER"),
                password_env: crate::EnvList::from_strs(&["WS_PASS", "WS_SECRET"]),
            },
        ];

        for auth in strategies {
            let api = WebSocketApi {
                name: "Test".to_string(),
                description: "Test".to_string(),
                base_url: "wss://test.com".to_string(),
                docs_url: None,
                auth: auth.clone(),
                auth_policy: None,
                env_auth: vec![],
                env_mapping: None,
                headers: vec![],
                version: None,
                endpoints: vec![],
                runtime: None,
            };

            let serialized = serde_json::to_string(&api).unwrap();
            let deserialized: WebSocketApi = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized.auth, auth);
        }
    }

    // ========== FrameFormat Tests ==========

    #[test]
    fn frame_format_default_is_json_text() {
        assert_eq!(FrameFormat::default(), FrameFormat::JsonText);
    }

    #[test]
    fn frame_format_display() {
        assert_eq!(FrameFormat::JsonText.to_string(), "json_text");
        assert_eq!(FrameFormat::JsonBinary.to_string(), "json_binary");
        assert_eq!(FrameFormat::RawBinary.to_string(), "raw_binary");
        assert_eq!(FrameFormat::Mixed.to_string(), "mixed");
    }

    #[test]
    fn frame_format_serde_roundtrip() {
        for fmt in FrameFormat::iter() {
            let serialized = serde_json::to_string(&fmt).unwrap();
            let deserialized: FrameFormat = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, fmt);
        }
    }

    // ========== RequestIdType Tests ==========

    #[test]
    fn request_id_type_default_is_u64() {
        assert_eq!(RequestIdType::default(), RequestIdType::U64);
    }

    #[test]
    fn request_id_type_serde_roundtrip() {
        for id_type in RequestIdType::iter() {
            let serialized = serde_json::to_string(&id_type).unwrap();
            let deserialized: RequestIdType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, id_type);
        }
    }

    // ========== CorrelationHints Tests ==========

    #[test]
    fn correlation_hints_serde_roundtrip() {
        let hints = CorrelationHints {
            request_id_field: "id".to_string(),
            response_id_field: "req_id".to_string(),
            default_timeout_secs: 60,
        };
        let serialized = serde_json::to_string(&hints).unwrap();
        let deserialized: CorrelationHints = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, hints);
    }

    #[test]
    fn correlation_hints_default_timeout() {
        let json = r#"{"request_id_field":"id","response_id_field":"id"}"#;
        let hints: CorrelationHints = serde_json::from_str(json).unwrap();
        assert_eq!(hints.default_timeout_secs, 30);
    }

    // ========== AuthFlowHints Tests ==========

    #[test]
    fn auth_flow_hints_serde_roundtrip() {
        let hints = AuthFlowHints {
            challenge_message: "auth_required".to_string(),
            auth_request_schema: "AuthMsg".to_string(),
            success_indicator: Some("authenticated".to_string()),
            failure_indicator: None,
        };
        let serialized = serde_json::to_string(&hints).unwrap();
        let deserialized: AuthFlowHints = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, hints);
    }

    #[test]
    fn auth_flow_hints_skips_none_fields() {
        let hints = AuthFlowHints {
            challenge_message: "auth".to_string(),
            auth_request_schema: "AuthMsg".to_string(),
            success_indicator: None,
            failure_indicator: None,
        };
        let serialized = serde_json::to_string(&hints).unwrap();
        assert!(!serialized.contains("success_indicator"));
        assert!(!serialized.contains("failure_indicator"));
    }

    // ========== HeartbeatHints Tests ==========

    #[test]
    fn heartbeat_hints_serde_roundtrip() {
        let hints = HeartbeatHints {
            interval_secs: 30,
            timeout_secs: Some(10),
        };
        let serialized = serde_json::to_string(&hints).unwrap();
        let deserialized: HeartbeatHints = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, hints);
    }

    // ========== WebSocketRuntimeHints Tests ==========

    #[test]
    fn runtime_hints_default_values() {
        let hints = WebSocketRuntimeHints::default();
        assert_eq!(hints.frame_format, FrameFormat::JsonText);
        assert!(hints.supports_reconnect);
        assert_eq!(hints.request_id_type, RequestIdType::U64);
    }

    #[test]
    fn runtime_hints_serde_roundtrip() {
        let hints = WebSocketRuntimeHints {
            frame_format: FrameFormat::Mixed,
            supports_reconnect: false,
            request_id_type: RequestIdType::String,
        };
        let serialized = serde_json::to_string(&hints).unwrap();
        let deserialized: WebSocketRuntimeHints = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, hints);
    }

    #[test]
    fn runtime_hints_deserializes_with_defaults() {
        let json = "{}";
        let hints: WebSocketRuntimeHints = serde_json::from_str(json).unwrap();
        assert_eq!(hints, WebSocketRuntimeHints::default());
    }

    // ========== WebSocketEndpointHints Tests ==========

    #[test]
    fn endpoint_hints_default_all_none() {
        let hints = WebSocketEndpointHints::default();
        assert!(hints.correlation.is_none());
        assert!(hints.auth_flow.is_none());
        assert!(hints.heartbeat.is_none());
    }

    #[test]
    fn endpoint_hints_serde_roundtrip() {
        let hints = WebSocketEndpointHints {
            correlation: Some(CorrelationHints {
                request_id_field: "id".to_string(),
                response_id_field: "id".to_string(),
                default_timeout_secs: 30,
            }),
            auth_flow: None,
            heartbeat: Some(HeartbeatHints {
                interval_secs: 15,
                timeout_secs: None,
            }),
        };
        let serialized = serde_json::to_string(&hints).unwrap();
        let deserialized: WebSocketEndpointHints = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, hints);
    }

    // ========== Backward Compatibility Tests ==========

    #[test]
    fn websocket_api_without_runtime_deserializes() {
        let json = r#"{
            "name": "Test",
            "description": "Test API",
            "base_url": "wss://test.com",
            "docs_url": null,
            "auth": "None",
            "env_auth": [],
            "headers": [],
            "endpoints": []
        }"#;
        let api: WebSocketApi = serde_json::from_str(json).unwrap();
        assert!(api.runtime.is_none());
    }

    #[test]
    fn websocket_endpoint_without_runtime_deserializes() {
        let json = r#"{
            "id": "Echo",
            "path": "/echo",
            "description": "Echo",
            "params": {"query": [], "header": [], "cookie": []},
            "lifecycle": {},
            "messages": []
        }"#;
        let endpoint: WebSocketEndpoint = serde_json::from_str(json).unwrap();
        assert!(endpoint.runtime.is_none());
        assert_eq!(endpoint.role, EndpointRole::Client);
    }

    #[test]
    fn websocket_api_with_runtime_roundtrip() {
        let api = WebSocketApi {
            name: "Test".to_string(),
            description: "Test".to_string(),
            base_url: "wss://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_mapping: None,
            headers: vec![],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Echo".to_string(),
                path: "/echo".to_string(),
                description: "Echo".to_string(),
                params: EndpointParams::default(),
                role: EndpointRole::Client,
                lifecycle: ConnectionLifecycle::default(),
                messages: vec![],
                runtime: Some(WebSocketEndpointHints {
                    correlation: Some(CorrelationHints {
                        request_id_field: "id".to_string(),
                        response_id_field: "id".to_string(),
                        default_timeout_secs: 30,
                    }),
                    auth_flow: None,
                    heartbeat: None,
                }),
            }],
            runtime: Some(WebSocketRuntimeHints::default()),
        };
        let serialized = serde_json::to_string(&api).unwrap();
        let deserialized: WebSocketApi = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, api);
    }

    // ========== Shared Vocabulary Tests ==========

    fn bare_api(auth: AuthStrategy, env_auth: &[&str]) -> WebSocketApi {
        WebSocketApi {
            name: "Test".to_string(),
            description: "Test".to_string(),
            base_url: "wss://test.com".to_string(),
            docs_url: None,
            auth,
            auth_policy: None,
            env_auth: env_auth.iter().map(|name| (*name).to_string()).collect(),
            env_mapping: None,
            headers: vec![],
            version: None,
            endpoints: vec![],
            runtime: None,
        }
    }

    #[test]
    fn derived_policy_and_env_mapping_follow_rest() {
        let auth = AuthStrategy::ApiKey {
            header: "xi-api-key".to_string(),
            value_prefix: None,
        };
        let api = bare_api(auth.clone(), &["FIRST", "SECOND"]);
        assert_eq!(api.effective_auth_policy(), AuthPolicy::from_auth_strategy(&auth));
        assert_eq!(
            api.default_env_mapping(),
            RestApi::legacy_env_mapping_for(&auth, &api.env_auth)
        );
        let key = api.default_env_mapping().api_key.expect("api key mapping");
        assert_eq!(key.names.names(), ["FIRST", "SECOND"]);
        assert_eq!(key.header, "xi-api-key");
    }

    #[test]
    fn explicit_policy_and_env_mapping_replace_the_derived_ones() {
        let mut api = bare_api(AuthStrategy::None, &["UCR_DOCK_TOKEN"]);
        assert_eq!(api.default_env_mapping(), EnvMapping::default());
        let mapping = EnvMapping {
            bearer_token: Some(crate::EnvList::single("UCR_DOCK_TOKEN")),
            ..EnvMapping::default()
        };
        let policy = AuthPolicy::from_auth_strategy(&AuthStrategy::BearerToken { header: None });
        api.env_mapping = Some(mapping.clone());
        api.auth_policy = Some(policy.clone());
        assert_eq!(api.default_env_mapping(), mapping);
        assert_eq!(api.effective_auth_policy(), policy);
    }

}
