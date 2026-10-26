use super::super::*;
use schematic_define::{
    Endpoint,
    params::{EndpointParams, QueryParamType},
};

/// Returns all CI/CD pipeline endpoints.
pub fn all(ctx: &mut schematic_define::DefinitionContext) -> Result<Vec<Endpoint>, schematic_define::DefinitionError> {
    Ok(vec![Endpoint::get("ListProjectPipelines", "/projects/{id}/pipelines")
                .describes("List CI/CD pipelines for a project")
                .returns_json::<Vec<Pipeline>>(ctx)
                .params(EndpointParams::default()
                .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)"))
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
                ),)
                .build()?])
}
