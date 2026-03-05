//! Unfolded Circle Dock WebSocket model types.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Dock WebSocket envelope discriminator (`type` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockWsEnvelopeType {
    /// Request envelope (`type=req`).
    Req,
    /// Response envelope (`type=resp`).
    Resp,
    /// Event envelope (`type=event`).
    Event,
}

/// Known Dock message identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockWsKnownMessage {
    /// Message-based authentication request.
    Auth,
    /// Server asks the peer to authenticate with a message.
    AuthRequired,
    /// Authentication result/confirmation.
    Authentication,
    /// Send an IR command.
    IrSend,
    /// Get dock status (charging, battery, connection).
    GetStatus,
    /// Start IR learning mode.
    IrLearnStart,
    /// Stop IR learning mode.
    IrLearnStop,
    /// IR learning result.
    IrLearnResult,
    /// Get dock configuration.
    GetConfig,
    /// Dock status update event.
    StatusUpdate,
}

/// Message identifier with known variants and passthrough fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DockWsMessageName {
    /// Known fixed message identifiers.
    Known(DockWsKnownMessage),
    /// Custom or future message identifiers.
    Other(String),
}

/// Dock request envelope (`type=req`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsRequestEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Client request id.
    pub id: u64,
    /// Command message id/name.
    pub msg: DockWsMessageName,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Dock response envelope (`type=resp`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsResponseEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Correlation id.
    pub req_id: u64,
    /// Command message id/name.
    pub msg: DockWsMessageName,
    /// Response status code.
    pub code: i32,
    /// Optional reboot flag.
    pub reboot: Option<bool>,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Dock event envelope (`type=event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsEventEnvelope {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Event message id/name.
    pub msg: DockWsMessageName,
    /// Optional raw payload. Deserialize after routing by `msg`.
    pub msg_data: Option<Box<RawValue>>,
}

/// Dock auth message for post-connect authentication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DockWsAuthMessage {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Message id/name (`auth`).
    pub msg: DockWsKnownMessage,
    /// Auth token.
    pub token: String,
}

/// Dock auth-required event (`msg=auth_required`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DockWsAuthRequired {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Message id/name (`auth_required`).
    pub msg: DockWsKnownMessage,
}

/// Dock authentication result (`msg=authentication`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockWsAuthenticationMessage {
    /// Envelope type discriminator.
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Message id/name (`authentication`).
    pub msg: DockWsKnownMessage,
    /// Optional auth status payload.
    pub msg_data: Option<Box<RawValue>>,
}

impl DockWsRequestEnvelope {
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

impl DockWsResponseEnvelope {
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

impl DockWsAuthenticationMessage {
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

impl DockWsEventEnvelope {
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

    // ── Enum serialization (Priority 1) ──────────────────────────────

    #[test]
    fn envelope_type_serializes_to_lowercase() {
        assert_eq!(serde_json::to_string(&DockWsEnvelopeType::Req).unwrap(), "\"req\"");
        assert_eq!(serde_json::to_string(&DockWsEnvelopeType::Resp).unwrap(), "\"resp\"");
        assert_eq!(serde_json::to_string(&DockWsEnvelopeType::Event).unwrap(), "\"event\"");
    }

    #[test]
    fn known_message_serializes_to_snake_case() {
        let cases = [
            (DockWsKnownMessage::Auth, "\"auth\""),
            (DockWsKnownMessage::AuthRequired, "\"auth_required\""),
            (DockWsKnownMessage::Authentication, "\"authentication\""),
            (DockWsKnownMessage::IrSend, "\"ir_send\""),
            (DockWsKnownMessage::GetStatus, "\"get_status\""),
            (DockWsKnownMessage::IrLearnStart, "\"ir_learn_start\""),
            (DockWsKnownMessage::IrLearnStop, "\"ir_learn_stop\""),
            (DockWsKnownMessage::IrLearnResult, "\"ir_learn_result\""),
            (DockWsKnownMessage::GetConfig, "\"get_config\""),
            (DockWsKnownMessage::StatusUpdate, "\"status_update\""),
        ];
        for (variant, expected) in cases {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected, "failed for {variant:?}");
        }
    }

    // ── Untagged enum behavior (Priority 2) ──────────────────────────

    #[test]
    fn message_name_deserializes_known_variant() {
        let name: DockWsMessageName = serde_json::from_str("\"auth\"").unwrap();
        assert_eq!(name, DockWsMessageName::Known(DockWsKnownMessage::Auth));
    }

    #[test]
    fn message_name_deserializes_known_new_variant() {
        let name: DockWsMessageName = serde_json::from_str("\"ir_send\"").unwrap();
        assert_eq!(name, DockWsMessageName::Known(DockWsKnownMessage::IrSend));
    }

    #[test]
    fn message_name_deserializes_unknown_as_other() {
        let name: DockWsMessageName = serde_json::from_str("\"some_future_msg\"").unwrap();
        assert_eq!(name, DockWsMessageName::Other("some_future_msg".to_string()));
    }

    // ── serde(rename = "type") field (Priority 4) ────────────────────

    #[test]
    fn request_envelope_roundtrip_uses_type_field() {
        let json = r#"{"type":"req","id":1,"msg":"get_status"}"#;
        let env: DockWsRequestEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(env.type_name, DockWsEnvelopeType::Req);
        assert_eq!(env.msg, DockWsMessageName::Known(DockWsKnownMessage::GetStatus));
        assert!(env.msg_data.is_none());

        let serialized = serde_json::to_string(&env).unwrap();
        assert!(serialized.contains("\"type\":"), "serialized JSON must use \"type\" not \"type_name\": {serialized}");
        assert!(!serialized.contains("\"type_name\":"), "must not contain \"type_name\": {serialized}");
    }

    #[test]
    fn response_envelope_serializes_with_type_field() {
        let env = DockWsResponseEnvelope {
            type_name: DockWsEnvelopeType::Resp,
            req_id: 1,
            msg: DockWsMessageName::Known(DockWsKnownMessage::Auth),
            code: 200,
            reboot: None,
            msg_data: None,
        };
        let serialized = serde_json::to_string(&env).unwrap();
        assert!(serialized.contains("\"type\":"));
        assert!(!serialized.contains("\"type_name\":"));
    }

    #[test]
    fn event_envelope_serializes_with_type_field() {
        let env = DockWsEventEnvelope {
            type_name: DockWsEnvelopeType::Event,
            msg: DockWsMessageName::Known(DockWsKnownMessage::StatusUpdate),
            msg_data: None,
        };
        let serialized = serde_json::to_string(&env).unwrap();
        assert!(serialized.contains("\"type\":"));
        assert!(!serialized.contains("\"type_name\":"));
    }

    #[test]
    fn auth_message_serializes_with_type_field() {
        let env = DockWsAuthMessage {
            type_name: DockWsEnvelopeType::Req,
            msg: DockWsKnownMessage::Auth,
            token: "test-token".to_string(),
        };
        let serialized = serde_json::to_string(&env).unwrap();
        assert!(serialized.contains("\"type\":"));
        assert!(!serialized.contains("\"type_name\":"));
    }

    #[test]
    fn auth_required_serializes_with_type_field() {
        let env = DockWsAuthRequired {
            type_name: DockWsEnvelopeType::Event,
            msg: DockWsKnownMessage::AuthRequired,
        };
        let serialized = serde_json::to_string(&env).unwrap();
        assert!(serialized.contains("\"type\":"));
        assert!(!serialized.contains("\"type_name\":"));
    }

    #[test]
    fn authentication_message_serializes_with_type_field() {
        let env = DockWsAuthenticationMessage {
            type_name: DockWsEnvelopeType::Resp,
            msg: DockWsKnownMessage::Authentication,
            msg_data: None,
        };
        let serialized = serde_json::to_string(&env).unwrap();
        assert!(serialized.contains("\"type\":"));
        assert!(!serialized.contains("\"type_name\":"));
    }

    // ── Envelope roundtrip with RawValue (Priority 3) ────────────────

    #[test]
    fn request_envelope_roundtrip_with_msg_data() {
        let json = r#"{"type":"req","id":42,"msg":"ir_send","msg_data":{"code":"ABC123","repeat":2}}"#;
        let env: DockWsRequestEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(env.id, 42);
        assert!(env.msg_data.is_some());

        let reserialized = serde_json::to_string(&env).unwrap();
        let roundtrip: DockWsRequestEnvelope = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(roundtrip.id, env.id);
        assert_eq!(roundtrip.msg, env.msg);
    }

    #[test]
    fn auth_message_roundtrip() {
        let json = r#"{"type":"req","msg":"auth","token":"my-secret"}"#;
        let env: DockWsAuthMessage = serde_json::from_str(json).unwrap();
        assert_eq!(env.token, "my-secret");
        assert_eq!(env.msg, DockWsKnownMessage::Auth);

        let reserialized = serde_json::to_string(&env).unwrap();
        let roundtrip: DockWsAuthMessage = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(roundtrip.token, env.token);
    }

    // ── parse_payload tests ──────────────────────────────────────────

    #[derive(Debug, Deserialize, PartialEq)]
    struct IrPayload {
        code: String,
        repeat: u32,
    }

    #[test]
    fn request_parse_payload_with_data() {
        let raw: Box<RawValue> = RawValue::from_string(r#"{"code":"NEC-1234","repeat":3}"#.to_string()).unwrap();
        let env = DockWsRequestEnvelope {
            type_name: DockWsEnvelopeType::Req,
            id: 1,
            msg: DockWsMessageName::Known(DockWsKnownMessage::IrSend),
            msg_data: Some(raw),
        };
        let payload: IrPayload = env.parse_payload().unwrap();
        assert_eq!(payload, IrPayload { code: "NEC-1234".to_string(), repeat: 3 });
    }

    #[test]
    fn request_parse_payload_returns_error_when_absent() {
        let env = DockWsRequestEnvelope {
            type_name: DockWsEnvelopeType::Req,
            id: 1,
            msg: DockWsMessageName::Known(DockWsKnownMessage::GetStatus),
            msg_data: None,
        };
        let result = env.parse_payload::<IrPayload>();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("msg_data is absent"));
    }

    #[test]
    fn event_parse_payload_with_data() {
        let raw: Box<RawValue> = RawValue::from_string(r#"{"code":"RC5-99","repeat":1}"#.to_string()).unwrap();
        let env = DockWsEventEnvelope {
            type_name: DockWsEnvelopeType::Event,
            msg: DockWsMessageName::Known(DockWsKnownMessage::IrLearnResult),
            msg_data: Some(raw),
        };
        let payload: IrPayload = env.parse_payload().unwrap();
        assert_eq!(payload.code, "RC5-99");
    }

    #[test]
    fn event_parse_payload_returns_error_when_absent() {
        let env = DockWsEventEnvelope {
            type_name: DockWsEnvelopeType::Event,
            msg: DockWsMessageName::Known(DockWsKnownMessage::StatusUpdate),
            msg_data: None,
        };
        let result = env.parse_payload::<IrPayload>();
        assert!(result.is_err());
    }
}
