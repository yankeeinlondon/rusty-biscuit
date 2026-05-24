use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestMethod};

/// Client management endpoints (list, get, disconnect, subscribe, unsubscribe).
pub fn client_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "ListClients".to_string(),
            method: RestMethod::Get,
            path: "/clients".to_string(),
            description: "List connected MQTT clients with pagination".to_string(),
            request: None,
            response: ApiResponse::json_type("ListClientsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetClient".to_string(),
            method: RestMethod::Get,
            path: "/clients/{clientid}".to_string(),
            description: "Get detailed information about a specific client".to_string(),
            request: None,
            response: ApiResponse::json_type("ClientInfo"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "DisconnectClient".to_string(),
            method: RestMethod::Delete,
            path: "/clients/{clientid}".to_string(),
            description: "Forcefully disconnect a client from the broker".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "SubscribeClient".to_string(),
            method: RestMethod::Post,
            path: "/clients/{clientid}/subscribe".to_string(),
            description: "Create a subscription for a connected client".to_string(),
            request: Some(ApiRequest::json_type("SubscribeBody")),
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "UnsubscribeClient".to_string(),
            method: RestMethod::Post,
            path: "/clients/{clientid}/unsubscribe".to_string(),
            description: "Remove a subscription from a connected client".to_string(),
            request: Some(ApiRequest::json_type("SubscribeBody")),
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
