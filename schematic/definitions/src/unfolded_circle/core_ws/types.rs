//! Unfolded Circle Core WebSocket model types.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Core WebSocket envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoreWsEnvelopeKind {
    /// Request envelope (`kind=req`).
    Req,
    /// Response envelope (`kind=resp`).
    Resp,
    /// Event envelope (`kind=event`).
    Event,
}

/// Known Core message identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreWsKnownMessage {
    /// Message-based authentication request.
    Auth,
    /// Server asks the client to authenticate with a message.
    AuthRequired,
    /// Authentication result/confirmation.
    Authentication,
    /// Ping keepalive message.
    Ping,
    /// Pong keepalive response.
    Pong,
    /// Generic command result message.
    Result,
    /// Version request message.
    Version,
    /// Version information response.
    VersionInfo,
    /// System information request message.
    System,
    /// System information response message.
    SystemInfo,

    // Entity messages
    /// Get all entities.
    GetEntities,
    /// Get entity states.
    GetEntityStates,
    /// Entity command.
    EntityCommand,
    /// Subscribe to entity events.
    SubscribeEntities,
    /// Unsubscribe from entity events.
    UnsubscribeEntities,
    /// Entity state change event.
    EntityChange,

    // Profile messages
    /// Get all profiles.
    GetProfiles,
    /// Get the active profile.
    GetActiveProfile,
    /// Set the active profile.
    SetActiveProfile,
    /// Profile change event.
    ProfileChange,

    // Integration messages
    /// Get all integrations.
    GetIntegrations,
    /// Get integration driver info.
    GetIntegrationDriver,
    /// Configure an integration driver.
    ConfigureIntegrationDriver,
    /// Delete an integration driver.
    DeleteIntegrationDriver,
    /// Integration driver change event.
    IntegrationChange,
    /// Integration driver state change event.
    IntegrationDriverChange,

    // Activity messages
    /// Get all activities.
    GetActivities,
    /// Get activity groups.
    GetActivityGroups,
    /// Activity group change event.
    ActivityGroupChange,

    // Remote/UI messages
    /// Get all remotes/pages.
    GetRemotes,

    // Dock messages
    /// Get configured docks.
    GetDocks,
    /// Get dock state.
    GetDockState,
    /// Dock change event.
    DockChange,

    // Software update messages
    /// Check for software updates.
    CheckUpdate,
    /// Start software update.
    StartUpdate,
    /// Software update change event.
    SoftwareUpdateChange,

    // WiFi messages
    /// Get WiFi status.
    GetWifiStatus,
    /// Scan WiFi networks.
    ScanWifi,
    /// WiFi change event.
    WifiChange,
}

/// Message identifier with known variants and passthrough fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoreWsMessageName {
    /// Known fixed message identifiers.
    Known(CoreWsKnownMessage),
    /// Custom or future message identifiers.
    Other(String),
}

/// Known Core WS event categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoreWsKnownCategory {
    /// Entity-related events.
    Entity,
    /// Integration-related events.
    Integration,
    /// System-level events.
    System,
    /// Activity-related events.
    Activity,
    /// Profile-related events.
    Profile,
    /// Remote/UI-related events.
    Remote,
    /// Dock-related events.
    Dock,
    /// Software update events.
    SoftwareUpdate,
}

/// Event category with known variants and passthrough fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoreWsEventCategory {
    /// Known event category.
    Known(CoreWsKnownCategory),
    /// Custom or future event category.
    Other(String),
}

/// Generic Core WS request envelope (`kind=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsRequestEnvelope {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client-generated correlation id.
    pub id: u64,
    /// Message name.
    pub msg: CoreWsMessageName,
    /// Optional raw payload.
    ///
    /// The payload is intentionally deferred as [`RawValue`] to avoid eagerly
    /// allocating a full `serde_json::Value` tree before routing.
    ///
    /// Deserialize on demand:
    ///
    /// ```rust
    /// use serde::Deserialize;
    /// use serde_json::value::RawValue;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct PingPayload {
    ///     id: u64,
    /// }
    ///
    /// fn decode(raw: &RawValue) -> serde_json::Result<PingPayload> {
    ///     serde_json::from_str(raw.get())
    /// }
    /// ```
    pub msg_data: Option<Box<RawValue>>,
}

/// Generic Core WS response envelope (`kind=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsResponseEnvelope {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlation id matching request `id`.
    pub req_id: u64,
    /// Message name.
    pub msg: CoreWsMessageName,
    /// Response status code (follows HTTP status code conventions).
    pub code: i32,
    /// Optional raw payload. Deserialize this after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Generic Core WS event envelope (`kind=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsEventEnvelope {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Event message id/name.
    pub msg: CoreWsMessageName,
    /// Optional category.
    pub cat: Option<CoreWsEventCategory>,
    /// Optional event timestamp (ISO 8601 format).
    pub ts: Option<String>,
    /// Event payload (raw JSON). Deserialize by inspecting `msg`.
    pub msg_data: Box<RawValue>,
}

/// Auth-required event emitted by the server.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CoreWsAuthRequired {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Event message id/name (`auth_required`).
    pub msg: CoreWsKnownMessage,
}

/// Message-based authentication request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsAuthMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Message id/name (`auth`).
    pub msg: CoreWsKnownMessage,
    /// Authentication payload.
    pub msg_data: Box<RawValue>,
}

/// Message-based authentication result from the server (`msg=authentication`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsAuthenticationMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Message id/name (`authentication`).
    pub msg: CoreWsKnownMessage,
    /// Optional server payload for auth status/details.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed ping request (`msg=ping`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsPingMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client correlation id.
    pub id: u64,
    /// Message id/name (`ping`).
    pub msg: CoreWsKnownMessage,
    /// Optional ping payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed pong response (`msg=pong`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsPongMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`pong`).
    pub msg: CoreWsKnownMessage,
    /// Optional pong payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed version request (`msg=version`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsVersionMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client correlation id.
    pub id: u64,
    /// Message id/name (`version`).
    pub msg: CoreWsKnownMessage,
    /// Optional version request payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed version info response (`msg=version_info`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsVersionInfoMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`version_info`).
    pub msg: CoreWsKnownMessage,
    /// Optional version payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed system-info request (`msg=system`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsSystemMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Client correlation id.
    pub id: u64,
    /// Message id/name (`system`).
    pub msg: CoreWsKnownMessage,
    /// Optional system request payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed system-info response (`msg=system_info`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsSystemInfoMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`system_info`).
    pub msg: CoreWsKnownMessage,
    /// Optional system payload.
    pub msg_data: Option<Box<RawValue>>,
}

/// Typed generic result response (`msg=result`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWsResultMessage {
    /// Message kind discriminator.
    pub kind: CoreWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name (`result`).
    pub msg: CoreWsKnownMessage,
    /// Optional result payload.
    pub msg_data: Option<Box<RawValue>>,
}

impl CoreWsRequestEnvelope {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsResponseEnvelope {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsEventEnvelope {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if the payload does not match the
    /// target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(self.msg_data.get())
    }
}

impl CoreWsAuthMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if the payload does not match the
    /// target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(self.msg_data.get())
    }
}

impl CoreWsAuthenticationMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsPingMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsPongMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsVersionMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsVersionInfoMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsSystemMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsSystemInfoMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

impl CoreWsResultMessage {
    /// Deserialize the deferred `msg_data` payload into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if `msg_data` is absent or the payload
    /// does not match the target type `T`.
    pub fn parse_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::RawValue;

    // ── Priority 1: Enum serialization ──────────────────────────────────

    #[test]
    fn envelope_kind_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_string(&CoreWsEnvelopeKind::Req).unwrap(),
            "\"req\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsEnvelopeKind::Resp).unwrap(),
            "\"resp\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsEnvelopeKind::Event).unwrap(),
            "\"event\""
        );
    }

    #[test]
    fn known_message_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::Auth).unwrap(),
            "\"auth\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::AuthRequired).unwrap(),
            "\"auth_required\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::Authentication).unwrap(),
            "\"authentication\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::Ping).unwrap(),
            "\"ping\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::Pong).unwrap(),
            "\"pong\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::Result).unwrap(),
            "\"result\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::Version).unwrap(),
            "\"version\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::VersionInfo).unwrap(),
            "\"version_info\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::System).unwrap(),
            "\"system\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::SystemInfo).unwrap(),
            "\"system_info\""
        );
    }

    #[test]
    fn new_known_message_variants_serialize_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::GetEntities).unwrap(),
            "\"get_entities\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::EntityCommand).unwrap(),
            "\"entity_command\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownMessage::SoftwareUpdateChange).unwrap(),
            "\"software_update_change\""
        );
    }

    #[test]
    fn known_category_serializes_to_screaming_snake_case() {
        assert_eq!(
            serde_json::to_string(&CoreWsKnownCategory::Entity).unwrap(),
            "\"ENTITY\""
        );
        assert_eq!(
            serde_json::to_string(&CoreWsKnownCategory::SoftwareUpdate).unwrap(),
            "\"SOFTWARE_UPDATE\""
        );
    }

    // ── Priority 2: Untagged enum behavior ──────────────────────────────

    #[test]
    fn message_name_deserializes_known_variant() {
        let name: CoreWsMessageName = serde_json::from_str("\"auth\"").unwrap();
        assert_eq!(name, CoreWsMessageName::Known(CoreWsKnownMessage::Auth));
    }

    #[test]
    fn message_name_deserializes_new_known_variant() {
        let name: CoreWsMessageName = serde_json::from_str("\"get_entities\"").unwrap();
        assert_eq!(
            name,
            CoreWsMessageName::Known(CoreWsKnownMessage::GetEntities)
        );
    }

    #[test]
    fn message_name_deserializes_unknown_as_other() {
        let name: CoreWsMessageName = serde_json::from_str("\"some_future_msg\"").unwrap();
        assert_eq!(
            name,
            CoreWsMessageName::Other("some_future_msg".to_string())
        );
    }

    #[test]
    fn event_category_deserializes_known_variant() {
        let cat: CoreWsEventCategory = serde_json::from_str("\"ENTITY\"").unwrap();
        assert_eq!(cat, CoreWsEventCategory::Known(CoreWsKnownCategory::Entity));
    }

    #[test]
    fn event_category_deserializes_unknown_as_other() {
        let cat: CoreWsEventCategory = serde_json::from_str("\"CUSTOM_CAT\"").unwrap();
        assert_eq!(cat, CoreWsEventCategory::Other("CUSTOM_CAT".to_string()));
    }

    // ── Priority 3: Envelope roundtrip with RawValue ────────────────────

    #[test]
    fn request_envelope_roundtrip() {
        let raw: Box<RawValue> = serde_json::value::to_raw_value(&serde_json::json!({})).unwrap();
        let envelope = CoreWsRequestEnvelope {
            kind: CoreWsEnvelopeKind::Req,
            id: 42,
            msg: CoreWsMessageName::Known(CoreWsKnownMessage::Ping),
            msg_data: Some(raw),
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: CoreWsRequestEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.kind, CoreWsEnvelopeKind::Req);
        assert_eq!(decoded.id, 42);
        assert_eq!(
            decoded.msg,
            CoreWsMessageName::Known(CoreWsKnownMessage::Ping)
        );
        assert!(decoded.msg_data.is_some());
    }

    #[test]
    fn response_envelope_roundtrip() {
        let envelope = CoreWsResponseEnvelope {
            kind: CoreWsEnvelopeKind::Resp,
            req_id: 42,
            msg: CoreWsMessageName::Known(CoreWsKnownMessage::Pong),
            code: 200,
            msg_data: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: CoreWsResponseEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.kind, CoreWsEnvelopeKind::Resp);
        assert_eq!(decoded.req_id, 42);
        assert_eq!(
            decoded.msg,
            CoreWsMessageName::Known(CoreWsKnownMessage::Pong)
        );
        assert_eq!(decoded.code, 200);
        assert!(decoded.msg_data.is_none());
    }

    #[test]
    fn event_envelope_roundtrip() {
        let raw: Box<RawValue> = serde_json::value::to_raw_value(&serde_json::json!({})).unwrap();
        let envelope = CoreWsEventEnvelope {
            kind: CoreWsEnvelopeKind::Event,
            msg: CoreWsMessageName::Known(CoreWsKnownMessage::EntityChange),
            cat: Some(CoreWsEventCategory::Known(CoreWsKnownCategory::Entity)),
            ts: Some("2024-01-01T00:00:00Z".to_string()),
            msg_data: raw,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains("\"ENTITY\""));

        let decoded: CoreWsEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.kind, CoreWsEnvelopeKind::Event);
        assert_eq!(
            decoded.msg,
            CoreWsMessageName::Known(CoreWsKnownMessage::EntityChange)
        );
        assert_eq!(
            decoded.cat,
            Some(CoreWsEventCategory::Known(CoreWsKnownCategory::Entity))
        );
        assert_eq!(decoded.ts.as_deref(), Some("2024-01-01T00:00:00Z"));
    }

    // ── parse_payload tests ─────────────────────────────────────────────

    #[test]
    fn parse_payload_with_present_msg_data() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Payload {
            id: u64,
        }

        let raw: Box<RawValue> =
            serde_json::value::to_raw_value(&serde_json::json!({"id": 1})).unwrap();
        let envelope = CoreWsRequestEnvelope {
            kind: CoreWsEnvelopeKind::Req,
            id: 1,
            msg: CoreWsMessageName::Known(CoreWsKnownMessage::Ping),
            msg_data: Some(raw),
        };

        let payload: Payload = envelope.parse_payload().unwrap();
        assert_eq!(payload, Payload { id: 1 });
    }

    #[test]
    fn parse_payload_returns_error_when_absent() {
        let envelope = CoreWsRequestEnvelope {
            kind: CoreWsEnvelopeKind::Req,
            id: 1,
            msg: CoreWsMessageName::Known(CoreWsKnownMessage::Ping),
            msg_data: None,
        };

        let result = envelope.parse_payload::<serde_json::Value>();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("msg_data is absent")
        );
    }
}
