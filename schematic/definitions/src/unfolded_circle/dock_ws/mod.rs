//! Unfolded Circle Dock WebSocket API definition.

mod types;

pub use types::*;

use schematic_define::websocket::{
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, Schema};

/// Build the Unfolded Circle Dock WebSocket API definition.
#[must_use]
pub fn define_unfolded_circle_dock_ws_api() -> WebSocketApi {
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
            description: "Dock command and telemetry channel".to_string(),
            connection_params: vec![],
            lifecycle: ConnectionLifecycle::default(),
            messages: vec![
                MessageSchema {
                    name: "Request".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("DockWsRequestEnvelope"),
                    description: Some("Request envelope using type/msg fields".to_string()),
                },
                MessageSchema {
                    name: "Response".to_string(),
                    direction: MessageDirection::Server,
                    schema: Schema::new("DockWsResponseEnvelope"),
                    description: Some("Response envelope with req_id/code".to_string()),
                },
                MessageSchema {
                    name: "Event".to_string(),
                    direction: MessageDirection::Server,
                    schema: Schema::new("DockWsEventEnvelope"),
                    description: Some("Asynchronous dock event envelope".to_string()),
                },
                MessageSchema {
                    name: "Auth".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("DockWsAuthMessage"),
                    description: Some("Post-connect auth message".to_string()),
                },
            ],
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_ws_has_root_channel() {
        let api = define_unfolded_circle_dock_ws_api();
        assert_eq!(api.endpoints.len(), 1);
        assert_eq!(api.endpoints[0].path, "/");
    }
}
