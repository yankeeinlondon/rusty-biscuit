use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::PaginationMeta;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListSubscriptionsResponse {
    /// List of subscriptions.
    pub data: Vec<SubscriptionInfo>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

/// Topic information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TopicInfo {
    /// Topic name.
    pub topic: String,

    /// Node where topic exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
}

/// Paginated response for topics list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListTopicsResponse {
    /// List of topics.
    pub data: Vec<TopicInfo>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

/// Retained message information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListRetainedResponse {
    /// List of retained messages.
    pub data: Vec<RetainedMessage>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PublishBatchBody {
    /// List of messages to publish.
    pub messages: Vec<PublishBody>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
