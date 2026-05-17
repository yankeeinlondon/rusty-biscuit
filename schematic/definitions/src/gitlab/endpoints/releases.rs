use schematic_define::{
    ApiResponse, Endpoint,
    params::{EndpointParams, PaginationStyle},
};

/// Returns all tag and release endpoints (list tags, get tag, list releases, get release, latest release).
pub fn all() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "ListTags".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/repository/tags".to_string(),
            description: "List repository tags (check `release` field for release tags)"
                .to_string(),
            request: None,
            response: ApiResponse::json_vec_type("Tag"),
            headers: vec![],
            params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitlab())),
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetTag".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/repository/tags/{tag_name}".to_string(),
            description: "Get a single tag by name".to_string(),
            request: None,
            response: ApiResponse::json_type("Tag"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListReleases".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/releases".to_string(),
            description: "List releases".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("Release"),
            headers: vec![],
            params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitlab())),
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetRelease".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/releases/{tag_name}".to_string(),
            description: "Get a single release by tag name".to_string(),
            request: None,
            response: ApiResponse::json_type("Release"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetLatestRelease".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/releases/permalink/latest".to_string(),
            description: "Get the latest release".to_string(),
            request: None,
            response: ApiResponse::json_type("Release"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
