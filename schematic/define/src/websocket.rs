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
//!     auth: AuthStrategy::ApiKey { header: "xi-api-key".to_string() },
//!     env_auth: vec!["ELEVEN_LABS_API_KEY".to_string()],
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
//!         },
//!     ],
//! };
//!
//! assert_eq!(api.name, "ElevenLabsTTS");
//! assert_eq!(api.endpoints.len(), 1);
//! ```

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::auth::AuthStrategy;
use crate::schema::Schema;

/// Parameter types for WebSocket connection parameters.
///
/// These types map to common JSON/query string value types.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::ParamType;
/// use std::str::FromStr;
///
/// // Display as lowercase
/// assert_eq!(ParamType::String.to_string(), "string");
/// assert_eq!(ParamType::Integer.to_string(), "integer");
///
/// // Parse from lowercase
/// assert_eq!(ParamType::from_str("boolean").unwrap(), ParamType::Boolean);
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumIter, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ParamType {
    /// String parameter
    String,
    /// Integer parameter (signed 64-bit)
    Integer,
    /// Boolean parameter
    Boolean,
    /// Floating-point parameter (64-bit)
    Float,
}

/// Direction of message flow in a WebSocket connection.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::MessageDirection;
/// use std::str::FromStr;
///
/// // Display as lowercase
/// assert_eq!(MessageDirection::Client.to_string(), "client");
/// assert_eq!(MessageDirection::Server.to_string(), "server");
/// assert_eq!(MessageDirection::Bidirectional.to_string(), "bidirectional");
///
/// // Parse from lowercase
/// assert_eq!(
///     MessageDirection::from_str("bidirectional").unwrap(),
///     MessageDirection::Bidirectional
/// );
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumIter, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum MessageDirection {
    /// Message sent from client to server
    Client,
    /// Message sent from server to client
    Server,
    /// Message can flow in either direction
    Bidirectional,
}

/// A connection parameter for WebSocket endpoints.
///
/// Connection parameters are typically passed as query string parameters
/// when establishing the WebSocket connection.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{ConnectionParam, ParamType};
///
/// let param = ConnectionParam {
///     name: "model_id".to_string(),
///     param_type: ParamType::String,
///     required: false,
///     description: Some("The model to use".to_string()),
/// };
///
/// assert_eq!(param.name, "model_id");
/// assert!(!param.required);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionParam {
    /// Parameter name (used in query string or path).
    pub name: String,
    /// Type of the parameter value.
    pub param_type: ParamType,
    /// Whether the parameter is required for connection.
    pub required: bool,
    /// Human-readable description of the parameter.
    pub description: Option<String>,
}

/// A message schema definition for WebSocket communication.
///
/// Defines a single message type that can be sent or received
/// over the WebSocket connection.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{MessageSchema, MessageDirection};
/// use schematic_define::Schema;
///
/// let message = MessageSchema {
///     name: "TextChunk".to_string(),
///     direction: MessageDirection::Client,
///     schema: Schema::new("TextChunkMessage"),
///     description: Some("A chunk of text to process".to_string()),
/// };
///
/// assert_eq!(message.name, "TextChunk");
/// assert_eq!(message.direction, MessageDirection::Client);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSchema {
    /// Name of this message type (used for generated enum variants).
    pub name: String,
    /// Direction this message flows.
    pub direction: MessageDirection,
    /// Schema for the message payload.
    pub schema: Schema,
    /// Human-readable description of the message.
    pub description: Option<String>,
}

/// Lifecycle messages for WebSocket connection management.
///
/// Defines optional messages for connection open, close, and keepalive.
/// These are separate from regular messages as they have special semantics.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{ConnectionLifecycle, MessageSchema, MessageDirection};
/// use schematic_define::Schema;
///
/// // Empty lifecycle (no special messages)
/// let empty = ConnectionLifecycle::default();
/// assert!(empty.open.is_none());
/// assert!(empty.close.is_none());
/// assert!(empty.keepalive.is_none());
///
/// // With open/close messages
/// let lifecycle = ConnectionLifecycle {
///     open: Some(MessageSchema {
///         name: "Init".to_string(),
///         direction: MessageDirection::Client,
///         schema: Schema::new("InitMessage"),
///         description: Some("Initialization message".to_string()),
///     }),
///     close: None,
///     keepalive: None,
/// };
/// assert!(lifecycle.open.is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionLifecycle {
    /// Message to send when connection is established.
    pub open: Option<MessageSchema>,
    /// Message to send before closing the connection.
    pub close: Option<MessageSchema>,
    /// Message for keepalive/heartbeat (if required by the API).
    pub keepalive: Option<MessageSchema>,
}

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
///     endpoints: vec![],
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
    /// All endpoints defined for this API.
    pub endpoints: Vec<WebSocketEndpoint>,
}

#[cfg(test)]
mod tests {
    use super::*;
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
            }],
        };

        let serialized = serde_json::to_string(&api).unwrap();
        let deserialized: WebSocketApi = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, api);
    }

    #[test]
    fn websocket_api_with_auth_strategy() {
        // Test that WebSocketApi works with all AuthStrategy variants
        let strategies = vec![
            AuthStrategy::None,
            AuthStrategy::BearerToken { header: None },
            AuthStrategy::BearerToken {
                header: Some("X-Auth".to_string()),
            },
            AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
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
                endpoints: vec![],
            };

            // Verify it serializes and deserializes correctly
            let serialized = serde_json::to_string(&api).unwrap();
            let deserialized: WebSocketApi = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized.auth, auth);
        }
    }

    // ========== Integration Test ==========

    #[test]
    fn elevenlabs_tts_websocket_example() {
        // Full example matching the ElevenLabs TTS WebSocket API structure
        let api = WebSocketApi {
            name: "ElevenLabsTTS".to_string(),
            description: "ElevenLabs Text-to-Speech WebSocket API".to_string(),
            base_url: "wss://api.elevenlabs.io/v1".to_string(),
            docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
            auth: AuthStrategy::ApiKey {
                header: "xi-api-key".to_string(),
            },
            env_auth: vec![
                "ELEVEN_LABS_API_KEY".to_string(),
                "ELEVENLABS_API_KEY".to_string(),
            ],
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
                        description: Some("End-of-stream signal to flush remaining audio".to_string()),
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
            }],
        };

        // Verify structure
        assert_eq!(api.name, "ElevenLabsTTS");
        assert_eq!(api.endpoints.len(), 1);

        let endpoint = &api.endpoints[0];
        assert_eq!(endpoint.id, "TextToSpeech");
        assert_eq!(endpoint.connection_params.len(), 3);
        assert!(endpoint.lifecycle.open.is_some());
        assert!(endpoint.lifecycle.close.is_some());
        assert_eq!(endpoint.messages.len(), 3);

        // Verify serde roundtrip
        let serialized = serde_json::to_string_pretty(&api).unwrap();
        let deserialized: WebSocketApi = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, api);
    }
}
