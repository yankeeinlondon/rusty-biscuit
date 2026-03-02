//! Unfolded Circle Dock WebSocket model types.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Dock WebSocket envelope discriminator (`type` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockWsEnvelopeType {
    /// Request envelope (`type=req`).
    Req,
    /// Response envelope (`type=resp`).
    Resp,
    /// Event envelope (`type=event`).
    Event,
}

/// Known Dock message identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockWsKnownMessage {
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
pub enum DockWsMessageName {
    /// Known fixed message identifiers.
    Known(DockWsKnownMessage),
    /// Custom or future message identifiers.
    Other(String),
}

/// Dock request envelope (`type=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsRequestEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Client request id.
    pub id: u64,
    /// Command message id/name.
    pub msg: DockWsMessageName,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Dock response envelope (`type=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsResponseEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Correlation id.
    pub req_id: u64,
    /// Command message id/name.
    pub msg: DockWsMessageName,
    /// Response status code.
    pub code: i32,
    /// Optional reboot flag.
    pub reboot: Option<bool>,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Dock event envelope (`type=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsEventEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Event message id/name.
    pub msg: DockWsMessageName,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Dock auth message for post-connect authentication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DockWsAuthMessage {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Message id/name (`auth`).
    pub msg: DockWsKnownMessage,
    /// Auth token.
    pub token: String,
}

/// Dock auth-required event (`msg=auth_required`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DockWsAuthRequired {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Message id/name (`auth_required`).
    pub msg: DockWsKnownMessage,
}

/// Dock authentication result (`msg=authentication`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsAuthenticationMessage {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Message id/name (`authentication`).
    pub msg: DockWsKnownMessage,
    /// Optional auth status payload.
    pub msg_data: Option<Box<RawValue>>,
}
