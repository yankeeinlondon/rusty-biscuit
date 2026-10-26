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
    AuthFlowHints, ConnectionLifecycle, CorrelationHints, MessageDirection, MessageSchema,
    WebSocketApi, WebSocketEndpoint, WebSocketEndpointHints,
};
use schematic_define::{ApiKeyEnv, AuthStrategy, EnvList, EnvMapping, Schema};

const ENV_AUTH: [&str; 2] = ["UCR_CORE_API_KEY", "UNFOLDED_CIRCLE_API_KEY"];

fn client(name: &str, schema: &str, description: &str) -> MessageSchema {
    MessageSchema::new(name, MessageDirection::Client, Schema::new(schema)).describe(description)
}

fn server(name: &str, schema: &str, description: &str) -> MessageSchema {
    MessageSchema::new(name, MessageDirection::Server, Schema::new(schema)).describe(description)
}

/// The messages every Core channel shares. Selectors are the documented
/// `kind`/`msg` wire values; an unmatched server frame is a generic response.
fn common_messages() -> Vec<MessageSchema> {
    vec![
        client("RequestEnvelope", "CoreWsRequestEnvelope",
            "Generic request envelope `{kind=req,id,msg,msg_data}`.")
            .matches("kind", "req")
            .responds_with("ResponseEnvelope"),
        server("ResponseEnvelope", "CoreWsResponseEnvelope",
            "Generic response envelope `{kind=resp,req_id,msg,code,msg_data}`.")
            .fallback(),
        server("EventEnvelope", "CoreWsEventEnvelope",
            "Generic event envelope `{kind=event,msg,cat,ts,msg_data}`.")
            .matches("kind", "event"),
        server("AuthRequired", "CoreWsAuthRequired",
            "Server challenge for message-based auth (`msg=auth_required`).")
            .matches("kind", "event")
            .matches("msg", "auth_required"),
        client("Auth", "CoreWsAuthMessage", "Message-based auth request (`msg=auth`).")
            .matches("kind", "req")
            .matches("msg", "auth")
            .responds_with("Authentication"),
        server("Authentication", "CoreWsAuthenticationMessage",
            "Authentication outcome (`msg=authentication`) returned after auth request.")
            .matches("kind", "resp")
            .matches("msg", "authentication"),
        client("Ping", "CoreWsPingMessage", "Typed ping request (`msg=ping`).")
            .matches("kind", "req")
            .matches("msg", "ping")
            .responds_with("Pong"),
        server("Pong", "CoreWsPongMessage", "Typed pong response (`msg=pong`).")
            .matches("kind", "resp")
            .matches("msg", "pong"),
        client("Version", "CoreWsVersionMessage", "Typed version request (`msg=version`).")
            .matches("kind", "req")
            .matches("msg", "version")
            .responds_with("VersionInfo"),
        server("VersionInfo", "CoreWsVersionInfoMessage",
            "Typed version response (`msg=version_info`).")
            .matches("kind", "resp")
            .matches("msg", "version_info"),
        client("System", "CoreWsSystemMessage", "Typed system-info request (`msg=system`).")
            .matches("kind", "req")
            .matches("msg", "system")
            .responds_with("SystemInfo"),
        server("SystemInfo", "CoreWsSystemInfoMessage",
            "Typed system-info response (`msg=system_info`).")
            .matches("kind", "resp")
            .matches("msg", "system_info"),
        server("Result", "CoreWsResultMessage", "Typed generic command result (`msg=result`).")
            .matches("kind", "resp")
            .matches("msg", "result"),
    ]
}

/// Requests carry `id`; responses echo it as `req_id`. The `auth_required`
/// challenge is sent only when the handshake carried no `API-KEY` header.
fn common_hints() -> WebSocketEndpointHints {
    WebSocketEndpointHints {
        correlation: Some(CorrelationHints {
            request_id_field: "id".to_string(),
            response_id_field: "req_id".to_string(),
            default_timeout_secs: 30,
        }),
        auth_flow: Some(AuthFlowHints {
            challenge_message: "auth_required".to_string(),
            auth_request_schema: "CoreWsAuthMessage".to_string(),
            success_indicator: Some("authentication".to_string()),
            failure_indicator: None,
        }),
        heartbeat: None,
    }
}

fn channel(id: &str, path: &str, description: &str) -> WebSocketEndpoint {
    WebSocketEndpoint {
        id: id.to_string(),
        path: path.to_string(),
        description: description.to_string(),
        lifecycle: ConnectionLifecycle::default(),
        messages: common_messages(),
        runtime: Some(common_hints()),
        ..WebSocketEndpoint::default()
    }
}

/// Build the Unfolded Circle Core WebSocket API definition.
#[must_use]
pub fn define_unfolded_circle_core_ws_api() -> WebSocketApi {
    let endpoints = vec![
        channel("CoreWs", "/ws", "Primary command/event channel for Core system control. Use this channel for RPC-style commands and asynchronous events documented in the Core WS docs."),
        channel("CoreIntegrations", "/intg", "Integration management channel for discovery, setup, and driver lifecycle events. See Core API integration WebSocket documentation for message semantics."),
        channel("CoreProfiles", "/profiles", "Profile-focused channel for querying and tracking profile updates without subscribing to all general events."),
        channel("CoreEvents", "/events", "General asynchronous event channel optimized for event consumers and telemetry-style subscribers."),
    ];

    WebSocketApi {
        name: "UnfoldedCircleCoreWs".to_string(),
        description: "Unfolded Circle Core WebSocket API".to_string(),
        // The origin only: each endpoint carries its own documented channel
        // path, so a base path here would double it (`/ws/ws`).
        base_url: "ws://remote.local".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/ws/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "API-KEY".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: ENV_AUTH.map(str::to_string).to_vec(),
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::from_strs(&ENV_AUTH),
                header: "API-KEY".to_string(),
            }),
            ..EnvMapping::default()
        }),
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
    fn core_ws_has_expected_endpoints() {
        let api = define_unfolded_circle_core_ws_api();
        assert_eq!(api.name, "UnfoldedCircleCoreWs");
        assert_eq!(api.base_url, "ws://remote.local");
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
        // Channel paths from the Core AsyncAPI (`specs/unfolded-circle/UCR-core-asyncapi.yaml`).
        let paths: Vec<(&str, &str)> = api
            .endpoints
            .iter()
            .map(|ep| (ep.id.as_str(), ep.path.as_str()))
            .collect();
        assert_eq!(
            paths,
            [
                ("CoreWs", "/ws"),
                ("CoreIntegrations", "/intg"),
                ("CoreProfiles", "/profiles"),
                ("CoreEvents", "/events"),
            ]
        );
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
