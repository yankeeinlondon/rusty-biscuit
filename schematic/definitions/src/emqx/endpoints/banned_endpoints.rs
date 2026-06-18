use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestMethod};

/// Banned clients endpoints (list, create, delete).
pub fn banned_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "ListBanned".to_string(),
            method: RestMethod::Get,
            path: "/banned".to_string(),
            description: "List all banned clients, usernames, and hosts".to_string(),
            request: None,
            response: ApiResponse::json_type("ListBannedResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "CreateBan".to_string(),
            method: RestMethod::Post,
            path: "/banned".to_string(),
            description: "Ban a client, username, or host".to_string(),
            request: Some(ApiRequest::json_type("CreateBanBody")),
            response: ApiResponse::json_type("BanInfo"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "DeleteBan".to_string(),
            method: RestMethod::Delete,
            path: "/banned/{ban_type}/{who}".to_string(),
            description: "Remove a ban by type (clientid, username, peerhost) and value"
                .to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
