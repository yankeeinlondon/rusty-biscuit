use super::super::*;
use schematic_define::{
    Endpoint,
    params::{EndpointParams, QueryParamType},
};

/// Returns all merge request endpoints (list, get, commits, changes).
pub fn all(ctx: &mut schematic_define::DefinitionContext) -> Result<Vec<Endpoint>, schematic_define::DefinitionError> {
    Ok(vec![
        Endpoint::get("ListMergeRequests", "/projects/{id}/merge_requests")
                .describes("List merge requests with metadata")
                .returns_json::<Vec<MergeRequest>>(ctx)
                .params(EndpointParams::default()
                    .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)"))
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
                    ),)
                .build()?,
        Endpoint::get("GetMergeRequest", "/projects/{id}/merge_requests/{merge_request_iid}")
                .describes("Get a single merge request by IID")
                .returns_json::<MergeRequest>(ctx)
                .build()?,
        Endpoint::get("ListMergeRequestCommits", "/projects/{id}/merge_requests/{merge_request_iid}/commits")
                .describes("List commits in a merge request")
                .returns_json::<Vec<Commit>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)")))
                .build()?,
        Endpoint::get("ListMergeRequestChanges", "/projects/{id}/merge_requests/{merge_request_iid}/changes")
                .describes("Get merge request with file changes/diffs")
                .returns_json::<MergeRequestChanges>(ctx)
                .build()?,
    ])
}
