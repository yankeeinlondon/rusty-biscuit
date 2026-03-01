//! Unfolded Circle Core WebSocket API definition.

mod types;

pub use types::*;

use schematic_define::websocket::{
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, Schema};

fn common_messages() -> Vec<MessageSchema> {
    vec![
        MessageSchema {
            name: "Request".to_string(),
            direction: MessageDirection::Client,
            schema: Schema::new("CoreWsRequestEnvelope"),
            description: Some("Request envelope {kind=req,id,msg,msg_data}".to_string()),
        },
        MessageSchema {
            name: "Response".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("CoreWsResponseEnvelope"),
            description: Some("Response envelope {kind=resp,req_id,msg,code,msg_data}".to_string()),
        },
        MessageSchema {
            name: "Event".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("CoreWsEventEnvelope"),
            description: Some("Event envelope {kind=event,msg,cat,ts,msg_data}".to_string()),
        },
        MessageSchema {
            name: "AuthRequired".to_string(),
            direction: MessageDirection::Server,
            schema: Schema::new("CoreWsAuthRequired"),
            description: Some("Server challenge for message-based auth".to_string()),
        },
        MessageSchema {
            name: "Auth".to_string(),
            direction: MessageDirection::Client,
            schema: Schema::new("CoreWsAuthMessage"),
            description: Some("Message-based auth request".to_string()),
        },
    ]
}

/// Build the Unfolded Circle Core WebSocket API definition.
#[must_use]
pub fn define_unfolded_circle_core_ws_api() -> WebSocketApi {
    WebSocketApi {
        name: "UnfoldedCircleCoreWs".to_string(),
        description: "Unfolded Circle Core WebSocket API".to_string(),
        base_url: "ws://remote.local/ws".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/ws/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "API-KEY".to_string(),
        },
        env_auth: vec![
            "UCR_CORE_API_KEY".to_string(),
            "UNFOLDED_CIRCLE_API_KEY".to_string(),
        ],
        endpoints: vec![
            WebSocketEndpoint {
                id: "CoreWs".to_string(),
                path: "/ws".to_string(),
                description: "Core command and event channel".to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: common_messages(),
            },
            WebSocketEndpoint {
                id: "CoreIntegrations".to_string(),
                path: "/intg".to_string(),
                description: "Integration management event channel".to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: common_messages(),
            },
            WebSocketEndpoint {
                id: "CoreProfiles".to_string(),
                path: "/profiles".to_string(),
                description: "Profile change event channel".to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: common_messages(),
            },
            WebSocketEndpoint {
                id: "CoreEvents".to_string(),
                path: "/events".to_string(),
                description: "General asynchronous event channel".to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: common_messages(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_ws_has_expected_endpoints() {
        let api = define_unfolded_circle_core_ws_api();
        assert_eq!(api.endpoints.len(), 4);
        assert!(api.endpoints.iter().any(|ep| ep.path == "/ws"));
        assert!(api.endpoints.iter().any(|ep| ep.path == "/intg"));
        assert!(api.endpoints.iter().any(|ep| ep.path == "/profiles"));
        assert!(api.endpoints.iter().any(|ep| ep.path == "/events"));
    }
}
