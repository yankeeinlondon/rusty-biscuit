use schematic_define::{
    ApiResponse, Endpoint,
    params::{EndpointParams, PaginationStyle, QueryParamType},
};

/// Returns all project-related endpoints (repository tree, files, project metadata, group projects).
pub fn all() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "ListRepositoryTree".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/repository/tree".to_string(),
            description: "List repository tree recursively (files and directories)".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("TreeItem"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_pagination(PaginationStyle::gitlab())
                    .with_query_param(
                        "recursive",
                        QueryParamType::Boolean,
                        false,
                        Some("Include files from subdirectories recursively"),
                    )
                    .with_query_param(
                        "git_ref",
                        QueryParamType::String,
                        false,
                        Some("Branch, tag, or commit SHA to list tree for"),
                    )
                    .with_query_param(
                        "path",
                        QueryParamType::String,
                        false,
                        Some("Subdirectory path to list"),
                    ),
            ),
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetRepositoryFile".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}/repository/files/{file_path}?ref={git_ref}".to_string(),
            description: "Get file content (Base64 encoded, provide git_ref for branch/tag)"
                .to_string(),
            request: None,
            response: ApiResponse::json_type("FileContent"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetProject".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/projects/{id}".to_string(),
            description: "Get project metadata (use URL-encoded path or numeric ID)".to_string(),
            request: None,
            response: ApiResponse::json_type("Project"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListGroupProjects".to_string(),
            method: schematic_define::RestMethod::Get,
            path: "/groups/{id}/projects".to_string(),
            description: "List projects in a group".to_string(),
            request: None,
            response: ApiResponse::json_vec_type("Project"),
            headers: vec![],
            params: Some(
                EndpointParams::default()
                    .with_pagination(PaginationStyle::gitlab())
                    .with_query_param(
                        "archived",
                        QueryParamType::Boolean,
                        false,
                        Some("Filter by archived status"),
                    )
                    .with_query_param(
                        "visibility",
                        QueryParamType::Enum(vec![
                            "public".to_string(),
                            "internal".to_string(),
                            "private".to_string(),
                        ]),
                        false,
                        Some("Filter by visibility level"),
                    )
                    .with_query_param(
                        "order_by",
                        QueryParamType::Enum(vec![
                            "id".to_string(),
                            "name".to_string(),
                            "path".to_string(),
                            "created_at".to_string(),
                            "updated_at".to_string(),
                            "last_activity_at".to_string(),
                        ]),
                        false,
                        Some("Sort field"),
                    )
                    .with_query_param(
                        "sort",
                        QueryParamType::Enum(vec!["asc".to_string(), "desc".to_string()]),
                        false,
                        Some("Sort direction"),
                    )
                    .with_query_param(
                        "include_subgroups",
                        QueryParamType::Boolean,
                        false,
                        Some("Include projects from subgroups"),
                    ),
            ),
            oauth_scopes: None,
        },
    ]
}
