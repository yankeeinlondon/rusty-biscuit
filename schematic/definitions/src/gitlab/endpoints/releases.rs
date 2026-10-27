use super::super::*;
use schematic_define::{
    Endpoint,
    params::{EndpointParams, QueryParamType},
};

/// Returns all tag and release endpoints (list tags, get tag, list releases, get release, latest release).
pub fn all(ctx: &mut schematic_define::DefinitionContext) -> Result<Vec<Endpoint>, schematic_define::DefinitionError> {
    Ok(vec![
        Endpoint::get("ListTags", "/projects/{id}/repository/tags")
                .describes("List repository tags (check `release` field for release tags)")
                .returns_json::<Vec<Tag>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)")))
                .build()?,
        Endpoint::get("GetTag", "/projects/{id}/repository/tags/{tag_name}")
                .describes("Get a single tag by name")
                .returns_json::<Tag>(ctx)
                .build()?,
        Endpoint::get("ListReleases", "/projects/{id}/releases")
                .describes("List releases")
                .returns_json::<Vec<Release>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("per_page", QueryParamType::Integer, false, Some("Items per page (default: 100, max: 100)")))
                .build()?,
        Endpoint::get("GetRelease", "/projects/{id}/releases/{tag_name}")
                .describes("Get a single release by tag name")
                .returns_json::<Release>(ctx)
                .build()?,
        Endpoint::get("GetLatestRelease", "/projects/{id}/releases/permalink/latest")
                .describes("Get the latest release")
                .returns_json::<Release>(ctx)
                .build()?,
    ])
}
