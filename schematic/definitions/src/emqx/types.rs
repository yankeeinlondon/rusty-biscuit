//! EMQX REST API types.
//!
//! This module contains request and response types for the EMQX Broker REST API
//! supporting both Basic Auth (API Key) and Bearer Token authentication.

use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

/// Pagination metadata in EMQX list responses.
///
/// ## Example
///
/// ```json
/// {
///   "count": 100,
///   "limit": 100,
///   "page": 1,
///   "hasnext": true
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationMeta {
    /// Total count of items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,

    /// Maximum items per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Current page number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,

    /// Whether more pages exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hasnext: Option<bool>,
}

/// Standard error response from EMQX API.
///
/// ## Example
///
/// ```json
/// {
///   "code": "RESOURCE_NOT_FOUND",
///   "reason": "Client id not found"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code (e.g., "RESOURCE_NOT_FOUND", "BAD_REQUEST").
    pub code: String,

    /// Human-readable error description.
    pub reason: String,
}

// =============================================================================
// Authentication Types (Bearer Token)
// =============================================================================

/// Request body for the `/login` endpoint.
///
/// ## Example
///
/// ```json
/// {
///   "username": "admin",
///   "password": "public"
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginBody {
    /// Dashboard username.
    pub username: String,

    /// Dashboard password.
    pub password: String,
}

/// Response from the `/login` endpoint.
///
/// ## Example
///
/// ```json
/// {
///   "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
///   "license": {"edition": "enterprise", ...}
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginResponse {
    /// JWT token for subsequent requests.
    pub token: String,

    /// License information (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<serde_json::Value>,
}

// =============================================================================
// Node & Cluster Types
// =============================================================================

/// Information about a single EMQX node.
///
/// ## Example
///
/// ```json
/// {
///   "node": "emqx@127.0.0.1",
///   "version": "5.0.0",
///   "uptime": 86400000,
///   "status": "running",
///   "memory_total": 1073741824,
///   "memory_used": 536870912,
///   "connections": 1000
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node name (e.g., "emqx@127.0.0.1").
    pub node: String,

    /// EMQX version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Node uptime in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<u64>,

    /// Node status (running, stopped, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Total memory in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_total: Option<u64>,

    /// Used memory in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_used: Option<u64>,

    /// Number of connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<u64>,

    /// Maximum file descriptors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fds: Option<u64>,

    /// Load averages [1min, 5min, 15min].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load: Option<Vec<f64>>,
}

/// Response from `/nodes` endpoint - list of cluster nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListNodesResponse {
    /// List of nodes in the cluster.
    pub data: Vec<NodeInfo>,
}

/// Cluster status information.
///
/// ## Example
///
/// ```json
/// {
///   "running": ["emqx@node1", "emqx@node2"],
///   "stopped": []
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterStatus {
    /// Running node names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<Vec<String>>,

    /// Stopped node names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<Vec<String>>,
}

// =============================================================================
// Client Types
// =============================================================================

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

// =============================================================================
// Subscription Types
// =============================================================================

/// Subscription information.
///
/// ## Example
///
/// ```json
/// {
///   "node": "emqx@127.0.0.1",
///   "topic": "sensors/temperature",
///   "clientid": "client123",
///   "qos": 1
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    /// Node where subscription exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,

    /// Subscribed topic.
    pub topic: String,

    /// Client ID that owns the subscription.
    pub clientid: String,

    /// QoS level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qos: Option<u8>,

    /// No local flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nl: Option<bool>,

    /// Retain as published flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rap: Option<bool>,

    /// Retain handling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rh: Option<u8>,
}

/// Paginated response for subscriptions list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSubscriptionsResponse {
    /// List of subscriptions.
    pub data: Vec<SubscriptionInfo>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

// =============================================================================
// Publishing Types
// =============================================================================

/// Request body for publishing a message.
///
/// ## Example
///
/// ```json
/// {
///   "topic": "sensors/temp",
///   "payload": "{\"temperature\": 25.5}",
///   "qos": 1,
///   "retain": false
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PublishBody {
    /// Topic to publish to.
    pub topic: String,

    /// Message payload.
    pub payload: String,

    /// QoS level (0, 1, or 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qos: Option<u8>,

    /// Retain flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retain: Option<bool>,

    /// Message encoding (plain, base64).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,

    /// Payload format indicator (MQTT 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_format_indicator: Option<u8>,

    /// Message expiry interval in seconds (MQTT 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_expiry_interval: Option<u32>,

    /// User properties (MQTT 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_properties: Option<serde_json::Value>,
}

/// Request body for batch publishing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PublishBatchBody {
    /// List of messages to publish.
    pub messages: Vec<PublishBody>,
}

// =============================================================================
// Rules Engine Types
// =============================================================================

/// Rule action configuration.
///
/// ## Example
///
/// ```json
/// {
///   "function": "republish",
///   "args": {"topic": "alerts/temp"}
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleAction {
    /// Action function name.
    pub function: String,

    /// Action arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

/// Rule definition.
///
/// ## Example
///
/// ```json
/// {
///   "id": "temp_alert",
///   "sql": "SELECT * FROM \"sensors/+/temp\" WHERE payload.value > 100",
///   "actions": [{"function": "republish", "args": {"topic": "alerts/temp"}}],
///   "enabled": true
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleInfo {
    /// Rule identifier.
    pub id: String,

    /// SQL query for the rule.
    pub sql: String,

    /// List of actions to execute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<RuleAction>>,

    /// Whether the rule is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Rule description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Rule metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Request body for creating or updating a rule.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateRuleBody {
    /// Rule identifier.
    pub id: String,

    /// SQL query for the rule.
    pub sql: String,

    /// List of actions to execute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<RuleAction>>,

    /// Whether the rule is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Rule description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Rule metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response for rules list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListRulesResponse {
    /// List of rules.
    pub data: Vec<RuleInfo>,
}

/// Rule test request body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TestRuleBody {
    /// SQL query to test.
    pub sql: String,

    /// Context for testing (topic, payload, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Rule test response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestRuleResponse {
    /// Test result data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// =============================================================================
// Authentication Types
// =============================================================================

/// Authentication provider configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthenticatorInfo {
    /// Authenticator ID.
    pub id: String,

    /// Authenticator type (built_in_database, http, jwt, etc.).
    #[serde(rename = "type")]
    pub auth_type: String,

    /// Whether the authenticator is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Backend configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,

    /// Additional configuration (type-specific).
    #[serde(flatten)]
    pub config: Option<serde_json::Value>,
}

/// Response for authenticators list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListAuthenticatorsResponse {
    /// List of configured authenticators.
    pub data: Vec<AuthenticatorInfo>,
}

/// User in built-in database authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthUser {
    /// User ID (username).
    pub user_id: String,

    /// Whether the user is a superuser.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_superuser: Option<bool>,
}

/// Request body for creating an auth user.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateAuthUserBody {
    /// User ID (username).
    pub user_id: String,

    /// Password.
    pub password: String,

    /// Whether the user is a superuser.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_superuser: Option<bool>,
}

// =============================================================================
// Authorization Types
// =============================================================================

/// Authorization source configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthzSourceInfo {
    /// Source type (built_in_database, file, http, etc.).
    #[serde(rename = "type")]
    pub source_type: String,

    /// Whether the source is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Additional configuration.
    #[serde(flatten)]
    pub config: Option<serde_json::Value>,
}

/// Response for authorization sources list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListAuthzSourcesResponse {
    /// List of authorization sources.
    pub sources: Vec<AuthzSourceInfo>,
}

// =============================================================================
// Listener Types
// =============================================================================

/// Listener configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListenerInfo {
    /// Listener identifier.
    pub id: String,

    /// Listener type (tcp, ssl, ws, wss).
    #[serde(rename = "type")]
    pub listener_type: String,

    /// Bind address and port.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,

    /// Whether the listener is running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,

    /// Current connections on this listener.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_connections: Option<u64>,

    /// Maximum connections allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u64>,
}

/// Response for listeners list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListListenersResponse {
    /// List of configured listeners.
    pub data: Vec<ListenerInfo>,
}

// =============================================================================
// Metrics Types
// =============================================================================

/// Broker metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsInfo {
    /// Total bytes received.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_received: Option<u64>,

    /// Total bytes sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_sent: Option<u64>,

    /// Total messages received.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_received: Option<u64>,

    /// Total messages sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_sent: Option<u64>,

    /// Total messages dropped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_dropped: Option<u64>,

    /// Total messages retained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_retained: Option<u64>,

    /// Messages publish received.
    #[serde(rename = "messages.publish", skip_serializing_if = "Option::is_none")]
    pub messages_publish: Option<u64>,

    /// Current topics count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics_count: Option<u64>,

    /// Current subscriptions count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriptions_count: Option<u64>,

    /// Current connections count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections_count: Option<u64>,

    /// All metrics as a flexible map.
    #[serde(flatten)]
    pub extra: Option<serde_json::Value>,
}

/// Response for metrics list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListMetricsResponse {
    /// List of per-node metrics.
    pub data: Vec<MetricsInfo>,
}

// =============================================================================
// Stats Types
// =============================================================================

/// Broker statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatsInfo {
    /// Current connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<u64>,

    /// Live connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_connections: Option<u64>,

    /// Retained messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retained_count: Option<u64>,

    /// Topics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<u64>,

    /// Subscriptions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriptions: Option<u64>,

    /// All stats as a flexible map.
    #[serde(flatten)]
    pub extra: Option<serde_json::Value>,
}

/// Response for stats list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListStatsResponse {
    /// List of per-node stats.
    pub data: Vec<StatsInfo>,
}

// =============================================================================
// Topic Types
// =============================================================================

/// Topic information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicInfo {
    /// Topic name.
    pub topic: String,

    /// Node where topic exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
}

/// Paginated response for topics list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTopicsResponse {
    /// List of topics.
    pub data: Vec<TopicInfo>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

// =============================================================================
// Retained Messages Types
// =============================================================================

/// Retained message information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetainedMessage {
    /// Topic of the retained message.
    pub topic: String,

    /// Message payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,

    /// QoS level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qos: Option<u8>,

    /// Publish timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_at: Option<String>,

    /// Publisher client ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_clientid: Option<String>,

    /// Publisher username.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_username: Option<String>,
}

/// Paginated response for retained messages list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListRetainedResponse {
    /// List of retained messages.
    pub data: Vec<RetainedMessage>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

// =============================================================================
// Banned Types
// =============================================================================

/// Ban record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BanInfo {
    /// Ban type (clientid, username, peerhost).
    #[serde(rename = "as")]
    pub ban_as: String,

    /// Banned value.
    pub who: String,

    /// Reason for ban.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Ban start time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,

    /// Ban expiration time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
}

/// Request body for creating a ban.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBanBody {
    /// Ban type (clientid, username, peerhost).
    #[serde(rename = "as")]
    pub ban_as: String,

    /// Value to ban.
    pub who: String,

    /// Reason for ban.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Ban duration in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

/// Response for banned list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBannedResponse {
    /// List of ban records.
    pub data: Vec<BanInfo>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

// =============================================================================
// Alarms Types
// =============================================================================

/// Alarm information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmInfo {
    /// Node where alarm was raised.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,

    /// Alarm name.
    pub name: String,

    /// Alarm message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Alarm details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,

    /// Activation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activate_at: Option<String>,

    /// Deactivation timestamp (if cleared).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deactivate_at: Option<String>,

    /// Duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

/// Response for alarms list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListAlarmsResponse {
    /// List of alarms.
    pub data: Vec<AlarmInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_body_serialization() {
        let body = LoginBody {
            username: "admin".to_string(),
            password: "public".to_string(),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"username\":\"admin\""));
        assert!(json.contains("\"password\":\"public\""));
    }

    #[test]
    fn publish_body_serialization() {
        let body = PublishBody {
            topic: "sensors/temp".to_string(),
            payload: r#"{"temp": 25}"#.to_string(),
            qos: Some(1),
            retain: Some(false),
            encoding: None,
            payload_format_indicator: None,
            message_expiry_interval: None,
            user_properties: None,
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"topic\":\"sensors/temp\""));
        assert!(json.contains("\"qos\":1"));
    }

    #[test]
    fn node_info_deserialization() {
        let json = r#"{
            "node": "emqx@127.0.0.1",
            "version": "5.0.0",
            "status": "running",
            "connections": 1000
        }"#;
        let node: NodeInfo = serde_json::from_str(json).unwrap();
        assert_eq!(node.node, "emqx@127.0.0.1");
        assert_eq!(node.version, Some("5.0.0".to_string()));
        assert_eq!(node.connections, Some(1000));
    }

    #[test]
    fn error_response_deserialization() {
        let json = r#"{"code": "RESOURCE_NOT_FOUND", "reason": "Client id not found"}"#;
        let err: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(err.code, "RESOURCE_NOT_FOUND");
        assert_eq!(err.reason, "Client id not found");
    }

    #[test]
    fn pagination_meta_deserialization() {
        let json = r#"{"count": 100, "limit": 50, "page": 1, "hasnext": true}"#;
        let meta: PaginationMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.count, Some(100));
        assert_eq!(meta.hasnext, Some(true));
    }

    #[test]
    fn rule_info_deserialization() {
        let json = r#"{
            "id": "temp_alert",
            "sql": "SELECT * FROM \"sensors/#\"",
            "enabled": true
        }"#;
        let rule: RuleInfo = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "temp_alert");
        assert_eq!(rule.enabled, Some(true));
    }
}
