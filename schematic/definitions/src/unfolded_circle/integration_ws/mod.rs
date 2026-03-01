//! Unfolded Circle Integration WebSocket API definition.

mod types;

pub use types::*;

use schematic_define::websocket::{
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, Schema};

/// Build the Unfolded Circle Integration WebSocket API definition.
#[must_use]
pub fn define_unfolded_circle_integration_ws_api() -> WebSocketApi {
    WebSocketApi {
        name: "UnfoldedCircleIntegrationWs".to_string(),
        description: "Unfolded Circle Integration WebSocket API".to_string(),
        base_url: "ws://remote.local".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/integration/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "auth-token".to_string(),
        },
        env_auth: vec!["UCR_INTEGRATION_TOKEN".to_string()],
        endpoints: vec![WebSocketEndpoint {
            id: "Integration".to_string(),
            path: "/intg".to_string(),
            description: "Integration driver websocket channel".to_string(),
            connection_params: vec![],
            lifecycle: ConnectionLifecycle::default(),
            messages: vec![
                MessageSchema {
                    name: "Request".to_string(),
                    direction: MessageDirection::Bidirectional,
                    schema: Schema::new("IntegrationWsRequestEnvelope"),
                    description: Some(
                        "Request envelope for client/server command exchange".to_string(),
                    ),
                },
                MessageSchema {
                    name: "Response".to_string(),
                    direction: MessageDirection::Bidirectional,
                    schema: Schema::new("IntegrationWsResponseEnvelope"),
                    description: Some("Response envelope for request correlation".to_string()),
                },
                MessageSchema {
                    name: "Event".to_string(),
                    direction: MessageDirection::Bidirectional,
                    schema: Schema::new("IntegrationWsEventEnvelope"),
                    description: Some("Asynchronous event envelope".to_string()),
                },
                MessageSchema {
                    name: "Auth".to_string(),
                    direction: MessageDirection::Bidirectional,
                    schema: Schema::new("IntegrationWsAuthMessage"),
                    description: Some("Header or message-based auth fallback".to_string()),
                },
            ],
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_ws_has_single_intg_endpoint() {
        let api = define_unfolded_circle_integration_ws_api();
        assert_eq!(api.endpoints.len(), 1);
        assert_eq!(api.endpoints[0].path, "/intg");
    }
}
