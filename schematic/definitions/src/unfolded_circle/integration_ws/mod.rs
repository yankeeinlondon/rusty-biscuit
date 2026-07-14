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
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, Schema};

/// Build the Unfolded Circle Integration WebSocket API definition.
#[must_use]
#[allow(clippy::vec_init_then_push)]
pub fn define_unfolded_circle_integration_ws_api() -> WebSocketApi {
    let mut messages = Vec::with_capacity(6);
    messages.push(MessageSchema {
        name: "RequestEnvelope".to_string(),
        direction: MessageDirection::Bidirectional,
        schema: Schema::new("IntegrationWsRequestEnvelope"),
        description: Some(
            "Bidirectional request envelope used by both remote-client and driver-host roles."
                .to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "ResponseEnvelope".to_string(),
        direction: MessageDirection::Bidirectional,
        schema: Schema::new("IntegrationWsResponseEnvelope"),
        description: Some(
            "Bidirectional response envelope for request correlation (`req_id`).".to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "EventEnvelope".to_string(),
        direction: MessageDirection::Bidirectional,
        schema: Schema::new("IntegrationWsEventEnvelope"),
        description: Some(
            "Bidirectional asynchronous event envelope for integration entity lifecycle events."
                .to_string(),
        ),
    });
    messages.push(MessageSchema {
        name: "AuthRequired".to_string(),
        direction: MessageDirection::Bidirectional,
        schema: Schema::new("IntegrationWsAuthRequired"),
        description: Some("Auth challenge message (`msg=auth_required`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Auth".to_string(),
        direction: MessageDirection::Bidirectional,
        schema: Schema::new("IntegrationWsAuthMessage"),
        description: Some("Message-based authentication request (`msg=auth`).".to_string()),
    });
    messages.push(MessageSchema {
        name: "Authentication".to_string(),
        direction: MessageDirection::Bidirectional,
        schema: Schema::new("IntegrationWsAuthenticationMessage"),
        description: Some("Authentication result message (`msg=authentication`).".to_string()),
    });

    WebSocketApi {
        name: "UnfoldedCircleIntegrationWs".to_string(),
        description: "Unfolded Circle Integration WebSocket API".to_string(),
        base_url: "ws://remote.local".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/integration/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "auth-token".to_string(),
            value_prefix: None,
        },
        env_auth: vec!["UCR_INTEGRATION_TOKEN".to_string()],
        version: None,
        endpoints: vec![WebSocketEndpoint {
            id: "Integration".to_string(),
            path: "/intg".to_string(),
            description: "Integration driver channel with role inversion support (driver-as-server). This endpoint is intentionally bidirectional for command handling and outbound event push.".to_string(),
            connection_params: vec![],
            lifecycle: ConnectionLifecycle::default(),
            messages,
            runtime: None,
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
    fn integration_ws_channel_is_bidirectional_and_has_auth_messages() {
        let api = define_unfolded_circle_integration_ws_api();
        let messages = &api.endpoints[0].messages;
        assert!(
            messages
                .iter()
                .all(|msg| msg.direction == MessageDirection::Bidirectional)
        );
        let names: Vec<&str> = messages.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"RequestEnvelope"));
        assert!(names.contains(&"ResponseEnvelope"));
        assert!(names.contains(&"EventEnvelope"));
        assert!(names.contains(&"AuthRequired"));
        assert!(names.contains(&"Auth"));
        assert!(names.contains(&"Authentication"));
    }
}
