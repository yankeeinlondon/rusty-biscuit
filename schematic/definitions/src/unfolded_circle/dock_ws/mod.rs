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
//! Neither auth message carries a request id, so the exchange is not a
//! correlated request: run it in the connect setup callback.
//!
//! ## Envelope Differences from Core WS
//!
//! The Dock API uses `type` (not `kind`) as the envelope discriminator field.
//! In Rust types this is mapped to `type_name` with `#[serde(rename = "type")]`.
//!
//! Commands and their replies share `type=dock`: a request names its command
//! in `command` (`{"type":"dock","id":2,"command":"get_sysinfo"}`), and the
//! reply echoes it in `msg` beside `req_id` and `code`. Events use
//! `type=event`. There is no `msg_data` object: parameters, results, and
//! event data are top-level fields, which the envelopes keep in a map
//! (`params` on a request, `data` on a reply or event).

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
/// (`specs/unfolded-circle/UCD2-asyncapi.yaml`): commands are `type=dock`,
/// events `type=event`, and the auth exchange uses `type=auth_required`,
/// `type=auth`, and `type=auth,msg=authentication`. Every other server frame,
/// a `type=dock` reply included, reaches the `ResponseEnvelope` fallback.
#[must_use]
pub fn define_unfolded_circle_dock_ws_api() -> WebSocketApi {
    use MessageDirection::{Client, Server};
    let messages = vec![
        message("RequestEnvelope", Client, "DockWsRequestEnvelope",
            "Command request (`type=dock`) with `id`, `command`, and top-level parameters.")
            .matches("type", "dock")
            .responds_with("ResponseEnvelope"),
        message("ResponseEnvelope", Server, "DockWsResponseEnvelope",
            "Command reply (`type=dock`) with `req_id`, `msg`, status `code`, and top-level result fields.")
            .fallback(),
        message("EventEnvelope", Server, "DockWsEventEnvelope",
            "Asynchronous dock event (`type=event`); `msg` names it and its fields are top-level.")
            .matches("type", "event"),
        message("AuthRequired", Server, "DockWsAuthRequired",
            "Post-connect auth challenge (`type=auth_required`).")
            .matches("type", "auth_required"),
        // No `responds_with`: neither the auth request nor its result carries
        // an id, so the exchange cannot be correlated; the connect setup
        // callback sends `Auth` and reads `Authentication`.
        message("Auth", Client, "DockWsAuthMessage", "Post-connect auth request (`type=auth`).")
            .matches("type", "auth"),
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
