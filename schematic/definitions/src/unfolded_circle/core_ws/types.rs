//! Unfolded Circle Core WebSocket model types.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Core WebSocket envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoreWsEnvelopeKind {
    /// Request envelope (`kind=req`).
    Req,
    /// Response envelope (`kind=resp`).
    Resp,
    /// Event envelope (`kind=event`).
    Event,
}

/// Known Core message identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreWsKnownMessage {
    /// Message-based authentication request.
    Auth,
    /// Server asks the client to authenticate with a message.
    AuthRequired,
    /// Authentication result/confirmation.
    Authentication,
    /// Ping keepalive message.
    Ping,
    /// Pong keepalive response.
    Pong,
    /// Generic command result message.
    Result,
    /// Version request message.
    Version,
    /// Version information response.
    VersionInfo,
    /// System information request message.
    System,
    /// System information response message.
    SystemInfo,
}

/// Message identifier with known variants and passthrough fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoreWsMessageName {
    /// Known fixed message identifiers.
    Known(CoreWsKnownMessage),
    /// Custom or future message identifiers.
    Other(String),
}

/// Generic Core WS request envelope (`kind=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsRequestEnvelope {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client-generated correlation id.
    pub id: u64,
    /// Message name.
    pub msg: CoreWsMessageName,
    /// Optional raw payload.
    ///
    /// The payload is intentionally deferred as [`RawValue`] to avoid eagerly
    /// allocating a full `serde_json::Value` tree before routing.
    ///
    /// Deserialize on demand:
    ///
    /// ```rust
    /// use serde::Deserialize;
    /// use serde_json::value::RawValue;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct PingPayload {
    ///     id: u64,
    /// }
    ///
    /// fn decode(raw: &RawValue) -> serde_json::Result<PingPayload> {
    ///     serde_json::from_str(raw.get())
    /// }
    /// ```
    pub msg_data: Option<Box<RawValue>>,
}

/// Generic Core WS response envelope (`kind=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsResponseEnvelope {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlation id matching request `id`.
    pub req_id: u64,
    /// Message name.
    pub msg: CoreWsMessageName,
    /// Response status code.
    pub code: i32,
    /// Optional raw payload. Deserialize this after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Generic Core WS event envelope (`kind=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsEventEnvelope {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Event message id/name.
    pub msg: CoreWsMessageName,
    /// Optional category.
    pub cat: Option<String>,
    /// Optional event timestamp.
    pub ts: Option<String>,
    /// Event payload (raw JSON). Deserialize by inspecting `msg`.
    pub msg_data: Box<RawValue>,
}

/// Auth-required event emitted by the server.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CoreWsAuthRequired {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Event message id/name (`auth_required`).
    pub msg: CoreWsKnownMessage,
}

/// Message-based authentication request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsAuthMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Message id/name (`auth`).
    pub msg: CoreWsKnownMessage,
    /// Authentication payload.
    pub msg_data: Box<RawValue>,
}

/// Message-based authentication result from the server (`msg=authentication`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsAuthenticationMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Message id/name (`authentication`).
    pub msg: CoreWsKnownMessage,
    /// Optional server payload for auth status/details.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed ping request (`msg=ping`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsPingMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client correlation id.
    pub id: u64,
    /// Message id/name (`ping`).
    pub msg: CoreWsKnownMessage,
    /// Optional ping payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed pong response (`msg=pong`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsPongMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`pong`).
    pub msg: CoreWsKnownMessage,
    /// Optional pong payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed version request (`msg=version`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsVersionMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client correlation id.
    pub id: u64,
    /// Message id/name (`version`).
    pub msg: CoreWsKnownMessage,
    /// Optional version request payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed version info response (`msg=version_info`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsVersionInfoMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`version_info`).
    pub msg: CoreWsKnownMessage,
    /// Optional version payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed system-info request (`msg=system`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsSystemMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client correlation id.
    pub id: u64,
    /// Message id/name (`system`).
    pub msg: CoreWsKnownMessage,
    /// Optional system request payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed system-info response (`msg=system_info`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsSystemInfoMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`system_info`).
    pub msg: CoreWsKnownMessage,
    /// Optional system payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed generic result response (`msg=result`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsResultMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`result`).
    pub msg: CoreWsKnownMessage,
    /// Optional result payload.
    pub msg_data: Option<Box<RawValue>>,
}
