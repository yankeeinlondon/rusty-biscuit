//! Unfolded Circle Integration WebSocket model types.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Integration WebSocket envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationWsEnvelopeKind {
    /// Request envelope (`kind=req`).
    Req,
    /// Response envelope (`kind=resp`).
    Resp,
    /// Event envelope (`kind=event`).
    Event,
}

/// Known Integration message identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationWsKnownMessage {
    /// Message-based authentication request.
    Auth,
    /// Server asks the peer to authenticate with a message.
    AuthRequired,
    /// Authentication result/confirmation.
    Authentication,
}

/// Message identifier with known variants and passthrough fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IntegrationWsMessageName {
    /// Known fixed message identifiers.
    Known(IntegrationWsKnownMessage),
    /// Custom or future message identifiers.
    Other(String),
}

/// Integration WS request envelope (`kind=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsRequestEnvelope {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Client-generated correlation id.
    pub id: u64,
    /// Message id/name.
    pub msg: IntegrationWsMessageName,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Integration WS response envelope (`kind=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsResponseEnvelope {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name.
    pub msg: IntegrationWsMessageName,
    /// Status code.
    pub code: i32,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Integration WS event envelope (`kind=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsEventEnvelope {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Event id/name.
    pub msg: IntegrationWsMessageName,
    /// Optional category.
    pub cat: Option<String>,
    /// Optional event timestamp.
    pub ts: Option<String>,
    /// Event payload (raw JSON). Deserialize by inspecting `msg`.
    pub msg_data: Box<RawValue>,
}

/// Integration auth message (`msg=auth`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsAuthMessage {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Message id/name (`auth`).
    pub msg: IntegrationWsKnownMessage,
    /// Authentication payload.
    pub msg_data: Box<RawValue>,
}

/// Integration auth-required event from peer (`msg=auth_required`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntegrationWsAuthRequired {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Message id/name (`auth_required`).
    pub msg: IntegrationWsKnownMessage,
}

/// Integration authentication result (`msg=authentication`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsAuthenticationMessage {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Message id/name (`authentication`).
    pub msg: IntegrationWsKnownMessage,
    /// Optional auth status payload.
    pub msg_data: Option<Box<RawValue>>,
}
