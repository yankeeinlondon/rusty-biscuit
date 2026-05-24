use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::PaginationMeta;

/// Connected client information.
///
/// ## Example
///
/// ```json
/// {
///   "clientid": "client123",
///   "username": "user1",
///   "node": "emqx@127.0.0.1",
///   "ip_address": "192.168.1.100",
///   "port": 54321,
///   "connected_at": "2024-01-01T12:00:00Z",
///   "keepalive": 60,
///   "clean_start": true,
///   "proto_ver": 5
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClientInfo {
    /// Client identifier.
    pub clientid: String,

    /// Client username.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Node the client is connected to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,

    /// Client IP address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    /// Client port.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// Connection timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_at: Option<String>,

    /// Keepalive interval in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keepalive: Option<u32>,

    /// Clean session/start flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clean_start: Option<bool>,

    /// MQTT protocol version (3, 4, or 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proto_ver: Option<u8>,

    /// Expiry interval for session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_interval: Option<u32>,

    /// Created at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Is client connected via bridge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_bridge: Option<bool>,

    /// Connection listener.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listener: Option<String>,

    /// Number of subscriptions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriptions_cnt: Option<u32>,

    /// Messages in queue (inflight + awaiting ack).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mqueue_len: Option<u32>,

    /// Messages dropped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mqueue_dropped: Option<u64>,
}

/// Paginated response for clients list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListClientsResponse {
    /// List of connected clients.
    pub data: Vec<ClientInfo>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

/// Request body for creating a subscription for a client.
///
/// ## Example
///
/// ```json
/// {
///   "topic": "sensors/#",
///   "qos": 1
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SubscribeBody {
    /// Topic filter to subscribe to.
    pub topic: String,

    /// QoS level (0, 1, or 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qos: Option<u8>,

    /// No local flag (MQTT 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nl: Option<bool>,

    /// Retain as published flag (MQTT 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rap: Option<bool>,

    /// Retain handling (MQTT 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rh: Option<u8>,
}
