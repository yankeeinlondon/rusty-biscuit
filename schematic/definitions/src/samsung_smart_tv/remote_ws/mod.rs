//! Samsung Smart TV remote control WebSocket API definition.

mod types;

pub use types::*;

use schematic_define::websocket::{
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, EndpointParams, EnvMapping, ParamDef, QueryParamType, Schema};

fn message(name: &str, direction: MessageDirection, schema: &str, description: &str) -> MessageSchema {
    MessageSchema::new(name, direction, Schema::new(schema)).describe(description)
}

/// The `name` and `token` query parameters both channels take.
fn channel_params(name_description: &str, token_description: &str) -> EndpointParams {
    EndpointParams {
        query: vec![
            ParamDef::new("name", QueryParamType::String)
                .required()
                .describes(name_description),
            ParamDef::new("token", QueryParamType::String)
                .secret()
                .describes(token_description),
        ],
        ..EndpointParams::default()
    }
}

/// Creates the Samsung Smart TV remote control WebSocket API definition.
///
/// Models the `samsung.remote.control` channel on WSS port 8002, used for
/// sending remote key commands and receiving channel lifecycle events.
///
/// Connection requires a `name` parameter (base64-encoded client name) and
/// optionally accepts a `token` from a previously approved session to bypass
/// the on-TV approval prompt. Approval happens on the TV, not through a
/// message challenge, and replies are matched by event name only, so the
/// definition declares no auth flow and no request correlation.
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
    use MessageDirection::{Client, Server};
    let messages = vec![
        message("RemoteControlCommand", Client, "SamsungRemoteControlCommand",
            "Remote key command (method, key, action). Use `Cmd=Click` for \
             single presses, `Press`/`Release` for hold gestures.")
            .matches("method", "ms.remote.control"),
        message("ChannelConnectEvent", Server, "SamsungRemoteConnectEvent",
            "Emitted when the channel is successfully connected and authorized. \
             May include a `token` for future reconnects.")
            .matches("event", "ms.channel.connect"),
        message("ChannelUnauthorizedEvent", Server, "SamsungRemoteUnauthorizedEvent",
            "Emitted when channel connection is unauthorized (approval needed or denied).")
            .matches("event", "ms.channel.unauthorized"),
        message("ChannelErrorEvent", Server, "SamsungRemoteErrorEvent",
            "Emitted on channel-level errors.")
            .matches("event", "ms.error"),
        message("ChannelEnvelope", Server, "SamsungRemoteEnvelope",
            "Generic envelope with deferred `RawValue` payload for any other event. \
             Route by inspecting `event` or `method`, then deserialize `data` on demand.")
            .fallback(),
        // App launch and installed-apps commands share `method=ms.channel.emit`
        // and differ only in the nested `params.event`; they are outbound only,
        // so they declare no selector.
        message("AppLaunchCommand", Client, "SamsungAppLaunchCommand",
            "Launch an application via `ms.channel.emit` with optional deep-link \
             metadata. Supports both `DEEP_LINK` and `NATIVE_LAUNCH` action types."),
        message("InstalledAppsCommand", Client, "SamsungInstalledAppsCommand",
            "Request the list of installed applications via `ms.channel.emit`. \
             The TV responds with an `ed.installedApp.get` event containing the app list."),
    ];

    let art_mode_messages = vec![
        message("ArtModeCommand", Client, "SamsungArtModeCommand",
            "Art Mode command sent via `ms.channel.emit` with double-encoded \
             data payload. Supports brightness, artwork selection, and mode control.")
            .matches("method", "ms.channel.emit"),
        message("ArtModeEvent", Server, "SamsungArtModeEvent",
            "Art Mode response event. The `data` field contains a double-encoded \
             JSON string that must be parsed separately.")
            .matches("event", "d2d_service_message"),
        message("ChannelConnectEvent", Server, "SamsungRemoteConnectEvent",
            "Emitted when the Art Mode channel is successfully connected.")
            .matches("event", "ms.channel.connect"),
        message("ChannelEnvelope", Server, "SamsungRemoteEnvelope",
            "Generic envelope with deferred payload for any other Art Mode event.")
            .fallback(),
    ];

    let endpoints = vec![
        WebSocketEndpoint {
            id: "RemoteControl".to_string(),
            path: "/api/v2/channels/samsung.remote.control".to_string(),
            description: "Samsung remote control channel for key transport and lifecycle events. \
            First connection may trigger an on-TV approval prompt; approved sessions \
            provide a token for subsequent reconnects."
                .to_string(),
            params: channel_params(
                "Base64-encoded client name used by Samsung remote channel",
                "Previously approved remote token to bypass repeated on-TV prompts",
            ),
            lifecycle: ConnectionLifecycle::default(),
            messages,
            ..WebSocketEndpoint::default()
        },
        WebSocketEndpoint {
            id: "ArtMode".to_string(),
            path: "/api/v2/channels/com.samsung.art-app".to_string(),
            description: "Samsung Art Mode (Frame TV) channel for artwork management, \
                brightness control, and Art Mode on/off. Uses double-encoded JSON in \
                the `data` field of `ms.channel.emit` messages."
                .to_string(),
            params: channel_params(
                "Base64-encoded client name for the Art Mode channel",
                "Previously approved token to bypass on-TV approval prompt",
            ),
            lifecycle: ConnectionLifecycle::default(),
            messages: art_mode_messages,
            ..WebSocketEndpoint::default()
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
        auth_policy: None,
        env_auth: vec![],
        env_mapping: Some(EnvMapping::default()),
        headers: vec![],
        version: None,
        endpoints,
        runtime: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn ws_api_declares_no_request_id_type_or_correlation() {
        // No documented request/response id pairing: replies match by event
        // name, so neither an id type nor correlation hints are declared.
        let api = define_samsung_smart_tv_remote_ws_api();
        assert!(api.runtime.is_none());
        assert!(api.endpoints.iter().all(|ep| ep.runtime.is_none()));
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
        assert!(ep.params.path.is_empty() && ep.params.header.is_empty());
        assert_eq!(ep.params.query.len(), 2);

        let name_param = ep
            .params
            .query
            .iter()
            .find(|p| p.name == "name")
            .expect("name param must exist");
        assert!(name_param.required);
        assert_eq!(name_param.param_type, QueryParamType::String);

        let token_param = ep
            .params
            .query
            .iter()
            .find(|p| p.name == "token")
            .expect("token param must exist");
        assert!(!token_param.required);
        assert!(token_param.secret);
        assert_eq!(token_param.param_type, QueryParamType::String);
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
        let names: Vec<&str> = ep.params.query.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["name", "token"]);
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

        let cmd = ep
            .messages
            .iter()
            .find(|m| m.name == "RemoteControlCommand")
            .unwrap();
        assert_eq!(cmd.direction, MessageDirection::Client);

        let connect = ep
            .messages
            .iter()
            .find(|m| m.name == "ChannelConnectEvent")
            .unwrap();
        assert_eq!(connect.direction, MessageDirection::Server);

        let envelope = ep
            .messages
            .iter()
            .find(|m| m.name == "ChannelEnvelope")
            .unwrap();
        assert_eq!(envelope.direction, MessageDirection::Server);
        assert!(envelope.fallback && envelope.selectors.is_none());
    }
}
