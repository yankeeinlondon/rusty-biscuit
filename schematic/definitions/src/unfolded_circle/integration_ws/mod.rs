//! Unfolded Circle Integration WebSocket API definition.
//!
//! ## Message Dispatch
//!
//! Route incoming requests from the remote by inspecting the `msg` field:
//!
//! ```rust,ignore
//! use schematic_definitions::unfolded_circle::integration_ws::*;
//!
//! fn handle(envelope: IntegrationWsRequestEnvelope) -> Result<(), Box<dyn std::error::Error>> {
//!     match &envelope.msg {
//!         IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityCommand) => {
//!             let cmd: serde_json::Value = envelope.parse_payload()?;
//!             // handle entity command from the remote
//!         }
//!         IntegrationWsMessageName::Known(IntegrationWsKnownMessage::GetAvailableEntities) => {
//!             // respond with available entities
//!         }
//!         _ => {}
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Integration Driver Required Messages
//!
//! Drivers must handle these requests from the remote:
//! `get_driver_version`, `get_device_state`, `get_available_entities`,
//! `subscribe_events`, `get_entity_states`, `entity_command`
//!
//! Drivers must emit these events:
//! `entity_change`, `device_state`

mod types;

pub use types::*;

use schematic_define::websocket::{
    AuthFlowHints, ConnectionLifecycle, CorrelationHints, EndpointRole, MessageDirection,
    MessageSchema, WebSocketApi, WebSocketEndpoint, WebSocketEndpointHints,
};
use schematic_define::{ApiKeyEnv, AuthStrategy, EnvList, EnvMapping, Schema};

fn message(name: &str, direction: MessageDirection, schema: &str, description: &str) -> MessageSchema {
    MessageSchema::new(name, direction, Schema::new(schema)).describe(description)
}

/// Build the Unfolded Circle Integration WebSocket API definition.
///
/// The integration driver is the WebSocket server and the remote is the
/// client, so the endpoint's role is [`EndpointRole::Both`]: drivers use the
/// generated host. Directions are client-relative: `Client` messages are what
/// the remote sends to the driver.
#[must_use]
pub fn define_unfolded_circle_integration_ws_api() -> WebSocketApi {
    use MessageDirection::{Bidirectional, Client, Server};
    let messages = vec![
        message("RequestEnvelope", Client, "IntegrationWsRequestEnvelope",
            "Request envelope `{kind=req,id,msg,msg_data}` sent by the remote to the driver.")
            .matches("kind", "req")
            .responds_with("ResponseEnvelope"),
        message("ResponseEnvelope", Server, "IntegrationWsResponseEnvelope",
            "Driver response envelope; `req_id` echoes the request `id`.")
            .fallback(),
        message("EventEnvelope", Bidirectional, "IntegrationWsEventEnvelope",
            "Asynchronous event envelope (`kind=event`): the driver pushes entity and device \
             state, the remote sends connection and setup events.")
            .matches("kind", "event"),
        message("AuthRequired", Server, "IntegrationWsAuthRequired",
            "Driver challenge for message-based auth (`msg=auth_required`).")
            .matches("kind", "event")
            .matches("msg", "auth_required"),
        message("Auth", Client, "IntegrationWsAuthMessage",
            "Remote's message-based authentication request (`msg=auth`).")
            .matches("kind", "req")
            .matches("msg", "auth")
            .responds_with("Authentication"),
        message("Authentication", Server, "IntegrationWsAuthenticationMessage",
            "Driver's authentication result (`msg=authentication`); `code` is the outcome.")
            .matches("kind", "resp")
            .matches("msg", "authentication"),
    ];

    WebSocketApi {
        name: "UnfoldedCircleIntegrationWs".to_string(),
        description: "Unfolded Circle Integration WebSocket API".to_string(),
        base_url: "ws://remote.local".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/integration/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "auth-token".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: vec!["UCR_INTEGRATION_TOKEN".to_string()],
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::single("UCR_INTEGRATION_TOKEN"),
                header: "auth-token".to_string(),
            }),
            ..EnvMapping::default()
        }),
        headers: vec![],
        version: None,
        endpoints: vec![WebSocketEndpoint {
            id: "Integration".to_string(),
            path: "/intg".to_string(),
            description: "Integration driver channel. The driver hosts it and the remote connects; requests flow from the remote, responses and entity events from the driver.".to_string(),
            role: EndpointRole::Both,
            lifecycle: ConnectionLifecycle::default(),
            messages,
            runtime: Some(WebSocketEndpointHints {
                correlation: Some(CorrelationHints {
                    request_id_field: "id".to_string(),
                    response_id_field: "req_id".to_string(),
                    default_timeout_secs: 30,
                }),
                // Sent only when the driver does not accept the `auth-token`
                // handshake header.
                auth_flow: Some(AuthFlowHints {
                    challenge_message: "auth_required".to_string(),
                    auth_request_schema: "IntegrationWsAuthMessage".to_string(),
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
    fn integration_ws_has_single_intg_endpoint() {
        let api = define_unfolded_circle_integration_ws_api();
        assert_eq!(api.name, "UnfoldedCircleIntegrationWs");
        assert_eq!(api.base_url, "ws://remote.local");
        assert_eq!(
            api.docs_url.as_deref(),
            Some("https://unfoldedcircle.github.io/core-api/integration/")
        );
        assert!(matches!(api.auth, AuthStrategy::ApiKey { ref header, .. } if header == "auth-token"));
        assert_eq!(api.env_auth, vec!["UCR_INTEGRATION_TOKEN".to_string()]);
        assert_eq!(api.endpoints.len(), 1);
        assert_eq!(api.endpoints[0].path, "/intg");
    }

    #[test]
    fn integration_ws_declares_both_roles_and_true_directions() {
        use MessageDirection::{Bidirectional, Client, Server};
        let api = define_unfolded_circle_integration_ws_api();
        let endpoint = &api.endpoints[0];
        assert_eq!(endpoint.role, EndpointRole::Both);
        let directions: Vec<(&str, MessageDirection)> = endpoint
            .messages
            .iter()
            .map(|m| (m.name.as_str(), m.direction))
            .collect();
        assert_eq!(
            directions,
            [
                ("RequestEnvelope", Client),
                ("ResponseEnvelope", Server),
                ("EventEnvelope", Bidirectional),
                ("AuthRequired", Server),
                ("Auth", Client),
                ("Authentication", Server),
            ]
        );
    }
}
