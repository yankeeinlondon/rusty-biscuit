use std::collections::HashMap;

use schematic_schema::unfolded_circle_integration_ws::{
    IntegrationClientMessage, IntegrationServerMessage, IntegrationWsEnvelopeKind,
    IntegrationWsEventEnvelope, IntegrationWsKnownMessage, IntegrationWsMessageName,
    IntegrationWsRequestEnvelope, IntegrationWsResponseEnvelope, WsConnectionContext,
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

/// Re-encodes a decoded inbound message as the JSON value the handlers route on.
///
/// The host hands handlers a typed [`IntegrationClientMessage`]; the handlers
/// route on the request/event JSON (see [`IntegrationRequest::parse`] and
/// [`remote_setup_abort_error`]).
///
/// ## Errors
///
/// Returns an error if the message does not serialize.
pub fn client_message_value(message: &IntegrationClientMessage) -> Result<Value, EnvelopeError> {
    Ok(serde_json::to_value(message)?)
}

/// Converts a handler-built JSON response or event into the typed host reply.
///
/// The handlers build their replies as JSON; the host boundary is typed. A
/// `kind: "event"` value becomes an [`IntegrationServerMessage::EventEnvelope`],
/// anything else an [`IntegrationServerMessage::ResponseEnvelope`]. The
/// envelopes' `msg_data` is a `RawValue`, which `serde_json::from_value`
/// cannot produce, so the value is decoded from its text.
///
/// ## Errors
///
/// Returns an error if the value is not a valid response or event envelope.
pub fn server_message(value: Value) -> Result<IntegrationServerMessage, EnvelopeError> {
    let text = value.to_string();
    Ok(if envelope_kind(&value) == Some("event") {
        IntegrationServerMessage::EventEnvelope(serde_json::from_str::<IntegrationWsEventEnvelope>(
            &text,
        )?)
    } else {
        IntegrationServerMessage::ResponseEnvelope(serde_json::from_str::<
            IntegrationWsResponseEnvelope,
        >(&text)?)
    })
}

/// The successful `authentication` response the Remote expects right after
/// connecting: `{"kind":"resp","req_id":0,"msg":"authentication","code":200,"msg_data":{}}`.
///
/// Sent as a response envelope because the declared `Authentication` payload
/// carries neither `req_id` nor `code`.
#[must_use]
pub fn authentication_response() -> IntegrationServerMessage {
    IntegrationServerMessage::ResponseEnvelope(IntegrationWsResponseEnvelope {
        kind: IntegrationWsEnvelopeKind::Resp,
        req_id: 0,
        msg: IntegrationWsMessageName::Known(IntegrationWsKnownMessage::Authentication),
        code: 200,
        msg_data: serde_json::value::to_raw_value(&json!({})).ok(),
    })
}

/// The message an integration's `WsHandler::on_authenticated` sends.
///
/// Returns [`authentication_response`] when the upgrade checked no credential
/// (no `UCR_INTEGRATION_TOKEN` configured) and `None` when the host verified
/// one, matching what the host itself sent before the hook existed.
#[must_use]
pub fn authentication_greeting(context: &WsConnectionContext) -> Option<IntegrationServerMessage> {
    (!context.credential_verified()).then(authentication_response)
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
    fn authentication_response_matches_the_former_host_envelope() {
        let bytes = serde_json::to_string(&authentication_response()).unwrap();

        assert_eq!(
            bytes,
            r#"{"kind":"resp","req_id":0,"msg":"authentication","code":200,"msg_data":{}}"#
        );
    }

    #[test]
    fn authentication_greeting_is_sent_when_no_credential_was_checked() {
        use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleIntegrationEventHub;

        let context = WsConnectionContext::new(1, UnfoldedCircleIntegrationEventHub::new());
        assert!(!context.credential_verified());

        let greeting = authentication_greeting(&context).expect("greeting");
        assert_eq!(
            serde_json::to_value(greeting).unwrap(),
            serde_json::to_value(authentication_response()).unwrap()
        );
    }

    #[test]
    fn server_message_round_trips_responses_and_events() {
        let response = result_response(7, 404);
        let event = device_state_event("CONNECTED");

        let typed_response = server_message(response.clone()).unwrap();
        let typed_event = server_message(event.clone()).unwrap();

        assert!(matches!(
            typed_response,
            IntegrationServerMessage::ResponseEnvelope(_)
        ));
        assert_eq!(serde_json::to_value(typed_response).unwrap(), response);
        assert!(matches!(
            typed_event,
            IntegrationServerMessage::EventEnvelope(_)
        ));
        assert_eq!(serde_json::to_value(typed_event).unwrap(), event);
    }

    #[test]
    fn client_message_value_restores_request_json() {
        use crate::test_fixtures::{client_message, request};

        let value = request(3, "get_driver_version", json!({"a": 1}));
        let restored = client_message_value(&client_message(value.clone())).unwrap();

        assert_eq!(restored, value);
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
