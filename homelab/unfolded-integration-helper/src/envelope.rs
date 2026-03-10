use std::collections::HashMap;

use schematic_schema::unfolded_circle_integration_ws::{
    IntegrationWsEnvelopeKind, IntegrationWsKnownMessage, IntegrationWsMessageName,
    IntegrationWsRequestEnvelope,
};
use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;

use crate::state_cache::EntityState;

#[derive(Debug, Error)]
pub enum EnvelopeError {
    #[error("invalid request envelope: {0}")]
    InvalidEnvelope(#[from] serde_json::Error),
    #[error("expected request envelope, got {0:?}")]
    UnexpectedKind(IntegrationWsEnvelopeKind),
    #[error("request payload is missing")]
    MissingPayload,
}

/// Typed request wrapper around the UC integration request envelope.
#[derive(Debug, Clone)]
pub struct RequestEnvelope {
    inner: IntegrationWsRequestEnvelope,
}

/// Backwards-compatible request shape used by the integration handlers.
#[derive(Debug, Clone, PartialEq)]
pub struct IntegrationRequest {
    pub id: u64,
    pub msg: String,
    pub msg_data: Value,
}

impl IntegrationRequest {
    pub fn parse(value: Value) -> Result<Self, EnvelopeError> {
        let request = RequestEnvelope::parse(value)?;
        Ok(Self {
            id: request.id(),
            msg: match request.msg() {
                IntegrationWsMessageName::Known(message) => {
                    serde_json::to_value(message).unwrap_or(Value::String("unknown".to_string()))
                }
                IntegrationWsMessageName::Other(message) => Value::String(message.clone()),
            }
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
            msg_data: request.payload_value(),
        })
    }
}

impl RequestEnvelope {
    /// Parse a raw JSON request into the typed UC envelope.
    ///
    /// ## Errors
    ///
    /// Returns an error if the payload is not a valid UC request envelope.
    pub fn parse(value: Value) -> Result<Self, EnvelopeError> {
        let kind: IntegrationWsEnvelopeKind =
            serde_json::from_value(value.get("kind").cloned().unwrap_or(Value::Null))?;
        if kind != IntegrationWsEnvelopeKind::Req {
            return Err(EnvelopeError::UnexpectedKind(kind));
        }

        let inner: IntegrationWsRequestEnvelope = serde_json::from_value(value)?;
        Ok(Self { inner })
    }

    #[must_use]
    pub fn id(&self) -> u64 {
        self.inner.id
    }

    #[must_use]
    pub fn msg(&self) -> &IntegrationWsMessageName {
        &self.inner.msg
    }

    #[must_use]
    pub fn known_message(&self) -> Option<IntegrationWsKnownMessage> {
        match self.inner.msg {
            IntegrationWsMessageName::Known(message) => Some(message),
            IntegrationWsMessageName::Other(_) => None,
        }
    }

    /// Parse `msg_data` into a concrete payload type.
    ///
    /// ## Errors
    ///
    /// Returns an error if the payload is absent or cannot be deserialized.
    pub fn payload<T: serde::de::DeserializeOwned>(&self) -> Result<T, EnvelopeError> {
        self.inner.parse_payload().map_err(|error| {
            if self.inner.msg_data.is_none() {
                EnvelopeError::MissingPayload
            } else {
                EnvelopeError::InvalidEnvelope(error)
            }
        })
    }

    #[must_use]
    pub fn payload_value(&self) -> Value {
        match &self.inner.msg_data {
            Some(raw) => serde_json::from_str(raw.get()).unwrap_or(Value::Null),
            None => Value::Null,
        }
    }
}

#[must_use]
pub fn envelope_kind(value: &Value) -> Option<&str> {
    value.get("kind").and_then(Value::as_str)
}

#[must_use]
pub fn envelope_message_name(value: &Value) -> Option<&str> {
    value.get("msg").and_then(Value::as_str)
}

#[must_use]
pub fn remote_setup_abort_error(value: &Value) -> Option<&str> {
    if envelope_kind(value) == Some("event")
        && envelope_message_name(value) == Some("abort_driver_setup")
    {
        value
            .get("msg_data")
            .and_then(|msg_data| msg_data.get("error"))
            .and_then(Value::as_str)
    } else {
        None
    }
}

fn response(req_id: u64, msg: &str, code: u16, msg_data: Option<Value>) -> Value {
    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": msg,
        "code": code,
        "msg_data": msg_data.unwrap_or_else(|| json!({})),
    })
}

fn event(msg: &str, cat: &str, msg_data: Value) -> Value {
    json!({
        "kind": "event",
        "msg": msg,
        "cat": cat,
        "msg_data": msg_data,
    })
}

fn to_value<T: Serialize + ?Sized>(value: &T) -> Value {
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(_) => Value::Null,
    }
}

#[must_use]
pub fn driver_version_response(
    req_id: u64,
    driver_id: &str,
    driver_version: &str,
    min_core_api: &str,
) -> Value {
    response(
        req_id,
        "driver_version",
        200,
        Some(json!({
            "name": driver_id,
            "version": { "driver": driver_version },
            "min_core_api": min_core_api,
        })),
    )
}

#[must_use]
pub fn driver_metadata_response<T: Serialize>(req_id: u64, metadata: &T) -> Value {
    response(req_id, "driver_metadata", 200, Some(to_value(metadata)))
}

#[must_use]
pub fn available_entities_response<T: Serialize>(req_id: u64, entities: &T) -> Value {
    response(
        req_id,
        "available_entities",
        200,
        Some(json!({ "available_entities": to_value(entities) })),
    )
}

#[must_use]
pub fn entity_states_response(req_id: u64, states: &[EntityState]) -> Value {
    response(req_id, "entity_states", 200, Some(to_value(states)))
}

#[must_use]
pub fn result_response(req_id: u64, code: u16) -> Value {
    response(req_id, "result", code, None)
}

#[must_use]
pub fn device_state_event(state: &str) -> Value {
    event("device_state", "DEVICE", json!({ "state": state }))
}

#[must_use]
pub fn entity_change_event(
    entity_id: &str,
    entity_type: &str,
    attributes: &HashMap<String, Value>,
) -> Value {
    event(
        "entity_change",
        "ENTITY",
        json!({
            "entity_id": entity_id,
            "entity_type": entity_type,
            "attributes": attributes,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_parser_uses_top_level_id() {
        let request = RequestEnvelope::parse(json!({
            "kind": "req",
            "id": 42,
            "msg": "get_driver_version",
        }))
        .unwrap();

        assert_eq!(request.id(), 42);
        assert_eq!(
            request.known_message(),
            Some(IntegrationWsKnownMessage::GetDriverVersion)
        );
    }

    #[test]
    fn result_response_puts_code_at_top_level() {
        let response = result_response(7, 200);

        assert_eq!(response["req_id"], 7);
        assert_eq!(response["code"], 200);
        assert_eq!(response["msg_data"], json!({}));
    }

    #[test]
    fn event_without_id_is_rejected_as_unexpected_kind_not_invalid_envelope() {
        let error = RequestEnvelope::parse(json!({
            "kind": "event",
            "msg": "abort_driver_setup",
            "cat": "DEVICE",
            "msg_data": {
                "error": "TIMEOUT"
            }
        }))
        .unwrap_err();

        assert!(matches!(
            error,
            EnvelopeError::UnexpectedKind(IntegrationWsEnvelopeKind::Event)
        ));
    }

    #[test]
    fn abort_setup_event_extracts_remote_error() {
        let message = json!({
            "kind": "event",
            "msg": "abort_driver_setup",
            "cat": "DEVICE",
            "msg_data": {
                "error": "TIMEOUT"
            }
        });

        assert_eq!(envelope_kind(&message), Some("event"));
        assert_eq!(envelope_message_name(&message), Some("abort_driver_setup"));
        assert_eq!(remote_setup_abort_error(&message), Some("TIMEOUT"));
    }
}
