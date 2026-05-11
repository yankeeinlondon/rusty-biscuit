use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

/// Frame format used by the WebSocket API.
///
/// Determines how outbound messages are framed and how inbound frames
/// are decoded by the generated runtime.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::FrameFormat;
/// use std::str::FromStr;
///
/// assert_eq!(FrameFormat::default(), FrameFormat::JsonText);
/// assert_eq!(FrameFormat::from_str("json_binary").unwrap(), FrameFormat::JsonBinary);
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
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FrameFormat {
    /// JSON payloads sent as text frames (most common).
    #[default]
    JsonText,
    /// JSON payloads sent as binary frames.
    JsonBinary,
    /// Raw binary payloads (no JSON envelope).
    RawBinary,
    /// Mixed: some messages use text, others use binary.
    Mixed,
}

/// Type used for request correlation IDs.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::RequestIdType;
/// use std::str::FromStr;
///
/// assert_eq!(RequestIdType::default(), RequestIdType::U64);
/// assert_eq!(RequestIdType::from_str("string").unwrap(), RequestIdType::String);
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
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RequestIdType {
    /// 64-bit unsigned integer IDs (monotonic).
    #[default]
    U64,
    /// String-based IDs.
    String,
}

/// Correlation hints for request-response matching over WebSocket.
///
/// When present, the generator produces `request()` methods that correlate
/// outbound requests with inbound responses using an ID field.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::CorrelationHints;
///
/// let hints = CorrelationHints {
///     request_id_field: "id".to_string(),
///     response_id_field: "id".to_string(),
///     default_timeout_secs: 30,
/// };
/// assert_eq!(hints.request_id_field, "id");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationHints {
    /// JSON field path for the request ID in outbound messages.
    pub request_id_field: String,
    /// JSON field path for the correlation ID in inbound responses.
    pub response_id_field: String,
    /// Default timeout in seconds for correlated requests.
    #[serde(default = "default_correlation_timeout")]
    pub default_timeout_secs: u64,
}

fn default_correlation_timeout() -> u64 {
    30
}

/// Auth flow hints for WebSocket APIs that use message-based authentication.
///
/// When present, the generator produces auth challenge/response handling
/// in the connection state machine.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::AuthFlowHints;
///
/// let hints = AuthFlowHints {
///     challenge_message: "auth_required".to_string(),
///     auth_request_schema: "AuthMessage".to_string(),
///     success_indicator: Some("authenticated".to_string()),
///     failure_indicator: Some("authentication_failed".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthFlowHints {
    /// Name of the server message that signals an auth challenge.
    pub challenge_message: String,
    /// Schema name for the auth request message the client must send.
    pub auth_request_schema: String,
    /// Optional message name or field value indicating auth success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_indicator: Option<String>,
    /// Optional message name or field value indicating auth failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_indicator: Option<String>,
}

/// Heartbeat configuration hints for WebSocket endpoints.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::HeartbeatHints;
///
/// let hints = HeartbeatHints {
///     interval_secs: 30,
///     timeout_secs: Some(10),
/// };
/// assert_eq!(hints.interval_secs, 30);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatHints {
    /// Interval in seconds between heartbeat messages.
    pub interval_secs: u64,
    /// Optional timeout in seconds to wait for heartbeat response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Runtime hints for a WebSocket API.
///
/// These hints guide the code generator to produce runtime-capable clients
/// with appropriate framing, reconnect, and ID strategies. All fields have
/// sensible defaults so existing definitions remain valid without changes.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{WebSocketRuntimeHints, FrameFormat, RequestIdType};
///
/// // All defaults
/// let hints = WebSocketRuntimeHints::default();
/// assert_eq!(hints.frame_format, FrameFormat::JsonText);
/// assert!(hints.supports_reconnect);
/// assert_eq!(hints.request_id_type, RequestIdType::U64);
///
/// // Custom configuration
/// let hints = WebSocketRuntimeHints {
///     frame_format: FrameFormat::Mixed,
///     supports_reconnect: false,
///     request_id_type: RequestIdType::String,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSocketRuntimeHints {
    /// Frame format for this API's messages.
    #[serde(default)]
    pub frame_format: FrameFormat,
    /// Whether the API supports client-initiated reconnect.
    #[serde(default = "default_true")]
    pub supports_reconnect: bool,
    /// Type used for request correlation IDs.
    #[serde(default)]
    pub request_id_type: RequestIdType,
}

impl Default for WebSocketRuntimeHints {
    fn default() -> Self {
        Self {
            frame_format: FrameFormat::default(),
            supports_reconnect: true,
            request_id_type: RequestIdType::default(),
        }
    }
}

/// Per-endpoint runtime hints for WebSocket endpoints.
///
/// These hints inform the generator about endpoint-specific behavior
/// like correlation, auth flow, and heartbeat requirements.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{WebSocketEndpointHints, CorrelationHints};
///
/// // Minimal: no special behavior
/// let hints = WebSocketEndpointHints::default();
/// assert!(hints.correlation.is_none());
/// assert!(hints.auth_flow.is_none());
/// assert!(hints.heartbeat.is_none());
///
/// // With correlation
/// let hints = WebSocketEndpointHints {
///     correlation: Some(CorrelationHints {
///         request_id_field: "id".to_string(),
///         response_id_field: "id".to_string(),
///         default_timeout_secs: 30,
///     }),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSocketEndpointHints {
    /// Correlation hints for request-response matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<CorrelationHints>,
    /// Auth flow hints for message-based authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_flow: Option<AuthFlowHints>,
    /// Heartbeat configuration hints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeat: Option<HeartbeatHints>,
}
