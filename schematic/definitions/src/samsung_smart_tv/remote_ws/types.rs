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
    /// Art Mode request/response event on the `com.samsung.art-app` channel.
    #[serde(rename = "art_app_request")]
    ArtAppRequest,
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

    /// Client identifier assigned by the TV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// List of currently connected clients.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clients: Option<Vec<SamsungConnectedClient>>,
}

/// A client currently connected to the TV's remote channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungConnectedClient {
    /// Client connection identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Client's device name.
    #[serde(rename = "deviceName", skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,

    /// Whether this client is the host controller.
    #[serde(rename = "isHost", skip_serializing_if = "Option::is_none")]
    pub is_host: Option<bool>,
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

// ── App Launch (ms.channel.emit) ────────────────────────────────────

/// Action type for app launch commands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamsungAppLaunchActionType {
    /// Deep link into an app with metadata.
    #[serde(rename = "DEEP_LINK")]
    DeepLink,
    /// Native launch without metadata.
    #[serde(rename = "NATIVE_LAUNCH")]
    NativeLaunch,
}

/// Data payload for an app launch emit command.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungAppLaunchData {
    /// Launch action type.
    pub action_type: SamsungAppLaunchActionType,

    /// Application package ID.
    #[serde(rename = "appId")]
    pub app_id: String,

    /// Optional deep-link metadata tag.
    #[serde(rename = "metaTag", skip_serializing_if = "Option::is_none")]
    pub meta_tag: Option<String>,
}

/// Parameters for an app launch emit command.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungAppLaunchParams {
    /// Event name (`"ed.apps.launch"`).
    pub event: String,
    /// Target (`"host"`).
    pub to: String,
    /// App launch payload.
    pub data: SamsungAppLaunchData,
}

/// Top-level app launch command sent via `ms.channel.emit`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungAppLaunchCommand {
    /// Method (`ms.channel.emit`).
    pub method: SamsungRemoteMethod,
    /// Emit parameters.
    pub params: SamsungAppLaunchParams,
}

// ── Installed Apps (ms.channel.emit) ────────────────────────────────

/// Parameters for requesting the installed apps list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungInstalledAppsParams {
    /// Event name (`"ed.installedApp.get"`).
    pub event: String,
    /// Target (`"host"`).
    pub to: String,
}

/// Top-level command to request installed apps list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungInstalledAppsCommand {
    /// Method (`ms.channel.emit`).
    pub method: SamsungRemoteMethod,
    /// Emit parameters.
    pub params: SamsungInstalledAppsParams,
}

// ── Art Mode (com.samsung.art-app channel) ──────────────────────────

/// Art mode request type discriminants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamsungArtModeRequest {
    /// Query the Art Mode API version.
    #[serde(rename = "get_api_version")]
    GetApiVersion,
    /// Query current Art Mode on/off state.
    #[serde(rename = "get_artmode_status")]
    GetArtModeStatus,
    /// Set Art Mode on or off.
    #[serde(rename = "set_artmode_status")]
    SetArtModeStatus,
    /// Get the currently displayed artwork.
    #[serde(rename = "get_current_artwork")]
    GetCurrentArtwork,
    /// List available artwork content.
    #[serde(rename = "get_content_list")]
    GetContentList,
    /// Select artwork to display.
    #[serde(rename = "select_image")]
    SelectImage,
    /// Upload a new image.
    #[serde(rename = "send_image")]
    SendImage,
    /// Get display brightness setting.
    #[serde(rename = "get_brightness")]
    GetBrightness,
    /// Set display brightness.
    #[serde(rename = "set_brightness")]
    SetBrightness,
}

/// Inner data payload for Art Mode commands (before double-encoding).
///
/// Art Mode uses double-encoded JSON: the `data` field in `ms.channel.emit`
/// params is a JSON *string*, not a nested object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SamsungArtModeData {
    /// Request type discriminant.
    pub request: SamsungArtModeRequest,
    /// Art Mode session identifier.
    pub id: String,

    /// Additional request-specific key/value pairs (e.g., `"value": "on"`).
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Emit parameters for an Art Mode command with double-encoded data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungArtModeEmitParams {
    /// Event name (always `"art_app_request"`).
    pub event: String,
    /// Target (always `"host"`).
    pub to: String,
    /// Double-encoded JSON string of [`SamsungArtModeData`].
    pub data: String,
}

/// Top-level Art Mode command sent via `ms.channel.emit`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungArtModeCommand {
    /// Method (`ms.channel.emit`).
    pub method: SamsungRemoteMethod,
    /// Emit parameters with double-encoded data.
    pub params: SamsungArtModeEmitParams,
}

/// Response event from the Art Mode channel (generic — data is double-encoded).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungArtModeEvent {
    /// Event name (e.g., `"art_app_request"`, `"d2d_service_message"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Method discriminant (present on some responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Double-encoded JSON string of the response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Parsed Art Mode status response.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungArtModeStatus {
    /// Art Mode state (e.g., `"on"`, `"off"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// A single artwork item from the TV's content list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamsungArtworkItem {
    /// Content identifier for selecting this artwork.
    #[serde(rename = "content_id", skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    /// Category label (e.g., `"MY-C0002"` for user photos).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Additional metadata.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

/// A single installed application from the TV's app list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamsungInstalledApp {
    /// Application package ID.
    #[serde(rename = "appId")]
    pub app_id: String,

    /// Application display name.
    #[serde(default)]
    pub name: String,

    /// Application type (e.g., `2` for user-installed).
    #[serde(rename = "app_type", skip_serializing_if = "Option::is_none")]
    pub app_type: Option<serde_json::Value>,

    /// Additional fields not covered by typed members.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
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

    #[test]
    fn connect_data_with_id_and_clients() {
        let json = r#"{
            "event": "ms.channel.connect",
            "data": {
                "token": "tok_abc",
                "id": "client-42",
                "clients": [
                    {"id": "c1", "deviceName": "HomeServer", "isHost": true},
                    {"id": "c2", "deviceName": "Phone", "isHost": false}
                ]
            }
        }"#;
        let event: SamsungRemoteConnectEvent = serde_json::from_str(json).unwrap();
        let data = event.data.as_ref().unwrap();
        assert_eq!(data.id.as_deref(), Some("client-42"));
        let clients = data.clients.as_ref().unwrap();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].device_name.as_deref(), Some("HomeServer"));
        assert_eq!(clients[0].is_host, Some(true));
        assert_eq!(clients[1].device_name.as_deref(), Some("Phone"));
        assert_eq!(clients[1].is_host, Some(false));
    }

    #[test]
    fn app_launch_command_serializes() {
        let cmd = SamsungAppLaunchCommand {
            method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsChannelEmit),
            params: SamsungAppLaunchParams {
                event: "ed.apps.launch".to_string(),
                to: "host".to_string(),
                data: SamsungAppLaunchData {
                    action_type: SamsungAppLaunchActionType::DeepLink,
                    app_id: "111299001912".to_string(),
                    meta_tag: Some("https://youtube.com/watch?v=abc".to_string()),
                },
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains(r#""ms.channel.emit""#));
        assert!(json.contains(r#""ed.apps.launch""#));
        assert!(json.contains(r#""appId":"111299001912""#));
        assert!(json.contains(r#""metaTag""#));
        assert!(json.contains("DEEP_LINK"));
    }

    #[test]
    fn installed_apps_command_serializes() {
        let cmd = SamsungInstalledAppsCommand {
            method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsChannelEmit),
            params: SamsungInstalledAppsParams {
                event: "ed.installedApp.get".to_string(),
                to: "host".to_string(),
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains(r#""ms.channel.emit""#));
        assert!(json.contains(r#""ed.installedApp.get""#));
        assert!(json.contains(r#""host""#));
    }

    #[test]
    fn installed_app_deserializes() {
        let json = r#"{"appId":"111299001912","name":"YouTube","app_type":2,"icon":"/path/icon.png"}"#;
        let app: SamsungInstalledApp = serde_json::from_str(json).unwrap();
        assert_eq!(app.app_id, "111299001912");
        assert_eq!(app.name, "YouTube");
        assert!(app.extra.contains_key("icon"));
    }

    #[test]
    fn art_mode_request_variants_serialize() {
        assert_eq!(
            serde_json::to_string(&SamsungArtModeRequest::GetArtModeStatus).unwrap(),
            r#""get_artmode_status""#
        );
        assert_eq!(
            serde_json::to_string(&SamsungArtModeRequest::SetArtModeStatus).unwrap(),
            r#""set_artmode_status""#
        );
        assert_eq!(
            serde_json::to_string(&SamsungArtModeRequest::GetBrightness).unwrap(),
            r#""get_brightness""#
        );
        assert_eq!(
            serde_json::to_string(&SamsungArtModeRequest::SelectImage).unwrap(),
            r#""select_image""#
        );
    }

    #[test]
    fn art_mode_command_serializes_with_double_encoding() {
        let inner = SamsungArtModeData {
            request: SamsungArtModeRequest::GetArtModeStatus,
            id: "art-session-1".to_string(),
            extra: std::collections::BTreeMap::new(),
        };
        let data_str = serde_json::to_string(&inner).unwrap();

        let cmd = SamsungArtModeCommand {
            method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsChannelEmit),
            params: SamsungArtModeEmitParams {
                event: "art_app_request".to_string(),
                to: "host".to_string(),
                data: data_str.clone(),
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains(r#""ms.channel.emit""#));
        assert!(json.contains(r#""art_app_request""#));
        // data field is a string (double-encoded), not a nested object
        assert!(json.contains(&format!(r#""data":"{}""#, data_str.replace('"', r#"\""#))));
    }

    #[test]
    fn art_mode_event_deserializes() {
        let json = r#"{"event":"d2d_service_message","data":"{\"request\":\"get_artmode_status\",\"value\":\"on\"}"}"#;
        let event: SamsungArtModeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event.as_deref(), Some("d2d_service_message"));
        assert!(event.data.is_some());
        // The data is a string, verify we can parse it
        let data_str = event.data.unwrap();
        assert!(data_str.contains("get_artmode_status"));
    }

    #[test]
    fn art_app_request_event_known() {
        let event = SamsungRemoteEventName::Known(SamsungRemoteKnownEvent::ArtAppRequest);
        assert_eq!(serde_json::to_string(&event).unwrap(), r#""art_app_request""#);
    }
}
