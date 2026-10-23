//! Unfolded Circle Dock WebSocket model types.
//!
//! Shapes follow the UCD2 AsyncAPI (`specs/unfolded-circle/UCD2-asyncapi.yaml`).
//! The Dock protocol has no nested payload object: a command's parameters and
//! a reply's or event's data are top-level fields beside `type`, `id`/`req_id`,
//! `command`/`msg`, and `code`. The envelopes name those common fields and
//! keep every other top-level field in a JSON map, so no recorded data is
//! dropped and no per-command type is needed.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Dock WebSocket message discriminator (`type` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockWsEnvelopeType {
    /// A command or its reply (`type=dock`).
    Dock,
    /// An asynchronous event (`type=event`).
    Event,
    /// The dock's authentication challenge (`type=auth_required`).
    AuthRequired,
    /// An authentication request or its result (`type=auth`).
    Auth,
}

/// The command and message names the UCD2 AsyncAPI records.
///
/// A request names one in `command`; its reply echoes it in `msg`. Events
/// use `msg` for their own names (`ir_receive`, `port_mode`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockWsKnownMessage {
    /// Authentication result (`type=auth`).
    Authentication,
    /// Application-level ping.
    Ping,
    /// Reply to `ping`.
    Pong,
    /// Get system information; also the reply's `msg`.
    GetSysinfo,
    /// Send an IR code.
    IrSend,
    /// Stop a repeating IR transmission.
    IrStop,
    /// Enable IR learning (system command, and the matching event).
    IrReceiveOn,
    /// Disable IR learning (system command, and the matching event).
    IrReceiveOff,
    /// A learned IR code (event).
    IrReceive,
    /// Status LED: remote charged (system command).
    RemoteCharged,
    /// Status LED: remote battery low (system command).
    RemoteLowbattery,
    /// Status LED: normal operation (system command).
    RemoteNormal,
    /// Blink the status LED for identification (system command).
    Identify,
    /// Reboot the dock (system command).
    Reboot,
    /// Factory-reset the dock (system command).
    Reset,
    /// Change the dock's configuration.
    SetConfig,
    /// Set LED brightness.
    SetBrightness,
    /// Set speaker volume.
    SetVolume,
    /// Configure logging.
    SetLogging,
    /// Get one external port's mode.
    GetPortMode,
    /// Set one external port's mode.
    SetPortMode,
    /// Get every external port's mode.
    GetPortModes,
    /// Set an external port's trigger.
    SetPortTrigger,
    /// Get an external port's trigger.
    GetPortTrigger,
    /// An external port's mode changed (event).
    PortMode,
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

/// Dock command request (`type=dock`), for example
/// `{"type":"dock","id":1,"command":"ir_send","code":"4;0x10;12;0","format":"hex"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockWsRequestEnvelope {
    /// Envelope type discriminator (`dock`).
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Client request id, echoed as the reply's `req_id`. A correlated
    /// request overwrites it with an id the client allocates.
    pub id: u64,
    /// The command to run.
    pub command: DockWsMessageName,
    /// The command's parameters, written as top-level fields.
    #[serde(flatten)]
    pub params: Map<String, Value>,
}

/// Dock reply (`type=dock`), for example the `get_sysinfo` reply
/// `{"type":"dock","req_id":2,"msg":"get_sysinfo","code":200,"name":"My dock",…}`.
///
/// Also the shape of `pong`, which carries neither `req_id` nor `code`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockWsResponseEnvelope {
    /// Envelope type discriminator (`dock`).
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// The request's `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
    /// The command this replies to.
    pub msg: DockWsMessageName,
    /// Status code, following HTTP status code conventions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    /// Set when the dock restarts after this message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reboot: Option<bool>,
    /// The reply's result fields (`name`, `model`, `version`, …).
    #[serde(flatten)]
    pub data: Map<String, Value>,
}

/// Dock event (`type=event`), for example
/// `{"type":"event","msg":"ir_receive","ir_code":"4;0x10;12;0"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockWsEventEnvelope {
    /// Envelope type discriminator (`event`).
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Event name.
    pub msg: DockWsMessageName,
    /// The event's fields (`ir_code`, `format`, …).
    #[serde(flatten)]
    pub data: Map<String, Value>,
}

/// Dock auth request (`{"type": "auth", "token": …}`) for post-connect
/// authentication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DockWsAuthMessage {
    /// Message type discriminator (`auth`).
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Auth token.
    pub token: String,
}

/// Dock authentication challenge (`type=auth_required`), sent right after
/// the connection opens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DockWsAuthRequired {
    /// Message type discriminator (`auth_required`).
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Dock model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Dock hardware revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Dock firmware version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Dock authentication result (`type=auth`, `msg=authentication`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DockWsAuthenticationMessage {
    /// Message type discriminator (`auth`).
    #[serde(rename = "type")]
    pub type_name: DockWsEnvelopeType,
    /// Message id/name (`authentication`).
    pub msg: DockWsKnownMessage,
    /// Result: `200` authenticated; `401` rejected, and the dock closes the
    /// connection.
    pub code: i32,
}

fn parse_fields<T: serde::de::DeserializeOwned>(fields: &Map<String, Value>) -> Result<T, serde_json::Error> {
    T::deserialize(Value::Object(fields.clone()))
}

impl DockWsResponseEnvelope {
    /// Deserialize the reply's result fields into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if the fields do not match `T`.
    pub fn parse_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        parse_fields(&self.data)
    }
}

impl DockWsEventEnvelope {
    /// Deserialize the event's fields into a concrete type.
    ///
    /// ## Errors
    ///
    /// Returns a [`serde_json::Error`] if the fields do not match `T`.
    pub fn parse_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        parse_fields(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Recorded examples from `specs/unfolded-circle/UCD2-asyncapi.yaml`.
    const SYSINFO_REQUEST: &str = r#"{"type":"dock","id":2,"command":"get_sysinfo"}"#;
    const IR_SEND_REQUEST: &str =
        r#"{"type":"dock","id":1,"command":"ir_send","code":"4;0x10;12;0","format":"hex","int_side":true}"#;
    const SYSINFO_RESPONSE: &str = r#"{"type":"dock","req_id":2,"msg":"get_sysinfo","code":200,"name":"My dock","hostname":"UC-Dock-1234","model":"UCD2","revision":"5.4","version":"0.4.0","serial":"0042","ir_learning":false}"#;
    const PONG: &str = r#"{"type":"dock","msg":"pong"}"#;
    const IR_RECEIVE_EVENT: &str = r#"{"type":"event","msg":"ir_receive","ir_code":"4;0x10;12;0"}"#;

    fn json_of<T: Serialize>(value: &T) -> Value {
        serde_json::to_value(value).unwrap()
    }

    fn raw(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn envelope_type_wire_names() {
        let cases = [
            (DockWsEnvelopeType::Dock, "dock"),
            (DockWsEnvelopeType::Event, "event"),
            (DockWsEnvelopeType::AuthRequired, "auth_required"),
            (DockWsEnvelopeType::Auth, "auth"),
        ];
        for (variant, wire) in cases {
            assert_eq!(json_of(&variant), json!(wire), "{variant:?}");
        }
        assert!(serde_json::from_str::<DockWsEnvelopeType>(r#""req""#).is_err());
    }

    #[test]
    fn known_message_uses_recorded_command_names() {
        let cases = [
            (DockWsKnownMessage::GetSysinfo, "get_sysinfo"),
            (DockWsKnownMessage::RemoteLowbattery, "remote_lowbattery"),
            (DockWsKnownMessage::IrReceiveOn, "ir_receive_on"),
            (DockWsKnownMessage::GetPortModes, "get_port_modes"),
            (DockWsKnownMessage::PortMode, "port_mode"),
        ];
        for (variant, wire) in cases {
            assert_eq!(json_of(&variant), json!(wire), "{variant:?}");
        }
    }

    #[test]
    fn message_name_passes_unknown_names_through() {
        let name: DockWsMessageName = serde_json::from_str(r#""some_future_msg""#).unwrap();
        assert_eq!(name, DockWsMessageName::Other("some_future_msg".to_string()));
        let name: DockWsMessageName = serde_json::from_str(r#""ir_send""#).unwrap();
        assert_eq!(name, DockWsMessageName::Known(DockWsKnownMessage::IrSend));
    }

    #[test]
    fn recorded_requests_round_trip_with_top_level_params() {
        let request: DockWsRequestEnvelope = serde_json::from_str(SYSINFO_REQUEST).unwrap();
        assert_eq!(request.type_name, DockWsEnvelopeType::Dock);
        assert_eq!(request.id, 2);
        assert_eq!(request.command, DockWsMessageName::Known(DockWsKnownMessage::GetSysinfo));
        assert!(request.params.is_empty());
        assert_eq!(json_of(&request), raw(SYSINFO_REQUEST));

        let request: DockWsRequestEnvelope = serde_json::from_str(IR_SEND_REQUEST).unwrap();
        assert_eq!(request.command, DockWsMessageName::Known(DockWsKnownMessage::IrSend));
        assert_eq!(request.params["code"], "4;0x10;12;0");
        assert_eq!(request.params["int_side"], true);
        assert_eq!(json_of(&request), raw(IR_SEND_REQUEST));
    }

    #[test]
    fn recorded_sysinfo_response_retains_its_result_fields() {
        #[derive(Debug, Deserialize)]
        struct SystemInfo {
            name: String,
            model: String,
            version: String,
            ir_learning: bool,
        }

        let response: DockWsResponseEnvelope = serde_json::from_str(SYSINFO_RESPONSE).unwrap();
        assert_eq!(response.type_name, DockWsEnvelopeType::Dock);
        assert_eq!(response.req_id, Some(2));
        assert_eq!(response.msg, DockWsMessageName::Known(DockWsKnownMessage::GetSysinfo));
        assert_eq!(response.code, Some(200));
        assert_eq!(response.data["hostname"], "UC-Dock-1234");
        assert!(!response.data.contains_key("type") && !response.data.contains_key("req_id"));

        let info: SystemInfo = response.parse_data().unwrap();
        assert_eq!((info.name.as_str(), info.model.as_str(), info.version.as_str()), ("My dock", "UCD2", "0.4.0"));
        assert!(!info.ir_learning);
        assert_eq!(json_of(&response), raw(SYSINFO_RESPONSE));
    }

    #[test]
    fn pong_decodes_as_a_response_without_id_or_code() {
        let pong: DockWsResponseEnvelope = serde_json::from_str(PONG).unwrap();
        assert_eq!(pong.msg, DockWsMessageName::Known(DockWsKnownMessage::Pong));
        assert_eq!((pong.req_id, pong.code), (None, None));
        assert_eq!(json_of(&pong), raw(PONG));
    }

    #[test]
    fn recorded_ir_receive_event_retains_its_code() {
        let event: DockWsEventEnvelope = serde_json::from_str(IR_RECEIVE_EVENT).unwrap();
        assert_eq!(event.type_name, DockWsEnvelopeType::Event);
        assert_eq!(event.msg, DockWsMessageName::Known(DockWsKnownMessage::IrReceive));
        assert_eq!(event.data["ir_code"], "4;0x10;12;0");

        #[derive(Deserialize)]
        struct IrReceive {
            ir_code: String,
        }
        assert_eq!(event.parse_data::<IrReceive>().unwrap().ir_code, "4;0x10;12;0");
        assert_eq!(json_of(&event), raw(IR_RECEIVE_EVENT));
    }

    #[test]
    fn parse_data_reports_a_mismatched_target() {
        let event: DockWsEventEnvelope = serde_json::from_str(IR_RECEIVE_EVENT).unwrap();
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Wrong {
            port: u8,
        }
        assert!(event.parse_data::<Wrong>().is_err());
    }

    /// The three authentication frames as the UCD2 AsyncAPI documents them.
    #[test]
    fn recorded_authentication_frames_round_trip() {
        let challenge_text = r#"{"type":"auth_required","model":"UCD2","revision":"5.4","version":"0.6.0"}"#;
        let challenge: DockWsAuthRequired = serde_json::from_str(challenge_text).unwrap();
        assert_eq!(challenge.type_name, DockWsEnvelopeType::AuthRequired);
        assert_eq!(challenge.model.as_deref(), Some("UCD2"));
        assert_eq!(json_of(&challenge), raw(challenge_text));

        let request_text = r#"{"type":"auth","token":"1234"}"#;
        let request: DockWsAuthMessage = serde_json::from_str(request_text).unwrap();
        assert_eq!(request.type_name, DockWsEnvelopeType::Auth);
        assert_eq!(request.token, "1234");
        assert_eq!(json_of(&request), raw(request_text));

        let result_text = r#"{"type":"auth","msg":"authentication","code":401}"#;
        let result: DockWsAuthenticationMessage = serde_json::from_str(result_text).unwrap();
        assert_eq!(result.msg, DockWsKnownMessage::Authentication);
        assert_eq!(result.code, 401);
        assert_eq!(json_of(&result), raw(result_text));
    }
}
