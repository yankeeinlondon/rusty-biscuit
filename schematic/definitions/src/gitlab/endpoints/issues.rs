use schematic_define::{
    ApiResponse, Endpoint,
    params::{EndpointParams, PaginationStyle, QueryParamType},
};

/// Returns all issue endpoints (list, get, notes, participants).
pub fn all() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "ListIssues".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/issues".to_string(),
            description: "List issues".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("Issue"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_pagination(PaginationStyle::gitlab())
                    .with_query_param(
                        "state",
                        QueryParamType::Enum(vec![
                            "opened".to_string(),
                            "closed".to_string(),
                            "all".to_string(),
                        ]),
                        false,
                        Some("Filter by issue state"),
                    ),
            ),
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetIssue".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/issues/{issue_iid}".to_string(),
            description: "Get a single issue by IID".to_string(),
            request: None,
            response: ApiResponse::json_type("Issue"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListIssueNotes".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/issues/{issue_iid}/notes".to_string(),
            description: "List comments/notes on an issue".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("Note"),
            headers: vec![],
            params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitlab())),
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListIssueParticipants".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/issues/{issue_iid}/participants".to_string(),
            description: "List participants on an issue".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("User"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
