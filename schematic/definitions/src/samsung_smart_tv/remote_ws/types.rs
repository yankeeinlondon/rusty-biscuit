//! Samsung Smart TV remote control WebSocket model types.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Samsung remote control method identifiers.
///
/// Known methods for the `samsung.remote.control` channel. Unknown methods
/// from future firmware are captured via the `Other` fallback variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SamsungRemoteMethod {
    /// Known fixed method identifiers.
    Known(SamsungRemoteKnownMethod),
    /// Future or firmware-specific method identifiers.
    Other(String),
}

/// Known method discriminants for the Samsung remote channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamsungRemoteKnownMethod {
    /// Remote control key transport method.
    #[serde(rename = "ms.remote.control")]
    MsRemoteControl,
    /// Channel event emit method.
    #[serde(rename = "ms.channel.emit")]
    MsChannelEmit,
}

/// Samsung remote event name discriminants.
///
/// Events received from the TV on the remote control channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SamsungRemoteEventName {
    /// Known fixed event identifiers.
    Known(SamsungRemoteKnownEvent),
    /// Future or firmware-specific event identifiers.
    Other(String),
}

/// Known event discriminants for the Samsung remote channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamsungRemoteKnownEvent {
    /// Channel successfully connected and authorized.
    #[serde(rename = "ms.channel.connect")]
    MsChannelConnect,
    /// Channel connection unauthorized (approval needed or denied).
    #[serde(rename = "ms.channel.unauthorized")]
    MsChannelUnauthorized,
    /// Channel error event.
    #[serde(rename = "ms.error")]
    MsError,
    /// App launch event emitted through `ms.channel.emit`.
    #[serde(rename = "ed.apps.launch")]
    EdAppsLaunch,
    /// Installed apps list event emitted through `ms.channel.emit`.
    #[serde(rename = "ed.installedApp.get")]
    EdInstalledAppGet,
}

/// Remote type discriminant for Samsung remote commands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SamsungRemoteType {
    /// Known remote type identifiers.
    Known(SamsungRemoteKnownType),
    /// Future or firmware-specific remote type identifiers.
    Other(String),
}

/// Known remote type variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamsungRemoteKnownType {
    /// Standard remote key send.
    SendRemoteKey,
}

/// Remote command action (press semantics).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SamsungRemoteCommandAction {
    /// Known command actions.
    Known(SamsungRemoteKnownAction),
    /// Future or firmware-specific actions.
    Other(String),
}

/// Known remote command action variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamsungRemoteKnownAction {
    /// Single key press and release.
    Click,
    /// Key press (hold down).
    Press,
    /// Key release (after press).
    Release,
}

/// Samsung remote key identifiers.
///
/// Common keys include `KEY_VOLUP`, `KEY_VOLDOWN`, `KEY_MUTE`, `KEY_POWER`,
/// `KEY_HOME`, `KEY_MENU`, `KEY_RETURN`, `KEY_UP`, `KEY_DOWN`, `KEY_LEFT`,
/// `KEY_RIGHT`, `KEY_ENTER`, and numeric `KEY_0` through `KEY_9`.
///
/// This is kept as a `String` because the key space is large and
/// firmware-variable.
pub type SamsungRemoteKey = String;

/// Remote control command sent to the TV.
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::samsung_smart_tv::remote_ws::{
///     SamsungRemoteControlCommand, SamsungRemoteControlParams,
///     SamsungRemoteMethod, SamsungRemoteKnownMethod,
///     SamsungRemoteCommandAction, SamsungRemoteKnownAction,
///     SamsungRemoteType, SamsungRemoteKnownType,
/// };
///
/// let cmd = SamsungRemoteControlCommand {
///     method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsRemoteControl),
///     params: SamsungRemoteControlParams {
///         cmd: SamsungRemoteCommandAction::Known(SamsungRemoteKnownAction::Click),
///         data_of_cmd: "KEY_VOLUP".to_string(),
///         option: "false".to_string(),
///         type_of_remote: SamsungRemoteType::Known(SamsungRemoteKnownType::SendRemoteKey),
///     },
/// };
///
/// let json = serde_json::to_string(&cmd).unwrap();
/// assert!(json.contains("\"Cmd\""));
/// assert!(json.contains("\"DataOfCmd\""));
/// assert!(json.contains("\"TypeOfRemote\""));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungRemoteControlCommand {
    /// Method identifier for the command.
    pub method: SamsungRemoteMethod,
    /// Command parameters.
    pub params: SamsungRemoteControlParams,
}

/// Parameters for a Samsung remote control command.
///
/// Field names are renamed to match the Samsung wire format (`Cmd`,
/// `DataOfCmd`, `Option`, `TypeOfRemote`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungRemoteControlParams {
    /// Command action (click, press, release).
    #[serde(rename = "Cmd")]
    pub cmd: SamsungRemoteCommandAction,

    /// Key identifier (e.g., `"KEY_VOLUP"`, `"KEY_POWER"`).
    #[serde(rename = "DataOfCmd")]
    pub data_of_cmd: SamsungRemoteKey,

    /// Option string. Samsung payloads frequently use string booleans
    /// (`"false"`) instead of JSON booleans, so this remains `String`.
    #[serde(rename = "Option")]
    pub option: String,

    /// Remote type discriminant.
    #[serde(rename = "TypeOfRemote")]
    pub type_of_remote: SamsungRemoteType,
}

/// Bidirectional envelope for the Samsung remote channel.
///
/// Uses [`RawValue`] for the `data` field to defer payload parsing until
/// the event/method is identified. Route by inspecting `event` or `method`
/// first, then deserialize `data` on demand.
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::samsung_smart_tv::remote_ws::SamsungRemoteEnvelope;
///
/// let json = r#"{"event":"ms.channel.connect","data":{"token":"abc123"}}"#;
/// let envelope: SamsungRemoteEnvelope = serde_json::from_str(json).unwrap();
/// assert!(envelope.event.is_some());
/// assert!(envelope.data.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamsungRemoteEnvelope {
    /// Event discriminant (present on server-originated messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<SamsungRemoteEventName>,

    /// Method discriminant (present on client-originated or method-bearing messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<SamsungRemoteMethod>,

    /// Deferred payload. Parse after routing by event/method name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Box<RawValue>>,
}

/// Channel connect event data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungRemoteConnectEvent {
    /// Event discriminant (`ms.channel.connect`).
    pub event: SamsungRemoteEventName,

    /// Connection data (may include approval token).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<SamsungRemoteConnectData>,
}

/// Data payload for a channel connect event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungRemoteConnectData {
    /// Approved remote token for future reconnects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Channel unauthorized event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungRemoteUnauthorizedEvent {
    /// Event discriminant (`ms.channel.unauthorized`).
    pub event: SamsungRemoteEventName,
}

/// Channel error event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamsungRemoteErrorEvent {
    /// Event discriminant (`ms.error`).
    pub event: SamsungRemoteEventName,

    /// Error payload (firmware-variable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Box<RawValue>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_command_serializes_with_correct_key_casing() {
        let cmd = SamsungRemoteControlCommand {
            method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsRemoteControl),
            params: SamsungRemoteControlParams {
                cmd: SamsungRemoteCommandAction::Known(SamsungRemoteKnownAction::Click),
                data_of_cmd: "KEY_VOLUP".to_string(),
                option: "false".to_string(),
                type_of_remote: SamsungRemoteType::Known(SamsungRemoteKnownType::SendRemoteKey),
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains(r#""Cmd""#), "Expected Cmd key, got: {json}");
        assert!(
            json.contains(r#""DataOfCmd""#),
            "Expected DataOfCmd key, got: {json}"
        );
        assert!(
            json.contains(r#""Option""#),
            "Expected Option key, got: {json}"
        );
        assert!(
            json.contains(r#""TypeOfRemote""#),
            "Expected TypeOfRemote key, got: {json}"
        );
    }

    #[test]
    fn remote_command_roundtrips() {
        let cmd = SamsungRemoteControlCommand {
            method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsRemoteControl),
            params: SamsungRemoteControlParams {
                cmd: SamsungRemoteCommandAction::Known(SamsungRemoteKnownAction::Click),
                data_of_cmd: "KEY_HOME".to_string(),
                option: "false".to_string(),
                type_of_remote: SamsungRemoteType::Known(SamsungRemoteKnownType::SendRemoteKey),
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let restored: SamsungRemoteControlCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, restored);
    }

    #[test]
    fn envelope_with_deferred_raw_value() {
        let json = r#"{"event":"ms.channel.connect","data":{"token":"abc123","id":"client-42"}}"#;
        let envelope: SamsungRemoteEnvelope = serde_json::from_str(json).unwrap();

        assert!(envelope.event.is_some());
        assert!(envelope.method.is_none());
        let raw = envelope.data.as_ref().unwrap();
        assert!(raw.get().contains("abc123"));
    }

    #[test]
    fn envelope_without_data() {
        let json = r#"{"event":"ms.channel.unauthorized"}"#;
        let envelope: SamsungRemoteEnvelope = serde_json::from_str(json).unwrap();
        assert!(envelope.event.is_some());
        assert!(envelope.data.is_none());
    }

    #[test]
    fn known_method_serializes_correctly() {
        let method = SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsRemoteControl);
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#""ms.remote.control""#);
    }

    #[test]
    fn known_method_channel_emit_serializes_correctly() {
        let method = SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsChannelEmit);
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#""ms.channel.emit""#);
    }

    #[test]
    fn unknown_method_passthrough() {
        let json = r#""ms.future.method""#;
        let method: SamsungRemoteMethod = serde_json::from_str(json).unwrap();
        assert!(matches!(method, SamsungRemoteMethod::Other(ref s) if s == "ms.future.method"));
    }

    #[test]
    fn known_event_serializes_correctly() {
        let event =
            SamsungRemoteEventName::Known(SamsungRemoteKnownEvent::MsChannelConnect);
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, r#""ms.channel.connect""#);
    }

    #[test]
    fn known_event_app_control_variants_serialize_correctly() {
        let launch = SamsungRemoteEventName::Known(SamsungRemoteKnownEvent::EdAppsLaunch);
        let installed = SamsungRemoteEventName::Known(SamsungRemoteKnownEvent::EdInstalledAppGet);
        assert_eq!(serde_json::to_string(&launch).unwrap(), r#""ed.apps.launch""#);
        assert_eq!(
            serde_json::to_string(&installed).unwrap(),
            r#""ed.installedApp.get""#
        );
    }

    #[test]
    fn unknown_event_passthrough() {
        let json = r#""ms.channel.future""#;
        let event: SamsungRemoteEventName = serde_json::from_str(json).unwrap();
        assert!(
            matches!(event, SamsungRemoteEventName::Other(ref s) if s == "ms.channel.future")
        );
    }

    #[test]
    fn connect_event_with_token() {
        let json = r#"{"event":"ms.channel.connect","data":{"token":"tok_abc"}}"#;
        let event: SamsungRemoteConnectEvent = serde_json::from_str(json).unwrap();
        assert_eq!(
            event.data.as_ref().and_then(|d| d.token.as_deref()),
            Some("tok_abc")
        );
    }
}
