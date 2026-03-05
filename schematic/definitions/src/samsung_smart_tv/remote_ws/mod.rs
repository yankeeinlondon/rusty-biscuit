//! Samsung Smart TV remote control WebSocket API definition.

mod types;

pub use types::*;

use schematic_define::websocket::{
    ConnectionLifecycle, ConnectionParam, FrameFormat, MessageDirection, MessageSchema, ParamType,
    RequestIdType, WebSocketApi, WebSocketEndpoint, WebSocketRuntimeHints,
};
use schematic_define::{AuthStrategy, Schema};

/// Creates the Samsung Smart TV remote control WebSocket API definition.
///
/// Models the `samsung.remote.control` channel on WSS port 8002, used for
/// sending remote key commands and receiving channel lifecycle events.
///
/// Connection requires a `name` parameter (base64-encoded client name) and
/// optionally accepts a `token` from a previously approved session to bypass
/// the on-TV approval prompt.
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::samsung_smart_tv::remote_ws::define_samsung_smart_tv_remote_ws_api;
///
/// let api = define_samsung_smart_tv_remote_ws_api();
/// assert_eq!(api.name, "SamsungSmartTvRemote");
/// assert_eq!(api.endpoints.len(), 2);
/// ```
#[must_use]
pub fn define_samsung_smart_tv_remote_ws_api() -> WebSocketApi {
    let messages = vec![
        MessageSchema {
            name: "RemoteControlCommand".to_string(),
            direction: MessageDirection::Client,
            schema: Schema::new("SamsungRemoteControlCommand"),
            description: Some(
                "Remote key command (method, key, action). Use `Cmd=Click` for \
                 single presses, `Press`/`Release` for hold gestures."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "ChannelConnectEvent".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("SamsungRemoteConnectEvent"),
            description: Some(
                "Emitted when the channel is successfully connected and authorized. \
                 May include a `token` for future reconnects."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "ChannelUnauthorizedEvent".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("SamsungRemoteUnauthorizedEvent"),
            description: Some(
                "Emitted when channel connection is unauthorized (approval needed or denied)."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "ChannelErrorEvent".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("SamsungRemoteErrorEvent"),
            description: Some("Emitted on channel-level errors.".to_string()),
        },
        MessageSchema {
            name: "ChannelEnvelope".to_string(),
            direction: MessageDirection::Bidirectional,
            schema: Schema::new("SamsungRemoteEnvelope"),
            description: Some(
                "Generic envelope with deferred `RawValue` payload. Route by \
                 inspecting `event` or `method`, then deserialize `data` on demand."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "AppLaunchCommand".to_string(),
            direction: MessageDirection::Client,
            schema: Schema::new("SamsungAppLaunchCommand"),
            description: Some(
                "Launch an application via `ms.channel.emit` with optional deep-link \
                 metadata. Supports both `DEEP_LINK` and `NATIVE_LAUNCH` action types."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "InstalledAppsCommand".to_string(),
            direction: MessageDirection::Client,
            schema: Schema::new("SamsungInstalledAppsCommand"),
            description: Some(
                "Request the list of installed applications via `ms.channel.emit`. \
                 The TV responds with an `ed.installedApp.get` event containing the app list."
                    .to_string(),
            ),
        },
    ];

    let art_mode_messages = vec![
        MessageSchema {
            name: "ArtModeCommand".to_string(),
            direction: MessageDirection::Client,
            schema: Schema::new("SamsungArtModeCommand"),
            description: Some(
                "Art Mode command sent via `ms.channel.emit` with double-encoded \
                 data payload. Supports brightness, artwork selection, and mode control."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "ArtModeEvent".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("SamsungArtModeEvent"),
            description: Some(
                "Art Mode response event. The `data` field contains a double-encoded \
                 JSON string that must be parsed separately."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "ChannelConnectEvent".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("SamsungRemoteConnectEvent"),
            description: Some(
                "Emitted when the Art Mode channel is successfully connected."
                    .to_string(),
            ),
        },
        MessageSchema {
            name: "ChannelEnvelope".to_string(),
            direction: MessageDirection::Bidirectional,
            schema: Schema::new("SamsungRemoteEnvelope"),
            description: Some(
                "Generic envelope with deferred payload for the Art Mode channel."
                    .to_string(),
            ),
        },
    ];

    let endpoints = vec![
        WebSocketEndpoint {
        id: "RemoteControl".to_string(),
        path: "/api/v2/channels/samsung.remote.control".to_string(),
        description: "Samsung remote control channel for key transport and lifecycle events. \
            First connection may trigger an on-TV approval prompt; approved sessions \
            provide a token for subsequent reconnects."
            .to_string(),
        connection_params: vec![
            ConnectionParam {
                name: "name".to_string(),
                param_type: ParamType::String,
                required: true,
                description: Some(
                    "Base64-encoded client name used by Samsung remote channel".to_string(),
                ),
            },
            ConnectionParam {
                name: "token".to_string(),
                param_type: ParamType::String,
                required: false,
                description: Some(
                    "Previously approved remote token to bypass repeated on-TV prompts"
                        .to_string(),
                ),
            },
        ],
        lifecycle: ConnectionLifecycle::default(),
        messages,
        runtime: None,
    },
        WebSocketEndpoint {
            id: "ArtMode".to_string(),
            path: "/api/v2/channels/com.samsung.art-app".to_string(),
            description: "Samsung Art Mode (Frame TV) channel for artwork management, \
                brightness control, and Art Mode on/off. Uses double-encoded JSON in \
                the `data` field of `ms.channel.emit` messages."
                .to_string(),
            connection_params: vec![
                ConnectionParam {
                    name: "name".to_string(),
                    param_type: ParamType::String,
                    required: true,
                    description: Some(
                        "Base64-encoded client name for the Art Mode channel".to_string(),
                    ),
                },
                ConnectionParam {
                    name: "token".to_string(),
                    param_type: ParamType::String,
                    required: false,
                    description: Some(
                        "Previously approved token to bypass on-TV approval prompt".to_string(),
                    ),
                },
            ],
            lifecycle: ConnectionLifecycle::default(),
            messages: art_mode_messages,
            runtime: None,
        },
    ];

    WebSocketApi {
        name: "SamsungSmartTvRemote".to_string(),
        description: "Samsung Smart TV remote-control websocket API (S95C-focused)".to_string(),
        base_url: "wss://192.168.1.1:8002".to_string(),
        docs_url: Some(
            "https://developer.samsung.com/smarttv/develop/extension-libraries/smart-view-sdk/receiver-apps/debugging.html".to_string(),
        ),
        auth: AuthStrategy::None,
        env_auth: vec![],
        endpoints,
        runtime: Some(WebSocketRuntimeHints {
            frame_format: FrameFormat::JsonText,
            supports_reconnect: true,
            request_id_type: RequestIdType::String,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::websocket::FrameFormat;

    #[test]
    fn ws_api_metadata() {
        let api = define_samsung_smart_tv_remote_ws_api();
        assert_eq!(api.name, "SamsungSmartTvRemote");
        assert_eq!(api.base_url, "wss://192.168.1.1:8002");
        assert!(api.docs_url.is_some());
        assert!(matches!(api.auth, AuthStrategy::None));
        assert!(api.env_auth.is_empty());
    }

    #[test]
    fn ws_api_has_runtime_hints() {
        let api = define_samsung_smart_tv_remote_ws_api();
        let hints = api.runtime.as_ref().expect("runtime hints must be set");
        assert_eq!(hints.frame_format, FrameFormat::JsonText);
        assert!(hints.supports_reconnect);
        assert_eq!(hints.request_id_type, RequestIdType::String);
    }

    #[test]
    fn ws_api_has_two_endpoints() {
        let api = define_samsung_smart_tv_remote_ws_api();
        assert_eq!(api.endpoints.len(), 2);
        assert_eq!(api.endpoints[0].id, "RemoteControl");
        assert_eq!(
            api.endpoints[0].path,
            "/api/v2/channels/samsung.remote.control"
        );
        assert_eq!(api.endpoints[1].id, "ArtMode");
        assert_eq!(
            api.endpoints[1].path,
            "/api/v2/channels/com.samsung.art-app"
        );
    }

    #[test]
    fn remote_control_has_name_and_token_params() {
        let api = define_samsung_smart_tv_remote_ws_api();
        let ep = &api.endpoints[0];
        assert_eq!(ep.connection_params.len(), 2);

        let name_param = ep
            .connection_params
            .iter()
            .find(|p| p.name == "name")
            .expect("name param must exist");
        assert!(name_param.required);
        assert_eq!(name_param.param_type, ParamType::String);

        let token_param = ep
            .connection_params
            .iter()
            .find(|p| p.name == "token")
            .expect("token param must exist");
        assert!(!token_param.required);
        assert_eq!(token_param.param_type, ParamType::String);
    }

    #[test]
    fn remote_control_has_seven_messages() {
        let api = define_samsung_smart_tv_remote_ws_api();
        let ep = &api.endpoints[0];
        assert_eq!(ep.messages.len(), 7);

        let names: Vec<&str> = ep.messages.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"RemoteControlCommand"));
        assert!(names.contains(&"ChannelConnectEvent"));
        assert!(names.contains(&"ChannelUnauthorizedEvent"));
        assert!(names.contains(&"ChannelErrorEvent"));
        assert!(names.contains(&"ChannelEnvelope"));
        assert!(names.contains(&"AppLaunchCommand"));
        assert!(names.contains(&"InstalledAppsCommand"));
    }

    #[test]
    fn art_mode_endpoint_metadata() {
        let api = define_samsung_smart_tv_remote_ws_api();
        let ep = &api.endpoints[1];
        assert_eq!(ep.id, "ArtMode");
        assert_eq!(ep.path, "/api/v2/channels/com.samsung.art-app");
        assert_eq!(ep.connection_params.len(), 2);
        assert_eq!(ep.messages.len(), 4);

        let names: Vec<&str> = ep.messages.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"ArtModeCommand"));
        assert!(names.contains(&"ArtModeEvent"));
        assert!(names.contains(&"ChannelConnectEvent"));
        assert!(names.contains(&"ChannelEnvelope"));
    }

    #[test]
    fn message_directions_are_correct() {
        let api = define_samsung_smart_tv_remote_ws_api();
        let ep = &api.endpoints[0];

        let cmd = ep.messages.iter().find(|m| m.name == "RemoteControlCommand").unwrap();
        assert_eq!(cmd.direction, MessageDirection::Client);

        let connect = ep.messages.iter().find(|m| m.name == "ChannelConnectEvent").unwrap();
        assert_eq!(connect.direction, MessageDirection::Server);

        let envelope = ep.messages.iter().find(|m| m.name == "ChannelEnvelope").unwrap();
        assert_eq!(envelope.direction, MessageDirection::Bidirectional);
    }
}
