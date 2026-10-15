use super::super::*;
use schematic_define::{
    Endpoint,
    params::{EndpointParams, QueryParamType},
};

/// Returns all project-related endpoints (repository tree, files, project metadata, group projects).
pub fn all(ctx: &mut schematic_define::DefinitionContext) -> Result<Vec<Endpoint>, schematic_define::DefinitionError> {
    Ok(vec![
        Endpoint::get("ListRepositoryTree", "/projects/{id}/repository/tree")
                .describes("List repository tree recursively (files and directories)")
                .returns_json::<Vec<TreeItem>>(ctx)
                .params(EndpointParams::default()
                    .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)"))
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
                    ),)
                .pagination(Collection::root().or_paginate("keyset", super::super::gitlab_keyset()))
                .build()?,
        Endpoint::get("GetRepositoryFile", "/projects/{id}/repository/files/{file_path}")
                .describes("Get file content (Base64 encoded, provide git_ref for branch/tag)")
                .returns_json::<FileContent>(ctx)
                .query(schematic_define::ParamDef::new("ref", QueryParamType::String).required().rust_name("git_ref"))
                .build()?,
        Endpoint::get("GetProject", "/projects/{id}")
                .describes("Get project metadata (use URL-encoded path or numeric ID)")
                .returns_json::<Project>(ctx)
                .build()?,
        Endpoint::get("ListGroupProjects", "/groups/{id}/projects")
                .describes("List projects in a group")
                .returns_json::<Vec<Project>>(ctx)
                .params(EndpointParams::default()
                    .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)"))
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
                    ),)
                .pagination(Collection::root().or_paginate("keyset", super::super::gitlab_keyset()))
                .build()?,
    ])
}
