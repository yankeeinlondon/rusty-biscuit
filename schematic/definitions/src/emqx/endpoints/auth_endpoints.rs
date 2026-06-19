use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestMethod};

/// Authentication and authorization endpoints (authenticators, users, authz sources).
pub fn auth_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "ListAuthenticators".to_string(),
            method: RestMethod::Get,
            path: "/authentication".to_string(),
            description: "List all configured authentication providers".to_string(),
            request: None,
            response: ApiResponse::json_type("ListAuthenticatorsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetAuthenticator".to_string(),
            method: RestMethod::Get,
            path: "/authentication/{id}".to_string(),
            description: "Get details of a specific authenticator".to_string(),
            request: None,
            response: ApiResponse::json_type("AuthenticatorInfo"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListAuthUsers".to_string(),
            method: RestMethod::Get,
            path: "/authentication/{id}/users".to_string(),
            description: "List users in a built-in database authenticator".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("AuthUser"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "CreateAuthUser".to_string(),
            method: RestMethod::Post,
            path: "/authentication/{id}/users".to_string(),
            description: "Create a new user in a built-in database authenticator".to_string(),
            request: Some(ApiRequest::json_type("CreateAuthUserBody")),
            response: ApiResponse::json_type("AuthUser"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "DeleteAuthUser".to_string(),
            method: RestMethod::Delete,
            path: "/authentication/{id}/users/{user_id}".to_string(),
            description: "Delete a user from a built-in database authenticator".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Authorization
        Endpoint {
            id: "ListAuthzSources".to_string(),
            method: RestMethod::Get,
            path: "/authorization/sources".to_string(),
            description: "List all authorization sources".to_string(),
            request: None,
            response: ApiResponse::json_type("ListAuthzSourcesResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
