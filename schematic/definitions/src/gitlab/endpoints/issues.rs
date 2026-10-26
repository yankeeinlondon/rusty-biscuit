use super::super::*;
use schematic_define::{
    Endpoint,
    params::{EndpointParams, QueryParamType},
};

/// Returns all issue endpoints (list, get, notes, participants).
pub fn all(ctx: &mut schematic_define::DefinitionContext) -> Result<Vec<Endpoint>, schematic_define::DefinitionError> {
    Ok(vec![
        Endpoint::get("ListIssues", "/projects/{id}/issues")
                .describes("List issues")
                .returns_json::<Vec<Issue>>(ctx)
                .params(EndpointParams::default()
                    .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)"))
                    .with_query_param(
                        "state",
                        QueryParamType::Enum(vec![
                            "opened".to_string(),
                            "closed".to_string(),
                            "all".to_string(),
                        ]),
                        false,
                        Some("Filter by issue state"),
                    ),)
                .build()?,
        Endpoint::get("GetIssue", "/projects/{id}/issues/{issue_iid}")
                .describes("Get a single issue by IID")
                .returns_json::<Issue>(ctx)
                .build()?,
        Endpoint::get("ListIssueNotes", "/projects/{id}/issues/{issue_iid}/notes")
                .describes("List comments/notes on an issue")
                .returns_json::<Vec<Note>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)")))
                .build()?,
        Endpoint::get("ListIssueParticipants", "/projects/{id}/issues/{issue_iid}/participants")
                .describes("List participants on an issue")
                .returns_json::<Vec<User>>(ctx)
                .pagination(Collection::root().unpaginated())
                .build()?,
    ])
}
