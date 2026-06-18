use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::schema::Schema;

/// Direction of message flow in a WebSocket connection.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::MessageDirection;
/// use std::str::FromStr;
///
/// // Display as lowercase
/// assert_eq!(MessageDirection::Client.to_string(), "client");
/// assert_eq!(MessageDirection::Server.to_string(), "server");
/// assert_eq!(MessageDirection::Bidirectional.to_string(), "bidirectional");
///
/// // Parse from lowercase
/// assert_eq!(
///     MessageDirection::from_str("bidirectional").unwrap(),
///     MessageDirection::Bidirectional
/// );
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumIter, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum MessageDirection {
    /// Message sent from client to server
    Client,
    /// Message sent from server to client
    Server,
    /// Message can flow in either direction
    Bidirectional,
}

/// A message schema definition for WebSocket communication.
///
/// Defines a single message type that can be sent or received
/// over the WebSocket connection.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{MessageSchema, MessageDirection};
/// use schematic_define::Schema;
///
/// let message = MessageSchema {
///     name: "TextChunk".to_string(),
///     direction: MessageDirection::Client,
///     schema: Schema::new("TextChunkMessage"),
///     description: Some("A chunk of text to process".to_string()),
/// };
///
/// assert_eq!(message.name, "TextChunk");
/// assert_eq!(message.direction, MessageDirection::Client);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSchema {
    /// Name of this message type (used for generated enum variants).
    pub name: String,
    /// Direction this message flows.
    pub direction: MessageDirection,
    /// Schema for the message payload.
    pub schema: Schema,
    /// Human-readable description of the message.
    pub description: Option<String>,
}

/// Lifecycle messages for WebSocket connection management.
///
/// Defines optional messages for connection open, close, and keepalive.
/// These are separate from regular messages as they have special semantics.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{ConnectionLifecycle, MessageSchema, MessageDirection};
/// use schematic_define::Schema;
///
/// // Empty lifecycle (no special messages)
/// let empty = ConnectionLifecycle::default();
/// assert!(empty.open.is_none());
/// assert!(empty.close.is_none());
/// assert!(empty.keepalive.is_none());
///
/// // With open/close messages
/// let lifecycle = ConnectionLifecycle {
///     open: Some(MessageSchema {
///         name: "Init".to_string(),
///         direction: MessageDirection::Client,
///         schema: Schema::new("InitMessage"),
///         description: Some("Initialization message".to_string()),
///     }),
///     close: None,
///     keepalive: None,
/// };
/// assert!(lifecycle.open.is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionLifecycle {
    /// Message to send when connection is established.
    pub open: Option<MessageSchema>,
    /// Message to send before closing the connection.
    pub close: Option<MessageSchema>,
    /// Message for keepalive/heartbeat (if required by the API).
    pub keepalive: Option<MessageSchema>,
}
