//! Unfolded Circle Dock WebSocket API definition.
//!
//! ## Authentication Flow
//!
//! The Dock uses post-connect token authentication:
//!
//! 1. Connect to `ws://dock.local/`
//! 2. Server sends `auth_required` (`DockWsAuthRequired`)
//! 3. Client sends `auth` (`DockWsAuthMessage`) with `token` field
//! 4. Server responds with `authentication` (`DockWsAuthenticationMessage`)
//!
//! ## Envelope Differences from Core WS
//!
//! The Dock API uses `type` (not `kind`) as the envelope discriminator field.
//! In Rust types this is mapped to `type_name` with `#[serde(rename = "type")]`.

mod types;

pub use types::*;

use schematic_define::websocket::{
    AuthFlowHints, ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi,
    WebSocketEndpoint, WebSocketEndpointHints,
};
use schematic_define::{AuthStrategy, Schema};

/// Build the Unfolded Circle Dock WebSocket API definition.
#[must_use]
#[allow(clippy::vec_init_then_push)]
pub fn define_unfolded_circle_dock_ws_api() -> WebSocketApi {
    let mut messages = Vec::with_capacity(6);
    messages.push(MessageSchema {
        name: "RequestEnvelope".to_string(),
        direction: MessageDirection::Client,
        schema: Schema::new("DockWsRequestEnvelope"),
        description: Some(
            "Client request envelope using `type=req`, `id`, `msg`, and deferred `msg_data`."
                .to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "ResponseEnvelope".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("DockWsResponseEnvelope"),
        description: Some(
            "Server response envelope with `req_id`, status `code`, and optional `reboot`."
                .to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "EventEnvelope".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("DockWsEventEnvelope"),
        description: Some(
            "Asynchronous dock event envelope. Deserialize `msg_data` based on `msg`.".to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "AuthRequired".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("DockWsAuthRequired"),
        description: Some("Post-connect auth challenge (`msg=auth_required`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Auth".to_string(),
        direction: MessageDirection::Client,
        schema: Schema::new("DockWsAuthMessage"),
        description: Some("Post-connect auth request (`msg=auth`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Authentication".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("DockWsAuthenticationMessage"),
        description: Some("Auth outcome (`msg=authentication`).".to_string()),
    });

    WebSocketApi {
        name: "UnfoldedCircleDockWs".to_string(),
        description: "Unfolded Circle Dock WebSocket API".to_string(),
        base_url: "ws://dock.local".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/dock/".to_string()),
        auth: AuthStrategy::None,
        env_auth: vec!["UCR_DOCK_TOKEN".to_string()],
        endpoints: vec![WebSocketEndpoint {
            id: "DockRoot".to_string(),
            path: "/".to_string(),
            description: "Dock command and telemetry channel. This endpoint uses `type` as the envelope discriminant (not `kind`) and requires post-connect authentication for most operations.".to_string(),
            connection_params: vec![],
            lifecycle: ConnectionLifecycle::default(),
            messages,
            runtime: Some(WebSocketEndpointHints {
                correlation: None,
                auth_flow: Some(AuthFlowHints {
                    challenge_message: "auth_required".to_string(),
                    auth_request_schema: "DockWsAuthMessage".to_string(),
                    success_indicator: Some("authentication".to_string()),
                    failure_indicator: Some("authentication_failed".to_string()),
                }),
                heartbeat: None,
            }),
        }],
        runtime: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_ws_has_root_channel() {
        let api = define_unfolded_circle_dock_ws_api();
        assert_eq!(api.name, "UnfoldedCircleDockWs");
        assert_eq!(api.base_url, "ws://dock.local");
        assert_eq!(
            api.docs_url.as_deref(),
            Some("https://unfoldedcircle.github.io/core-api/dock/")
        );
        assert!(matches!(api.auth, AuthStrategy::None));
        assert_eq!(api.env_auth, vec!["UCR_DOCK_TOKEN".to_string()]);
        assert_eq!(api.endpoints.len(), 1);
        assert_eq!(api.endpoints[0].path, "/");
        assert!(
            api.endpoints[0]
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.auth_flow.as_ref())
                .is_some()
        );
    }

    #[test]
    fn dock_ws_has_envelope_and_auth_control_messages() {
        let api = define_unfolded_circle_dock_ws_api();
        let names: Vec<&str> = api.endpoints[0]
            .messages
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert!(names.contains(&"RequestEnvelope"));
        assert!(names.contains(&"ResponseEnvelope"));
        assert!(names.contains(&"EventEnvelope"));
        assert!(names.contains(&"AuthRequired"));
        assert!(names.contains(&"Auth"));
        assert!(names.contains(&"Authentication"));
    }
}
