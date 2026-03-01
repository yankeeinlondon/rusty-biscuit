//! Unfolded Circle Core WebSocket model types.

use serde::{Deserialize, Serialize};

/// Generic Core WS request envelope (`kind=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsRequestEnvelope {
    /// Message kind discriminator.
    pub kind: String,
    /// Client-generated correlation id.
    pub id: u64,
    /// Message name.
    pub msg: String,
    /// Optional payload.
    pub msg_data: Option<serde_json::Value>,
}

/// Generic Core WS response envelope (`kind=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsResponseEnvelope {
    /// Message kind discriminator.
    pub kind: String,
    /// Correlation id matching request `id`.
    pub req_id: u64,
    /// Message name.
    pub msg: String,
    /// Response status code.
    pub code: i32,
    /// Optional payload.
    pub msg_data: Option<serde_json::Value>,
}

/// Generic Core WS event envelope (`kind=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsEventEnvelope {
    /// Message kind discriminator.
    pub kind: String,
    /// Event message id/name.
    pub msg: String,
    /// Optional category.
    pub cat: Option<String>,
    /// Optional event timestamp.
    pub ts: Option<String>,
    /// Event payload.
    pub msg_data: serde_json::Value,
}

/// Auth-required event emitted by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsAuthRequired {
    /// Message kind discriminator.
    pub kind: String,
    /// Event message id/name.
    pub msg: String,
}

/// Message-based authentication request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsAuthMessage {
    /// Message kind discriminator.
    pub kind: String,
    /// Message id/name (`auth`).
    pub msg: String,
    /// Authentication payload.
    pub msg_data: serde_json::Value,
}
