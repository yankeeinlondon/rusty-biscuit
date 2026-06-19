use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestMethod};

pub fn all() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "CreateSoundEffect".to_string(),
            method: RestMethod::Post,
            path: "/v1/sound-generation".to_string(),
            description: "Generates a sound effect from text".to_string(),
            request: Some(ApiRequest::json_type("CreateSoundEffectBody")),
            response: ApiResponse::Binary,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListModels".to_string(),
            method: RestMethod::Get,
            path: "/v1/models".to_string(),
            description: "Lists all available models".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("ModelInfo"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "CreateSingleUseToken".to_string(),
            method: RestMethod::Post,
            path: "/v1/single-use-token/{token_type}".to_string(),
            description: "Creates a single-use token for WebSocket auth".to_string(),
            request: None,
            response: ApiResponse::json_type("SingleUseTokenResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
