//! Unfolded Circle Dock WebSocket API definition.
//!
//! ## Authentication Flow
//!
//! The Dock uses post-connect token authentication:
//!
//! 1. Connect to `ws://dock.local/`
//! 2. Server sends `type=auth_required` (`DockWsAuthRequired`)
//! 3. Client sends `type=auth` (`DockWsAuthMessage`) with `token` field
//! 4. Server responds with `type=auth`, `msg=authentication`
//!    (`DockWsAuthenticationMessage`); its `code` is the result
//!
//! ## Envelope Differences from Core WS
//!
//! The Dock API uses `type` (not `kind`) as the envelope discriminator field.
//! In Rust types this is mapped to `type_name` with `#[serde(rename = "type")]`.

mod types;

pub use types::*;

use schematic_define::websocket::{
    AuthFlowHints, ConnectionLifecycle, CorrelationHints, MessageDirection, MessageSchema,
    WebSocketApi, WebSocketEndpoint, WebSocketEndpointHints,
};
use schematic_define::{AuthStrategy, EnvMapping, Schema};

fn message(name: &str, direction: MessageDirection, schema: &str, description: &str) -> MessageSchema {
    MessageSchema::new(name, direction, Schema::new(schema)).describe(description)
}

/// Build the Unfolded Circle Dock WebSocket API definition.
///
/// Selectors are the `type`/`msg` wire values of the official Dock AsyncAPI
/// (`specs/unfolded-circle/UCD2-asyncapi.yaml`): the auth exchange uses
/// `type=auth_required`, `type=auth`, and `type=auth,msg=authentication`
/// rather than the `req`/`resp`/`event` envelope types.
#[must_use]
pub fn define_unfolded_circle_dock_ws_api() -> WebSocketApi {
    use MessageDirection::{Client, Server};
    let messages = vec![
        message("RequestEnvelope", Client, "DockWsRequestEnvelope",
            "Client request envelope using `type=req`, `id`, `msg`, and deferred `msg_data`.")
            .matches("type", "req")
            .responds_with("ResponseEnvelope"),
        message("ResponseEnvelope", Server, "DockWsResponseEnvelope",
            "Server response envelope with `req_id`, status `code`, and optional `reboot`.")
            .fallback(),
        message("EventEnvelope", Server, "DockWsEventEnvelope",
            "Asynchronous dock event envelope. Deserialize `msg_data` based on `msg`.")
            .matches("type", "event"),
        message("AuthRequired", Server, "DockWsAuthRequired",
            "Post-connect auth challenge (`type=auth_required`).")
            .matches("type", "auth_required"),
        message("Auth", Client, "DockWsAuthMessage", "Post-connect auth request (`type=auth`).")
            .matches("type", "auth")
            .responds_with("Authentication"),
        message("Authentication", Server, "DockWsAuthenticationMessage",
            "Auth outcome (`type=auth`, `msg=authentication`); `code` is the result.")
            .matches("type", "auth")
            .matches("msg", "authentication"),
    ];

    WebSocketApi {
        name: "UnfoldedCircleDockWs".to_string(),
        description: "Unfolded Circle Dock WebSocket API".to_string(),
        base_url: "ws://dock.local".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/dock/".to_string()),
        // The token travels in the `auth` message after the upgrade, never in
        // the handshake, so the handshake has no credential to map.
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec!["UCR_DOCK_TOKEN".to_string()],
        env_mapping: Some(EnvMapping::default()),
        headers: vec![],
        version: None,
        endpoints: vec![WebSocketEndpoint {
            id: "DockRoot".to_string(),
            path: "/".to_string(),
            description: "Dock command and telemetry channel. This endpoint uses `type` as the envelope discriminant (not `kind`) and requires post-connect authentication for most operations.".to_string(),
            lifecycle: ConnectionLifecycle::default(),
            messages,
            runtime: Some(WebSocketEndpointHints {
                correlation: Some(CorrelationHints {
                    request_id_field: "id".to_string(),
                    response_id_field: "req_id".to_string(),
                    default_timeout_secs: 30,
                }),
                auth_flow: Some(AuthFlowHints {
                    challenge_message: "auth_required".to_string(),
                    auth_request_schema: "DockWsAuthMessage".to_string(),
                    success_indicator: Some("authentication".to_string()),
                    failure_indicator: None,
                }),
                heartbeat: None,
            }),
            ..WebSocketEndpoint::default()
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
