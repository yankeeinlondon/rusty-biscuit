use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestMethod};

/// Messaging endpoints (subscriptions, publishing, topics, retained messages).
pub fn messaging_endpoints() -> Vec<Endpoint> {
    vec![
        // Subscriptions
        Endpoint {
            id: "ListSubscriptions".to_string(),
            method: RestMethod::Get,
            path: "/subscriptions".to_string(),
            description: "List all subscriptions across the cluster".to_string(),
            request: None,
            response: ApiResponse::json_type("ListSubscriptionsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Publishing
        Endpoint {
            id: "Publish".to_string(),
            method: RestMethod::Post,
            path: "/publish".to_string(),
            description: "Publish an MQTT message to a topic".to_string(),
            request: Some(ApiRequest::json_type("PublishBody")),
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "PublishBulk".to_string(),
            method: RestMethod::Post,
            path: "/publish/bulk".to_string(),
            description: "Publish multiple MQTT messages in a single request".to_string(),
            request: Some(ApiRequest::json_type("PublishBatchBody")),
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Topics
        Endpoint {
            id: "ListTopics".to_string(),
            method: RestMethod::Get,
            path: "/topics".to_string(),
            description: "List active topics in the broker".to_string(),
            request: None,
            response: ApiResponse::json_type("ListTopicsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Retained Messages
        Endpoint {
            id: "ListRetained".to_string(),
            method: RestMethod::Get,
            path: "/retainer/messages".to_string(),
            description: "List retained messages with pagination".to_string(),
            request: None,
            response: ApiResponse::json_type("ListRetainedResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetRetained".to_string(),
            method: RestMethod::Get,
            path: "/retainer/messages/{topic}".to_string(),
            description: "Get a specific retained message by topic".to_string(),
            request: None,
            response: ApiResponse::json_type("RetainedMessage"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "DeleteRetained".to_string(),
            method: RestMethod::Delete,
            path: "/retainer/messages/{topic}".to_string(),
            description: "Delete a retained message".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
