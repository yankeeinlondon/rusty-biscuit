//! Unfolded Circle Core WebSocket API definition.
//!
//! ## Message Dispatch
//!
//! Route incoming messages by inspecting the `msg` field on an envelope:
//!
//! ```rust,ignore
//! use schematic_definitions::unfolded_circle::core_ws::*;
//!
//! fn handle(envelope: CoreWsResponseEnvelope) -> Result<(), Box<dyn std::error::Error>> {
//!     match &envelope.msg {
//!         CoreWsMessageName::Known(CoreWsKnownMessage::SystemInfo) => {
//!             // parse msg_data into your payload type
//!             let info: serde_json::Value = envelope.parse_payload()?;
//!             println!("system info: {info:?}");
//!         }
//!         CoreWsMessageName::Known(CoreWsKnownMessage::Pong) => {
//!             println!("keepalive ok");
//!         }
//!         CoreWsMessageName::Other(name) => {
//!             println!("unhandled message: {name}");
//!         }
//!         _ => {}
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Authentication Flow
//!
//! 1. Connect to WebSocket endpoint (e.g. `ws://remote.local/ws`)
//! 2. Server sends `auth_required` (`CoreWsAuthRequired`)
//! 3. Client sends `auth` (`CoreWsAuthMessage`) with API key in `msg_data`
//! 4. Server responds with `authentication` (`CoreWsAuthenticationMessage`)

mod types;

pub use types::*;

use schematic_define::websocket::{
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, Schema};

#[allow(clippy::vec_init_then_push)]
fn common_messages() -> Vec<MessageSchema> {
    let mut messages = Vec::with_capacity(13);
    messages.push(MessageSchema {
        name: "RequestEnvelope".to_string(),
        direction: MessageDirection::Client,
        schema: Schema::new("CoreWsRequestEnvelope"),
        description: Some("Generic request envelope `{kind=req,id,msg,msg_data}`.".to_string()),
    });
    messages.push(MessageSchema {
        name: "ResponseEnvelope".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsResponseEnvelope"),
        description: Some(
            "Generic response envelope `{kind=resp,req_id,msg,code,msg_data}`.".to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "EventEnvelope".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsEventEnvelope"),
        description: Some("Generic event envelope `{kind=event,msg,cat,ts,msg_data}`.".to_string()),
    });
    messages.push(MessageSchema {
        name: "AuthRequired".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsAuthRequired"),
        description: Some(
            "Server challenge for message-based auth (`msg=auth_required`).".to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "Auth".to_string(),
        direction: MessageDirection::Client,
        schema: Schema::new("CoreWsAuthMessage"),
        description: Some("Message-based auth request (`msg=auth`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Authentication".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsAuthenticationMessage"),
        description: Some(
            "Authentication outcome (`msg=authentication`) returned after auth request."
                .to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "Ping".to_string(),
        direction: MessageDirection::Client,
        schema: Schema::new("CoreWsPingMessage"),
        description: Some("Typed ping request (`msg=ping`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Pong".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsPongMessage"),
        description: Some("Typed pong response (`msg=pong`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Version".to_string(),
        direction: MessageDirection::Client,
        schema: Schema::new("CoreWsVersionMessage"),
        description: Some("Typed version request (`msg=version`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "VersionInfo".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsVersionInfoMessage"),
        description: Some("Typed version response (`msg=version_info`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "System".to_string(),
        direction: MessageDirection::Client,
        schema: Schema::new("CoreWsSystemMessage"),
        description: Some("Typed system-info request (`msg=system`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "SystemInfo".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsSystemInfoMessage"),
        description: Some("Typed system-info response (`msg=system_info`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Result".to_string(),
        direction: MessageDirection::Server,
        schema: Schema::new("CoreWsResultMessage"),
        description: Some("Typed generic command result (`msg=result`).".to_string()),
    });
    messages
}

/// Build the Unfolded Circle Core WebSocket API definition.
#[must_use]
#[allow(clippy::vec_init_then_push)]
pub fn define_unfolded_circle_core_ws_api() -> WebSocketApi {
    let common_messages = common_messages();
    let mut endpoints = Vec::with_capacity(4);
    endpoints.push(WebSocketEndpoint {
        id: "CoreWs".to_string(),
        path: "/ws".to_string(),
        description: "Primary command/event channel for Core system control. Use this channel for RPC-style commands and asynchronous events documented in the Core WS docs.".to_string(),
        connection_params: vec![],
        lifecycle: ConnectionLifecycle::default(),
        messages: common_messages.clone(),
        runtime: None,
    });
    endpoints.push(WebSocketEndpoint {
        id: "CoreIntegrations".to_string(),
        path: "/intg".to_string(),
        description: "Integration management channel for discovery, setup, and driver lifecycle events. See Core API integration WebSocket documentation for message semantics.".to_string(),
        connection_params: vec![],
        lifecycle: ConnectionLifecycle::default(),
        messages: common_messages.clone(),
        runtime: None,
    });
    endpoints.push(WebSocketEndpoint {
        id: "CoreProfiles".to_string(),
        path: "/profiles".to_string(),
        description: "Profile-focused channel for querying and tracking profile updates without subscribing to all general events.".to_string(),
        connection_params: vec![],
        lifecycle: ConnectionLifecycle::default(),
        messages: common_messages.clone(),
        runtime: None,
    });
    endpoints.push(WebSocketEndpoint {
        id: "CoreEvents".to_string(),
        path: "/events".to_string(),
        description: "General asynchronous event channel optimized for event consumers and telemetry-style subscribers.".to_string(),
        connection_params: vec![],
        lifecycle: ConnectionLifecycle::default(),
        messages: common_messages,
        runtime: None,
    });

    WebSocketApi {
        name: "UnfoldedCircleCoreWs".to_string(),
        description: "Unfolded Circle Core WebSocket API".to_string(),
        base_url: "ws://remote.local/ws".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/ws/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "API-KEY".to_string(),
            value_prefix: None,
        },
        env_auth: vec![
            "UCR_CORE_API_KEY".to_string(),
            "UNFOLDED_CIRCLE_API_KEY".to_string(),
        ],
        version: None,
        endpoints,
        runtime: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_ws_has_expected_endpoints() {
        let api = define_unfolded_circle_core_ws_api();
        assert_eq!(api.name, "UnfoldedCircleCoreWs");
        assert_eq!(api.base_url, "ws://remote.local/ws");
        assert_eq!(
            api.docs_url.as_deref(),
            Some("https://unfoldedcircle.github.io/core-api/ws/")
        );
        assert!(matches!(api.auth, AuthStrategy::ApiKey { ref header, .. } if header == "API-KEY"));
        assert_eq!(
            api.env_auth,
            vec![
                "UCR_CORE_API_KEY".to_string(),
                "UNFOLDED_CIRCLE_API_KEY".to_string()
            ]
        );
        assert_eq!(api.endpoints.len(), 4);
        assert!(api.endpoints.iter().any(|ep| ep.path == "/ws"));
        assert!(api.endpoints.iter().any(|ep| ep.path == "/intg"));
        assert!(api.endpoints.iter().any(|ep| ep.path == "/profiles"));
        assert!(api.endpoints.iter().any(|ep| ep.path == "/events"));
    }

    #[test]
    fn core_ws_exposes_envelope_and_typed_control_messages() {
        let api = define_unfolded_circle_core_ws_api();
        let ws = api
            .endpoints
            .iter()
            .find(|ep| ep.id == "CoreWs")
            .expect("CoreWs endpoint must exist");

        let names: Vec<&str> = ws.messages.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"RequestEnvelope"));
        assert!(names.contains(&"ResponseEnvelope"));
        assert!(names.contains(&"EventEnvelope"));
        assert!(names.contains(&"AuthRequired"));
        assert!(names.contains(&"Auth"));
        assert!(names.contains(&"Authentication"));
        assert!(names.contains(&"Ping"));
        assert!(names.contains(&"Pong"));
        assert!(names.contains(&"Version"));
        assert!(names.contains(&"VersionInfo"));
        assert!(names.contains(&"System"));
        assert!(names.contains(&"SystemInfo"));
        assert!(names.contains(&"Result"));
    }
}
