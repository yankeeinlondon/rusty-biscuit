use schematic_define::{
    ApiResponse, Endpoint,
    params::{EndpointParams, PaginationStyle, QueryParamType},
};

/// Returns all CI/CD pipeline endpoints.
pub fn all() -> Vec<Endpoint> {
    vec![Endpoint {
        id: "ListProjectPipelines".to_string(),
        method: schematic_define::RestMethod::Get,
        path: "/projects/{id}/pipelines".to_string(),
        description: "List CI/CD pipelines for a project".to_string(),
        request: None,
        response: ApiResponse::json_vec_type("Pipeline"),
        headers: vec![],
        params: Some(
            EndpointParams::default()
                .with_pagination(PaginationStyle::gitlab())
                .with_query_param(
                    "status",
                    QueryParamType::Enum(vec![
                        "created".to_string(),
                        "waiting_for_resource".to_string(),
                        "preparing".to_string(),
                        "pending".to_string(),
                        "running".to_string(),
                        "success".to_string(),
                        "failed".to_string(),
                        "canceled".to_string(),
                        "skipped".to_string(),
                        "manual".to_string(),
                        "scheduled".to_string(),
                    ]),
                    false,
                    Some("Filter by pipeline status"),
                )
                .with_query_param(
                    "source",
                    QueryParamType::String,
                    false,
                    Some("Filter by pipeline source (e.g., push, merge_request_event, schedule)"),
                )
                .with_query_param(
                    "git_ref",
                    QueryParamType::String,
                    false,
                    Some("Filter by branch or tag name"),
                )
                .with_query_param(
                    "sha",
                    QueryParamType::String,
                    false,
                    Some("Filter by commit SHA"),
                ),
        ),
        oauth_scopes: None,
    }]
}
