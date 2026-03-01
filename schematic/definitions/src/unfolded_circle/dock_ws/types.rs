//! Unfolded Circle Dock WebSocket model types.

use serde::{Deserialize, Serialize};

/// Dock request envelope (`type=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsRequestEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Client request id.
    pub id: u64,
    /// Command message id/name.
    pub msg: String,
    /// Optional payload.
    pub msg_data: Option<serde_json::Value>,
}

/// Dock response envelope (`type=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsResponseEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Correlation id.
    pub req_id: u64,
    /// Command message id/name.
    pub msg: String,
    /// Response status code.
    pub code: i32,
    /// Optional reboot flag.
    pub reboot: Option<bool>,
    /// Optional payload.
    pub msg_data: Option<serde_json::Value>,
}

/// Dock event envelope (`type=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsEventEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Event message id/name.
    pub msg: String,
    /// Optional payload.
    pub msg_data: Option<serde_json::Value>,
}

/// Dock auth message for post-connect authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsAuthMessage {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Message id/name (`auth`).
    pub msg: String,
    /// Auth token.
    pub token: String,
}
