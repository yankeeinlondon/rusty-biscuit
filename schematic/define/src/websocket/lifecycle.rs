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

/// One equality check on an existing top-level wire field of an inbound
/// frame: the frame's `field` must be the JSON string `value`.
///
/// A message's selectors are a conjunction; together they decide which
/// message an inbound frame is decoded as, before any payload is decoded.
/// `field` uses the same field syntax as [`crate::websocket::CorrelationHints`]
/// (a top-level key); no nested path is introduced.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::MessageSelector;
///
/// let selector = MessageSelector::new("kind", "resp");
/// assert_eq!(serde_json::to_string(&selector).unwrap(), r#"{"field":"kind","value":"resp"}"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct MessageSelector {
    /// Wire field the frame is checked on.
    pub field: String,
    /// The string the field must equal.
    pub value: String,
}

impl<'de> Deserialize<'de> for MessageSelector {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename = "MessageSelector")]
        struct Fields {
            field: String,
            value: String,
        }
        // An object only: `["kind", "req"]` is not a selector.
        crate::serde_presence::map_only::<D, Fields>(deserializer)
            .map(|fields| Self { field: fields.field, value: fields.value })
    }
}

impl MessageSelector {
    /// A check that wire field `field` equals `value`.
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self { field: field.into(), value: value.into() }
    }
}

/// A message schema definition for WebSocket communication.
///
/// Defines a single message type that can be sent or received over the
/// WebSocket connection. Directions are client-relative: a `Client` message
/// is sent by the client and received by a generated host.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{MessageDirection, MessageSchema};
/// use schematic_define::Schema;
///
/// let ping = MessageSchema::new("Ping", MessageDirection::Client, Schema::new("PingMessage"))
///     .matches("msg", "ping")
///     .responds_with("Pong");
/// let pong = MessageSchema::new("Pong", MessageDirection::Server, Schema::new("PongMessage"))
///     .matches("kind", "resp")
///     .matches("msg", "pong");
/// let other = MessageSchema::new("Response", MessageDirection::Server, Schema::new("Envelope"))
///     .fallback();
///
/// assert_eq!(ping.responds_with.as_deref(), Some("Pong"));
/// assert_eq!(pong.selectors.as_ref().map(Vec::len), Some(2));
/// assert!(other.fallback && other.selectors.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSchema {
    /// Name of this message type (used for generated enum variants).
    pub name: String,
    /// Direction this message flows, from the client's point of view.
    pub direction: MessageDirection,
    /// Schema for the message payload.
    pub schema: Schema,
    /// Human-readable description of the message.
    pub description: Option<String>,
    /// On a request message, the name of the message in the same endpoint
    /// that answers it. `None` declares no response link; nothing is inferred
    /// from message names.
    #[serde(
        default,
        deserialize_with = "crate::serde_presence::non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub responds_with: Option<String>,
    /// Wire checks that select this message for an inbound frame. `None`
    /// declares no selectors; `Some(vec![])` is kept as declared and left to
    /// generation-time validation.
    #[serde(
        default,
        deserialize_with = "crate::serde_presence::non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub selectors: Option<Vec<MessageSelector>>,
    /// Whether an inbound frame matching no selector in this message's
    /// direction is decoded as this message.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fallback: bool,
}

impl MessageSchema {
    /// A message with no description, selectors, response link, or fallback.
    pub fn new(name: impl Into<String>, direction: MessageDirection, schema: Schema) -> Self {
        Self {
            name: name.into(),
            direction,
            schema,
            description: None,
            responds_with: None,
            selectors: None,
            fallback: false,
        }
    }

    /// Sets the human-readable description.
    #[must_use]
    pub fn describe(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds the selector `field == value`.
    #[must_use]
    pub fn matches(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.selectors
            .get_or_insert_with(Vec::new)
            .push(MessageSelector::new(field, value));
        self
    }

    /// Marks this message as its direction's fallback.
    #[must_use]
    pub fn fallback(mut self) -> Self {
        self.fallback = true;
        self
    }

    /// Links this request to the message named `response`.
    #[must_use]
    pub fn responds_with(mut self, response: impl Into<String>) -> Self {
        self.responds_with = Some(response.into());
        self
    }
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
///     open: Some(
///         MessageSchema::new("Init", MessageDirection::Client, Schema::new("InitMessage"))
///             .describe("Initialization message"),
///     ),
///     close: None,
///     keepalive: None,
///     open_credential_field: None,
/// };
/// assert!(lifecycle.open.is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionLifecycle {
    /// Message a generated client sends first on every connection, the
    /// initial one and each reconnect, before reporting it ready. The
    /// caller supplies its payload once, when connecting.
    pub open: Option<MessageSchema>,
    /// Message to send before closing the connection.
    pub close: Option<MessageSchema>,
    /// Message for keepalive/heartbeat (if required by the API).
    pub keepalive: Option<MessageSchema>,
    /// The top-level wire field of the `open` payload that carries the API
    /// key or token. A generated client fills it from the credential it
    /// selected for the handshake when the payload leaves it empty, and
    /// refuses a payload whose value differs, so the payload is never a
    /// second credential source. `None` binds no field; nothing is inferred
    /// from the payload's shape.
    #[serde(
        default,
        deserialize_with = "crate::serde_presence::non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub open_credential_field: Option<String>,
}
