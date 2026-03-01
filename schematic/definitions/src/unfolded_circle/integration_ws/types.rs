//! Unfolded Circle Integration WebSocket model types.

use serde::{Deserialize, Serialize};

/// Integration WS request envelope (`kind=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsRequestEnvelope {
    /// Envelope kind discriminator.
    pub kind: String,
    /// Client-generated correlation id.
    pub id: u64,
    /// Message id/name.
    pub msg: String,
    /// Optional payload.
    pub msg_data: Option<serde_json::Value>,
}

/// Integration WS response envelope (`kind=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsResponseEnvelope {
    /// Envelope kind discriminator.
    pub kind: String,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name.
    pub msg: String,
    /// Status code.
    pub code: i32,
    /// Optional payload.
    pub msg_data: Option<serde_json::Value>,
}

/// Integration WS event envelope (`kind=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsEventEnvelope {
    /// Envelope kind discriminator.
    pub kind: String,
    /// Event id/name.
    pub msg: String,
    /// Optional category.
    pub cat: Option<String>,
    /// Optional event timestamp.
    pub ts: Option<String>,
    /// Event payload.
    pub msg_data: serde_json::Value,
}

/// Integration auth message (`msg=auth`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsAuthMessage {
    /// Envelope kind discriminator.
    pub kind: String,
    /// Message id/name (`auth`).
    pub msg: String,
    /// Authentication payload.
    pub msg_data: serde_json::Value,
}
