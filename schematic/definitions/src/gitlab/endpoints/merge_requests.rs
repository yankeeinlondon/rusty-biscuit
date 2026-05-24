use schematic_define::{
    ApiResponse, Endpoint,
    params::{EndpointParams, PaginationStyle, QueryParamType},
};

/// Returns all merge request endpoints (list, get, commits, changes).
pub fn all() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "ListMergeRequests".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/merge_requests".to_string(),
            description: "List merge requests with metadata".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("MergeRequest"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_pagination(PaginationStyle::gitlab())
                    .with_query_param(
                        "state",
                        QueryParamType::Enum(vec![
                            "opened".to_string(),
                            "closed".to_string(),
                            "merged".to_string(),
                            "all".to_string(),
                        ]),
                        false,
                        Some("Filter by merge request state"),
                    ),
            ),
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetMergeRequest".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/merge_requests/{merge_request_iid}".to_string(),
            description: "Get a single merge request by IID".to_string(),
            request: None,
            response: ApiResponse::json_type("MergeRequest"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListMergeRequestCommits".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/merge_requests/{merge_request_iid}/commits".to_string(),
            description: "List commits in a merge request".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("Commit"),
            headers: vec![],
            params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitlab())),
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListMergeRequestChanges".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/merge_requests/{merge_request_iid}/changes".to_string(),
            description: "Get merge request with file changes/diffs".to_string(),
            request: None,
            response: ApiResponse::json_type("MergeRequestChanges"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
