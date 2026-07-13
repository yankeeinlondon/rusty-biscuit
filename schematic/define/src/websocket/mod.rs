//! WebSocket API definitions.
//!
//! This module provides types for defining WebSocket APIs in a declarative way,
//! parallel to the REST API types in [`crate::types`]. These definitions can be
//! used to generate strongly-typed WebSocket client code.
//!
//! ## Core Types
//!
//! - [`WebSocketApi`] - Complete WebSocket API definition with base URL, auth, and endpoints
//! - [`WebSocketEndpoint`] - Single WebSocket endpoint with path, parameters, and message schemas
//! - [`ConnectionParam`] - Query/path parameter definition for WebSocket connections
//! - [`ParamType`] - Parameter types (String, Integer, Boolean, Float)
//! - [`ConnectionLifecycle`] - Open, close, and keepalive message schemas
//! - [`MessageSchema`] - Single message type with direction and schema
//! - [`MessageDirection`] - Message flow direction (Client, Server, Bidirectional)
//!
//! ## Examples
//!
//! Define a WebSocket API like ElevenLabs Text-to-Speech:
//!
//! ```
//! use schematic_define::websocket::*;
//! use schematic_define::{AuthStrategy, Schema};
//!
//! let api = WebSocketApi {
//!     name: "ElevenLabsTTS".to_string(),
//!     description: "ElevenLabs Text-to-Speech WebSocket API".to_string(),
//!     base_url: "wss://api.elevenlabs.io/v1".to_string(),
//!     docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
//!     auth: AuthStrategy::ApiKey { header: "xi-api-key".to_string(), value_prefix: None },
//!     env_auth: vec!["ELEVEN_LABS_API_KEY".to_string()],
//!     version: None,
//!     endpoints: vec![
//!         WebSocketEndpoint {
//!             id: "TextToSpeech".to_string(),
//!             path: "/text-to-speech/{voice_id}/stream-input".to_string(),
//!             description: "Stream text and receive audio chunks".to_string(),
//!             connection_params: vec![
//!                 ConnectionParam {
//!                     name: "model_id".to_string(),
//!                     param_type: ParamType::String,
//!                     required: false,
//!                     description: Some("Model to use for synthesis".to_string()),
//!                 },
//!                 ConnectionParam {
//!                     name: "output_format".to_string(),
//!                     param_type: ParamType::String,
//!                     required: false,
//!                     description: Some("Audio output format".to_string()),
//!                 },
//!             ],
//!             lifecycle: ConnectionLifecycle {
//!                 open: Some(MessageSchema {
//!                     name: "BOS".to_string(),
//!                     direction: MessageDirection::Client,
//!                     schema: Schema::new("BeginOfStreamMessage"),
//!                     description: Some("Begin-of-stream message with config".to_string()),
//!                 }),
//!                 close: Some(MessageSchema {
//!                     name: "EOS".to_string(),
//!                     direction: MessageDirection::Client,
//!                     schema: Schema::new("EndOfStreamMessage"),
//!                     description: Some("End-of-stream signal".to_string()),
//!                 }),
//!                 keepalive: None,
//!             },
//!             messages: vec![
//!                 MessageSchema {
//!                     name: "TextChunk".to_string(),
//!                     direction: MessageDirection::Client,
//!                     schema: Schema::new("TextChunkMessage"),
//!                     description: Some("Text to synthesize".to_string()),
//!                 },
//!                 MessageSchema {
//!                     name: "AudioChunk".to_string(),
//!                     direction: MessageDirection::Server,
//!                     schema: Schema::new("AudioChunkResponse"),
//!                     description: Some("Audio data chunk".to_string()),
//!                 },
//!             ],
//!             runtime: None,
//!         },
//!     ],
//!     runtime: None,
//! };
//!
//! assert_eq!(api.name, "ElevenLabsTTS");
//! assert_eq!(api.endpoints.len(), 1);
//! ```

mod hints;
mod lifecycle;
mod params;

pub use hints::{
    AuthFlowHints, CorrelationHints, FrameFormat, HeartbeatHints, RequestIdType,
    WebSocketEndpointHints, WebSocketRuntimeHints,
};
pub use lifecycle::{ConnectionLifecycle, MessageDirection, MessageSchema};
pub use params::{ConnectionParam, ParamType};

use serde::{Deserialize, Serialize};

use crate::auth::AuthStrategy;

/// A single WebSocket endpoint definition.
///
/// Defines how to connect to a WebSocket endpoint, including the path,
/// connection parameters, and message schemas.
///
/// ## Path Parameters
///
/// Paths support template parameters using curly braces: `/stream/{channel_id}`.
/// These become required fields when establishing the connection.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{
///     WebSocketEndpoint, ConnectionParam, ParamType, ConnectionLifecycle, MessageSchema, MessageDirection
/// };
/// use schematic_define::Schema;
///
/// let endpoint = WebSocketEndpoint {
///     id: "StreamAudio".to_string(),
///     path: "/audio/{voice_id}/stream".to_string(),
///     description: "Stream audio synthesis".to_string(),
///     connection_params: vec![
///         ConnectionParam {
///             name: "quality".to_string(),
///             param_type: ParamType::String,
///             required: false,
///             description: Some("Audio quality setting".to_string()),
///         },
///     ],
///     lifecycle: ConnectionLifecycle::default(),
///     messages: vec![
///         MessageSchema {
///             name: "AudioData".to_string(),
///             direction: MessageDirection::Server,
///             schema: Schema::new("AudioDataMessage"),
///             description: Some("Audio data chunk".to_string()),
///         },
///     ],
///     runtime: None,
/// };
///
/// assert_eq!(endpoint.id, "StreamAudio");
/// assert!(endpoint.path.contains("{voice_id}"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSocketEndpoint {
    /// Identifier for this endpoint (becomes the struct/method name).
    ///
    /// Should be PascalCase (e.g., "TextToSpeech", "StreamAudio").
    pub id: String,
    /// Path template (e.g., "/stream/{channel_id}").
    ///
    /// Parameters in curly braces become required connection parameters.
    pub path: String,
    /// Human-readable description of what this endpoint does.
    pub description: String,
    /// Query/path parameters for establishing the connection.
    pub connection_params: Vec<ConnectionParam>,
    /// Lifecycle messages (open, close, keepalive).
    pub lifecycle: ConnectionLifecycle,
    /// Regular messages that can be sent/received.
    pub messages: Vec<MessageSchema>,
    /// Optional runtime hints for code generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<WebSocketEndpointHints>,
}

/// A complete WebSocket API definition.
///
/// This struct captures all the information needed to generate a typed client
/// for a WebSocket API, including the base URL, authentication strategy, and
/// all endpoint definitions.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{WebSocketApi, WebSocketEndpoint};
/// use schematic_define::AuthStrategy;
///
/// let api = WebSocketApi {
///     name: "MyStreamingAPI".to_string(),
///     description: "Real-time streaming API".to_string(),
///     base_url: "wss://stream.example.com/v1".to_string(),
///     docs_url: Some("https://docs.example.com/websocket".to_string()),
///     auth: AuthStrategy::BearerToken { header: None },
///     env_auth: vec!["STREAM_API_KEY".to_string()],
///     version: None,
///     endpoints: vec![],
///     runtime: None,
/// };
///
/// assert_eq!(api.name, "MyStreamingAPI");
/// assert!(api.base_url.starts_with("wss://"));
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
    /// Endpoint paths are appended to this URL when connecting.
    pub base_url: String,
    /// Link to API documentation (optional).
    pub docs_url: Option<String>,
    /// Authentication strategy for this API.
    ///
    /// Reuses [`AuthStrategy`] from the REST API types.
    pub auth: AuthStrategy,
    /// Environment variable names for authentication credentials.
    ///
    /// Works the same as [`crate::RestApi::env_auth`]: a fallback chain
    /// where the first set env var is used.
    pub env_auth: Vec<String>,
    /// Semantic version of this API definition (e.g., `"1.0.0"`).
    ///
    /// Used as the `info.version` field in exported API documents.
    /// If `None`, falls back to a default of `"1.0.0"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// All endpoints defined for this API.
    pub endpoints: Vec<WebSocketEndpoint>,
    /// Optional runtime hints for code generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<WebSocketRuntimeHints>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Schema;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    // ========== ParamType Tests ==========

    #[test]
    fn param_type_display_lowercase() {
        assert_eq!(ParamType::String.to_string(), "string");
        assert_eq!(ParamType::Integer.to_string(), "integer");
        assert_eq!(ParamType::Boolean.to_string(), "boolean");
        assert_eq!(ParamType::Float.to_string(), "float");
    }

    #[test]
    fn param_type_from_str_lowercase() {
        assert_eq!(ParamType::from_str("string").unwrap(), ParamType::String);
        assert_eq!(ParamType::from_str("integer").unwrap(), ParamType::Integer);
        assert_eq!(ParamType::from_str("boolean").unwrap(), ParamType::Boolean);
        assert_eq!(ParamType::from_str("float").unwrap(), ParamType::Float);
    }

    #[test]
    fn param_type_from_str_invalid() {
        assert!(ParamType::from_str("INVALID").is_err());
        assert!(ParamType::from_str("STRING").is_err()); // Case-sensitive
        assert!(ParamType::from_str("int").is_err());
    }

    #[test]
    fn param_type_from_str_empty() {
        assert!(ParamType::from_str("").is_err());
    }

    #[test]
    fn param_type_iter_all_variants() {
        let variants: Vec<_> = ParamType::iter().collect();
        assert_eq!(variants.len(), 4);
        assert!(variants.contains(&ParamType::String));
        assert!(variants.contains(&ParamType::Integer));
        assert!(variants.contains(&ParamType::Boolean));
        assert!(variants.contains(&ParamType::Float));
    }

    #[test]
    fn param_type_copy() {
        let original = ParamType::String;
        let copied = original; // Copy, not move
        assert_eq!(original, copied);
    }

    #[test]
    fn param_type_serde_roundtrip() {
        for param_type in ParamType::iter() {
            let serialized = serde_json::to_string(&param_type).unwrap();
            let deserialized: ParamType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, param_type);
        }
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

    // ========== ConnectionParam Tests ==========

    #[test]
    fn connection_param_serde_roundtrip() {
        let param = ConnectionParam {
            name: "model_id".to_string(),
            param_type: ParamType::String,
            required: true,
            description: Some("The model identifier".to_string()),
        };

        let serialized = serde_json::to_string(&param).unwrap();
        let deserialized: ConnectionParam = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, param);
    }

    // ========== MessageSchema Tests ==========

    #[test]
    fn message_schema_serde_roundtrip() {
        let schema = MessageSchema {
            name: "AudioChunk".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("AudioChunkPayload"),
            description: Some("Audio data chunk".to_string()),
        };

        let serialized = serde_json::to_string(&schema).unwrap();
        let deserialized: MessageSchema = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, schema);
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
            open: Some(MessageSchema {
                name: "Init".to_string(),
                direction: MessageDirection::Client,
                schema: Schema::new("InitMessage"),
                description: None,
            }),
            close: Some(MessageSchema {
                name: "Close".to_string(),
                direction: MessageDirection::Client,
                schema: Schema::new("CloseMessage"),
                description: Some("Graceful close".to_string()),
            }),
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
        assert!(endpoint.connection_params.is_empty());
        assert!(endpoint.messages.is_empty());
    }

    #[test]
    fn websocket_endpoint_serde_roundtrip() {
        let endpoint = WebSocketEndpoint {
            id: "StreamAudio".to_string(),
            path: "/audio/{voice_id}".to_string(),
            description: "Audio streaming".to_string(),
            connection_params: vec![ConnectionParam {
                name: "quality".to_string(),
                param_type: ParamType::String,
                required: false,
                description: None,
            }],
            lifecycle: ConnectionLifecycle::default(),
            messages: vec![MessageSchema {
                name: "Data".to_string(),
                direction: MessageDirection::Server,
                schema: Schema::new("DataPayload"),
                description: None,
            }],
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
            env_auth: vec!["TEST_API_KEY".to_string()],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Echo".to_string(),
                path: "/echo".to_string(),
                description: "Echo messages".to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: vec![MessageSchema {
                    name: "Message".to_string(),
                    direction: MessageDirection::Bidirectional,
                    schema: Schema::new("EchoMessage"),
                    description: None,
                }],
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
            AuthStrategy::Basic,
        ];

        for auth in strategies {
            let api = WebSocketApi {
                name: "Test".to_string(),
                description: "Test".to_string(),
                base_url: "wss://test.com".to_string(),
                docs_url: None,
                auth: auth.clone(),
                env_auth: vec![],
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
            "connection_params": [],
            "lifecycle": {},
            "messages": []
        }"#;
        let endpoint: WebSocketEndpoint = serde_json::from_str(json).unwrap();
        assert!(endpoint.runtime.is_none());
    }

    #[test]
    fn websocket_api_with_runtime_roundtrip() {
        let api = WebSocketApi {
            name: "Test".to_string(),
            description: "Test".to_string(),
            base_url: "wss://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Echo".to_string(),
                path: "/echo".to_string(),
                description: "Echo".to_string(),
                connection_params: vec![],
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

    // ========== Integration Test ==========

    #[test]
    fn elevenlabs_tts_websocket_example() {
        let api = WebSocketApi {
            name: "ElevenLabsTTS".to_string(),
            description: "ElevenLabs Text-to-Speech WebSocket API".to_string(),
            base_url: "wss://api.elevenlabs.io/v1".to_string(),
            docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
            auth: AuthStrategy::ApiKey {
                header: "xi-api-key".to_string(),
                value_prefix: None,
            },
            env_auth: vec![
                "ELEVEN_LABS_API_KEY".to_string(),
                "ELEVENLABS_API_KEY".to_string(),
            ],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "TextToSpeech".to_string(),
                path: "/text-to-speech/{voice_id}/stream-input".to_string(),
                description: "Stream text and receive audio chunks in real-time".to_string(),
                connection_params: vec![
                    ConnectionParam {
                        name: "model_id".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("Model to use (e.g., eleven_turbo_v2)".to_string()),
                    },
                    ConnectionParam {
                        name: "output_format".to_string(),
                        param_type: ParamType::String,
                        required: false,
                        description: Some("Audio format (mp3_44100, pcm_16000, etc.)".to_string()),
                    },
                    ConnectionParam {
                        name: "optimize_streaming_latency".to_string(),
                        param_type: ParamType::Integer,
                        required: false,
                        description: Some("Latency optimization (0-4)".to_string()),
                    },
                ],
                lifecycle: ConnectionLifecycle {
                    open: Some(MessageSchema {
                        name: "BOS".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("BeginOfStreamMessage"),
                        description: Some(
                            "Begin-of-stream with voice settings and generation config".to_string(),
                        ),
                    }),
                    close: Some(MessageSchema {
                        name: "EOS".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("EndOfStreamMessage"),
                        description: Some(
                            "End-of-stream signal to flush remaining audio".to_string(),
                        ),
                    }),
                    keepalive: None,
                },
                messages: vec![
                    MessageSchema {
                        name: "TextChunk".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("TextChunkMessage"),
                        description: Some("Text chunk to synthesize".to_string()),
                    },
                    MessageSchema {
                        name: "AudioChunk".to_string(),
                        direction: MessageDirection::Server,
                        schema: Schema::new("AudioChunkResponse"),
                        description: Some("Base64-encoded audio data".to_string()),
                    },
                    MessageSchema {
                        name: "AlignmentInfo".to_string(),
                        direction: MessageDirection::Server,
                        schema: Schema::new("AlignmentResponse"),
                        description: Some("Word-level timing alignment".to_string()),
                    },
                ],
                runtime: None,
            }],
            runtime: None,
        };

        assert_eq!(api.name, "ElevenLabsTTS");
        assert_eq!(api.endpoints.len(), 1);

        let endpoint = &api.endpoints[0];
        assert_eq!(endpoint.id, "TextToSpeech");
        assert_eq!(endpoint.connection_params.len(), 3);
        assert!(endpoint.lifecycle.open.is_some());
        assert!(endpoint.lifecycle.close.is_some());
        assert_eq!(endpoint.messages.len(), 3);

        let serialized = serde_json::to_string_pretty(&api).unwrap();
        let deserialized: WebSocketApi = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, api);
    }
}
