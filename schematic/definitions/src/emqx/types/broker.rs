use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClusterStatus {
    /// Running node names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<Vec<String>>,

    /// Stopped node names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<Vec<String>>,
}

/// Listener configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListListenersResponse {
    /// List of configured listeners.
    pub data: Vec<ListenerInfo>,
}

/// Broker metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListMetricsResponse {
    /// List of per-node metrics.
    pub data: Vec<MetricsInfo>,
}

/// Broker statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListStatsResponse {
    /// List of per-node stats.
    pub data: Vec<StatsInfo>,
}

/// Alarm information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListAlarmsResponse {
    /// List of alarms.
    pub data: Vec<AlarmInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
