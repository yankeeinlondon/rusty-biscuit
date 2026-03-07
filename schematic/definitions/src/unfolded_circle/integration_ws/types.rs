//! Unfolded Circle Integration WebSocket model types.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Integration WebSocket envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationWsEnvelopeKind {
    /// Request envelope (`kind=req`).
    Req,
    /// Response envelope (`kind=resp`).
    Resp,
    /// Event envelope (`kind=event`).
    Event,
}

/// Known Integration message identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationWsKnownMessage {
    /// Message-based authentication request.
    Auth,
    /// Server asks the peer to authenticate with a message.
    AuthRequired,
    /// Authentication result/confirmation.
    Authentication,

    // Required request types (remote → driver)
    /// Get driver version information.
    GetDriverVersion,
    /// Get device connection state.
    GetDeviceState,
    /// Get available entities from the integration.
    GetAvailableEntities,
    /// Subscribe to entity events.
    SubscribeEvents,
    /// Get current entity states.
    GetEntityStates,
    /// Execute an entity command.
    EntityCommand,

    // Required events (driver → remote)
    /// Entity state change event.
    EntityChange,
    /// Device connection state change event.
    DeviceState,
    /// Connection state event (connected/disconnected).
    Connected,
    /// Disconnected state event.
    Disconnected,

    // Setup flow messages
    /// Start driver setup flow.
    SetupDriver,
    /// Provide user data during setup.
    SetDriverUserData,
    /// Driver setup state change event.
    DriverSetupChange,

    // Driver info
    /// Driver version response.
    DriverVersion,

    // Abort
    /// Abort current driver action.
    AbortDriverSetup,

    // Entity management
    /// Available entities response.
    AvailableEntities,
    /// Entity states response.
    EntityStates,

    // Event subscription response
    /// Subscribe events result.
    SubscribeEventsResult,
}

/// Message identifier with known variants and passthrough fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IntegrationWsMessageName {
    /// Known fixed message identifiers.
    Known(IntegrationWsKnownMessage),
    /// Custom or future message identifiers.
    Other(String),
}

/// Integration WS request envelope (`kind=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsRequestEnvelope {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Client-generated correlation id.
    pub id: u64,
    /// Message id/name.
    pub msg: IntegrationWsMessageName,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Integration WS response envelope (`kind=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsResponseEnvelope {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Correlated request id.
    pub req_id: u64,
    /// Message id/name.
    pub msg: IntegrationWsMessageName,
    /// Status code (follows HTTP status code conventions).
    pub code: i32,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Integration WS event envelope (`kind=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsEventEnvelope {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Event id/name.
    pub msg: IntegrationWsMessageName,
    /// Optional category.
    pub cat: Option<String>,
    /// Optional event timestamp (ISO 8601 format).
    pub ts: Option<String>,
    /// Event payload (raw JSON). Deserialize by inspecting `msg`.
    pub msg_data: Box<RawValue>,
}

/// Integration auth message (`msg=auth`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsAuthMessage {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Message id/name (`auth`).
    pub msg: IntegrationWsKnownMessage,
    /// Authentication payload.
    pub msg_data: Box<RawValue>,
}

/// Integration auth-required event from peer (`msg=auth_required`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntegrationWsAuthRequired {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Message id/name (`auth_required`).
    pub msg: IntegrationWsKnownMessage,
}

/// Integration authentication result (`msg=authentication`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWsAuthenticationMessage {
    /// Envelope kind discriminator.
    pub kind: IntegrationWsEnvelopeKind,
    /// Message id/name (`authentication`).
    pub msg: IntegrationWsKnownMessage,
    /// Optional auth status payload.
    pub msg_data: Option<Box<RawValue>>,
}

impl IntegrationWsRequestEnvelope {
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

impl IntegrationWsResponseEnvelope {
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

impl IntegrationWsAuthenticationMessage {
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

impl IntegrationWsEventEnvelope {
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

impl IntegrationWsAuthMessage {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Enum serialization (Priority 1) ──────────────────────────────

    #[test]
    fn envelope_kind_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_string(&IntegrationWsEnvelopeKind::Req).unwrap(),
            "\"req\""
        );
        assert_eq!(
            serde_json::to_string(&IntegrationWsEnvelopeKind::Resp).unwrap(),
            "\"resp\""
        );
        assert_eq!(
            serde_json::to_string(&IntegrationWsEnvelopeKind::Event).unwrap(),
            "\"event\""
        );
    }

    #[test]
    fn known_message_serializes_to_snake_case() {
        let cases: Vec<(IntegrationWsKnownMessage, &str)> = vec![
            (IntegrationWsKnownMessage::Auth, "\"auth\""),
            (IntegrationWsKnownMessage::AuthRequired, "\"auth_required\""),
            (
                IntegrationWsKnownMessage::Authentication,
                "\"authentication\"",
            ),
            (
                IntegrationWsKnownMessage::GetDriverVersion,
                "\"get_driver_version\"",
            ),
            (
                IntegrationWsKnownMessage::GetDeviceState,
                "\"get_device_state\"",
            ),
            (
                IntegrationWsKnownMessage::GetAvailableEntities,
                "\"get_available_entities\"",
            ),
            (
                IntegrationWsKnownMessage::SubscribeEvents,
                "\"subscribe_events\"",
            ),
            (
                IntegrationWsKnownMessage::GetEntityStates,
                "\"get_entity_states\"",
            ),
            (
                IntegrationWsKnownMessage::EntityCommand,
                "\"entity_command\"",
            ),
            (IntegrationWsKnownMessage::EntityChange, "\"entity_change\""),
            (IntegrationWsKnownMessage::DeviceState, "\"device_state\""),
            (IntegrationWsKnownMessage::SetupDriver, "\"setup_driver\""),
            (
                IntegrationWsKnownMessage::SetDriverUserData,
                "\"set_driver_user_data\"",
            ),
            (
                IntegrationWsKnownMessage::DriverSetupChange,
                "\"driver_setup_change\"",
            ),
        ];
        for (variant, expected) in cases {
            assert_eq!(
                serde_json::to_string(&variant).unwrap(),
                expected,
                "failed for {variant:?}"
            );
        }
    }

    // ── Untagged enum behavior (Priority 2) ──────────────────────────

    #[test]
    fn message_name_deserializes_known_auth() {
        let name: IntegrationWsMessageName = serde_json::from_str("\"auth\"").unwrap();
        assert_eq!(
            name,
            IntegrationWsMessageName::Known(IntegrationWsKnownMessage::Auth)
        );
    }

    #[test]
    fn message_name_deserializes_known_entity_command() {
        let name: IntegrationWsMessageName = serde_json::from_str("\"entity_command\"").unwrap();
        assert_eq!(
            name,
            IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityCommand)
        );
    }

    #[test]
    fn message_name_deserializes_unknown_as_other() {
        let name: IntegrationWsMessageName = serde_json::from_str("\"some_future_msg\"").unwrap();
        assert_eq!(
            name,
            IntegrationWsMessageName::Other("some_future_msg".to_string())
        );
    }

    // ── Envelope roundtrip with RawValue (Priority 3) ────────────────

    #[test]
    fn request_envelope_roundtrip() {
        let raw_data =
            RawValue::from_string(r#"{"entity_id":"light.1","cmd":"on"}"#.to_string()).unwrap();
        let envelope = IntegrationWsRequestEnvelope {
            kind: IntegrationWsEnvelopeKind::Req,
            id: 1,
            msg: IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityCommand),
            msg_data: Some(raw_data),
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let roundtripped: IntegrationWsRequestEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtripped.kind, IntegrationWsEnvelopeKind::Req);
        assert_eq!(roundtripped.id, 1);
        assert_eq!(
            roundtripped.msg,
            IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityCommand)
        );
        assert!(roundtripped.msg_data.is_some());
    }

    #[test]
    fn event_envelope_roundtrip() {
        let raw_data = RawValue::from_string("{}".to_string()).unwrap();
        let envelope = IntegrationWsEventEnvelope {
            kind: IntegrationWsEnvelopeKind::Event,
            msg: IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityChange),
            cat: None,
            ts: Some("2024-01-01T00:00:00Z".to_string()),
            msg_data: raw_data,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let roundtripped: IntegrationWsEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtripped.kind, IntegrationWsEnvelopeKind::Event);
        assert_eq!(
            roundtripped.msg,
            IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityChange)
        );
        assert_eq!(roundtripped.ts.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert!(roundtripped.cat.is_none());
    }

    // ── parse_payload tests ──────────────────────────────────────────

    #[test]
    fn request_envelope_parse_payload_succeeds() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Cmd {
            entity_id: String,
        }

        let raw = RawValue::from_string(json!({"entity_id": "light.1"}).to_string()).unwrap();
        let envelope = IntegrationWsRequestEnvelope {
            kind: IntegrationWsEnvelopeKind::Req,
            id: 1,
            msg: IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityCommand),
            msg_data: Some(raw),
        };

        let cmd: Cmd = envelope.parse_payload().unwrap();
        assert_eq!(cmd.entity_id, "light.1");
    }

    #[test]
    fn request_envelope_parse_payload_errors_when_absent() {
        let envelope = IntegrationWsRequestEnvelope {
            kind: IntegrationWsEnvelopeKind::Req,
            id: 1,
            msg: IntegrationWsMessageName::Known(IntegrationWsKnownMessage::GetDriverVersion),
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

    #[test]
    fn event_envelope_parse_payload_succeeds() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct State {
            brightness: u8,
        }

        let raw = RawValue::from_string(json!({"brightness": 255}).to_string()).unwrap();
        let envelope = IntegrationWsEventEnvelope {
            kind: IntegrationWsEnvelopeKind::Event,
            msg: IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityChange),
            cat: None,
            ts: None,
            msg_data: raw,
        };

        let state: State = envelope.parse_payload().unwrap();
        assert_eq!(state.brightness, 255);
    }
}
