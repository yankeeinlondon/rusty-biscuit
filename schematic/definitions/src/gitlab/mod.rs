//! GitLab REST API definition.
//!
//! This module provides a focused definition of the GitLab REST API v4,
//! optimized for common developer workflows: README discovery, merge requests,
//! issues, and tag/release management.
//!
//! ## API Overview
//!
//! The GitLab REST API provides access to:
//!
//! - **Repository Tree**: Recursive file discovery
//! - **Repository Files**: Raw file content (Base64 encoded)
//! - **Merge Requests**: List MRs with associated metadata, commits, and changes
//! - **Issues**: List issues, comments, participants, time stats
//! - **Branches**: A branch and its head commit
//! - **Tags & Releases**: Distinguish regular tags from release tags
//!
//! ## Authentication
//!
//! Uses `PRIVATE-TOKEN` header authentication via `GITLAB_TOKEN` or
//! `GITLAB_PRIVATE_TOKEN` environment variables.
//!
//! **Important:** GitLab uses a custom header (`PRIVATE-TOKEN`) instead of Bearer token.
//!
//! ## GitLab-Specific Patterns
//!
//! - Project paths must be URL-encoded (`%2F` for `/`)
//! - File content is Base64 encoded
//! - Uses `iid` (internal ID) scoped to project, not global `id`
//! - Releases are optional metadata on tags (unlike GitHub's separate entities)
//!
//! ## Endpoint Coverage (V1)
//!
//! | Category | Endpoints |
//! |----------|-----------|
//! | Repository | `ListRepositoryTree`, `GetRepositoryFile` |
//! | Merge Requests | `ListMergeRequests`, `GetMergeRequest`, `ListMergeRequestCommits`, `ListMergeRequestChanges` |
//! | Issues | `ListIssues`, `GetIssue`, `ListIssueNotes`, `ListIssueParticipants` |
//! | Branches | `GetBranch` |
//! | Tags/Releases | `ListTags`, `GetTag`, `ListReleases`, `GetRelease`, `GetLatestRelease` |
//! | Projects | `GetProject` |
//! | Pipelines | `ListProjectPipelines` |
//! | Groups | `ListGroupProjects` |

mod endpoints;
mod types;

pub use types::*;

use crate::registry::assemble_module_group;
use schematic_define::{
    ApiKeyEnv, EnvList, EnvMapping, AuthStrategy, DefinitionError, Endpoint,
    RestApi,
    pagination::{Collection, LinkSource, MoreSignal, PageSize, Strategy, ValueSource},
};

/// GitLab's page size: `per_page`, at most 100, 20 when omitted.
fn gitlab_page_size() -> PageSize {
    PageSize::new("per_page").server_default(20).max(100)
}

/// GitLab's offset-based pagination, modeled as a page-number strategy: a
/// 1-based `page`, ended when the response carries no `X-Next-Page` header,
/// with the item count in `X-Total`.
fn gitlab_page_number() -> Strategy {
    Strategy::page_number("page", MoreSignal::header_present("X-Next-Page"))
        .page_size(gitlab_page_size())
        .total(ValueSource::header("X-Total"))
}

/// GitLab's keyset pagination, an alternate to [`gitlab_page_number`] on the
/// endpoints that support it: selected by the constants, continued through
/// the `Link` header.
fn gitlab_keyset() -> Strategy {
    Strategy::next_link(LinkSource::link_header("next"))
        .page_size(gitlab_page_size())
        .constant("pagination", "keyset")
        .constant("order_by", "id")
        .constant("sort", "asc")
}

/// Creates the GitLab REST API definition.
///
/// This defines a focused slice of the GitLab REST API covering repository
/// files, merge requests, issues, tags, and releases.
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | ListRepositoryTree | GET | /projects/{id}/repository/tree | List repository tree |
/// | GetRepositoryFile | GET | /projects/{id}/repository/files/{file_path} | Get file content |
/// | ListMergeRequests | GET | /projects/{id}/merge_requests | List merge requests |
/// | GetMergeRequest | GET | /projects/{id}/merge_requests/{merge_request_iid} | Get single MR |
/// | ListMergeRequestCommits | GET | /projects/{id}/merge_requests/{merge_request_iid}/commits | List MR commits |
/// | ListMergeRequestChanges | GET | /projects/{id}/merge_requests/{merge_request_iid}/changes | List MR changes |
/// | ListIssues | GET | /projects/{id}/issues | List issues |
/// | GetIssue | GET | /projects/{id}/issues/{issue_iid} | Get single issue |
/// | ListIssueNotes | GET | /projects/{id}/issues/{issue_iid}/notes | List issue comments |
/// | ListIssueParticipants | GET | /projects/{id}/issues/{issue_iid}/participants | List issue participants |
/// | GetBranch | GET | /projects/{id}/repository/branches/{branch} | Get single branch |
/// | ListTags | GET | /projects/{id}/repository/tags | List repository tags |
/// | GetTag | GET | /projects/{id}/repository/tags/{tag_name} | Get single tag |
/// | ListReleases | GET | /projects/{id}/releases | List releases |
/// | GetRelease | GET | /projects/{id}/releases/{tag_name} | Get single release |
/// | GetLatestRelease | GET | /projects/{id}/releases/permalink/latest | Get latest release |
/// | GetProject | GET | /projects/{id} | Get project metadata |
/// | ListProjectPipelines | GET | /projects/{id}/pipelines | List CI/CD pipelines |
/// | ListGroupProjects | GET | /groups/{id}/projects | List group projects |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::gitlab::define_gitlab_api;
///
/// let api = define_gitlab_api();
/// assert_eq!(api.name, "GitLab");
/// assert_eq!(api.endpoints.len(), 19);
/// ```
pub fn define_gitlab_api() -> RestApi {
    module_group().expect("valid gitlab definition").remove(0)
}

/// Assemble the API and its schemas from one ordered registration context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::gitlab", |ctx| {
        ctx.register::<TreeItem>()?;
        ctx.register::<FileContent>()?;
        ctx.register::<MergeRequest>()?;
        ctx.register::<Commit>()?;
        ctx.register::<MergeRequestChanges>()?;
        ctx.register::<Issue>()?;
        ctx.register::<Note>()?;
        ctx.register::<User>()?;
        ctx.register::<Branch>()?;
        ctx.register::<Tag>()?;
        ctx.register::<Release>()?;
        ctx.register::<Project>()?;
        ctx.register::<Pipeline>()?;
        let endpoints: Vec<Endpoint> = vec![
        endpoints::projects::all(ctx)?,
        endpoints::branches::all(ctx)?,
        endpoints::merge_requests::all(ctx)?,
        endpoints::issues::all(ctx)?,
        endpoints::releases::all(ctx)?,
        endpoints::pipelines::all(ctx)?,
    ]
    .into_iter()
    .flatten()
    .collect();
        let api = RestApi::builder(ctx, "GitLab", "https://gitlab.com/api/v4")
            .describes("GitLab REST API v4 for repository, MR, issue, and release workflows")
            .authentication(AuthStrategy::ApiKey {
            header: "PRIVATE-TOKEN".to_string(),
            value_prefix: None,
        }, vec![
            "GITLAB_TOKEN".to_string(),
            "GITLAB_PRIVATE_TOKEN".to_string(),
        ])
            .docs_url("https://docs.gitlab.com/api/rest/")
            .module_path("gitlab".to_string())
            .env_mapping(EnvMapping {
            api_key: Some(ApiKeyEnv {
                header: "PRIVATE-TOKEN".to_string(),
                names: EnvList::new(vec![
                    "GITLAB_TOKEN".to_string(),
                    "GITLAB_PRIVATE_TOKEN".to_string(),
                ]),
            }),
            ..Default::default()
        })
            .default_pagination(gitlab_page_number())
            .header(
            "User-Agent".to_string(),
            "schematic-gitlab-client".to_string(),
        )
            .endpoints(endpoints)
            .build()?;
        Ok(vec![api])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::QueryParamType;
    use schematic_define::{ResponseShape, RestMethod, TypeRef};
    use schematic_define::pagination::{resolve, Resolution};

    // =============================================
    // api.schemas tests
    // =============================================

    #[test]
    fn schemas_contain_all_response_types() {
        let api = define_gitlab_api();

        assert!(api.component("Project").is_some());
        assert!(api.component("Vec<Project>").is_none());
        assert!(api.component("TreeItem").is_some());
        assert!(api.component("Vec<TreeItem>").is_none());
        assert!(api.component("FileContent").is_some());
        assert!(api.component("MergeRequest").is_some());
        assert!(api.component("Vec<MergeRequest>").is_none());
        assert!(api.component("Issue").is_some());
        assert!(api.component("Tag").is_some());
        assert!(api.component("Release").is_some());
        assert!(api.component("Pipeline").is_some());
        assert_eq!(crate::registry::registered_names(&api).len(), 13);
    }

    #[test]
    fn references_resolve_in_api_schemas() {
        let api = define_gitlab_api();

        api.validate_references().expect("every reference resolves in api.schemas");
    }

    #[test]
    fn openapi_export_includes_api_schemas() {
        let api = define_gitlab_api();
        let openapi_schemas = schematic_define::openapi::export(&api, &schematic_define::openapi::ExportOptions::new())
            .expect("export succeeds").components.expect("components").schemas;

        assert!(
            openapi_schemas.len() >= 13,
            "expected at least 13 schemas, got {}",
            openapi_schemas.len()
        );
        assert!(openapi_schemas.contains_key("Project"));
        assert!(!openapi_schemas.contains_key("Vec<Project>"));
        assert!(openapi_schemas.contains_key("MergeRequest"));
        assert!(!openapi_schemas.contains_key("Vec<MergeRequest>"));
        assert!(openapi_schemas.contains_key("Issue"));
        assert!(openapi_schemas.contains_key("Pipeline"));
    }

    // =============================================
    // API definition tests
    // =============================================

    #[test]
    fn api_has_correct_metadata() {
        let api = define_gitlab_api();

        assert_eq!(api.name, "GitLab");
        assert_eq!(api.base_url, "https://gitlab.com/api/v4");
        assert!(api.docs_url.is_some());
        assert!(api.docs_url.as_ref().unwrap().contains("docs.gitlab.com"));
    }

    #[test]
    fn api_uses_api_key_auth_with_private_token() {
        let api = define_gitlab_api();

        match &api.auth {
            AuthStrategy::ApiKey { header, .. } => {
                assert_eq!(header, "PRIVATE-TOKEN");
            }
            _ => panic!("Expected ApiKey auth strategy"),
        }

        assert!(api.env_auth.contains(&"GITLAB_TOKEN".to_string()));
        assert!(api.env_auth.contains(&"GITLAB_PRIVATE_TOKEN".to_string()));
    }

    #[test]
    fn api_has_user_agent_header() {
        let api = define_gitlab_api();

        let user_agent = api.headers.iter().find(|(k, _)| k == "User-Agent");
        assert!(user_agent.is_some());
        assert!(user_agent.unwrap().1.contains("gitlab"));
    }

    #[test]
    fn api_has_nineteen_endpoints() {
        let api = define_gitlab_api();
        assert_eq!(api.endpoints.len(), 19);
    }

    #[test]
    fn repository_tree_endpoint() {
        let api = define_gitlab_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListRepositoryTree")
            .expect("ListRepositoryTree endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert!(endpoint.path.contains("/repository/tree"));
        assert!(
            !endpoint.path.contains("?"),
            "Path should not contain query params"
        );
        assert!(endpoint.request.is_none());
        match &endpoint.response[0].shape {
            ResponseShape::Json(schema) => {
                assert!(matches!(schema, TypeRef::Array(_)));
            }
            _ => panic!("Expected JSON response"),
        }

        // Verify pagination and query params are in params
        let params = endpoint.params.as_ref().expect("params should be set");
        assert!(params.query.iter().any(|p| p.name == "page"));
        assert!(params.query.iter().any(|p| p.name == "recursive"));
        assert!(params.query.iter().any(|p| p.name == "git_ref"));
        assert!(params.query.iter().any(|p| p.name == "path"));
    }

    #[test]
    fn repository_file_endpoint() {
        let api = define_gitlab_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetRepositoryFile")
            .expect("GetRepositoryFile endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert!(endpoint.path.contains("/repository/files/"));
        assert_eq!(endpoint.path, "/projects/{id}/repository/files/{file_path}");
        let query = &endpoint.params.as_ref().unwrap().query;
        assert_eq!(query.len(), 1);
        let git_ref = &query[0];
        assert_eq!(git_ref.name, "ref");
        assert_eq!(git_ref.rust_name.as_deref(), Some("git_ref"));
        assert!(git_ref.required);
        assert_eq!(git_ref.param_type, QueryParamType::String);
        assert!(matches!(&endpoint.response[0].shape, ResponseShape::Json(schema) if *schema == TypeRef::Named("FileContent".into())));
    }

    #[test]
    fn merge_request_endpoints() {
        let api = define_gitlab_api();

        let list = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListMergeRequests")
            .unwrap();
        assert!(
            !list.path.contains("?"),
            "Path should not contain query params"
        );
        let list_params = list.params.as_ref().expect("params should be set");
        assert!(list_params.query.iter().any(|p| p.name == "page"));
        assert!(list_params.query.iter().any(|p| p.name == "state"));

        let get = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetMergeRequest")
            .unwrap();
        assert!(get.path.contains("{merge_request_iid}"));

        let commits = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListMergeRequestCommits")
            .unwrap();
        assert!(commits.path.contains("/commits"));
        assert!(
            !commits.path.contains("?"),
            "Path should not contain query params"
        );
        let commits_params = commits.params.as_ref().expect("params should be set");
        assert!(commits_params.query.iter().any(|p| p.name == "page"));

        let changes = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListMergeRequestChanges")
            .unwrap();
        assert!(changes.path.contains("/changes"));
    }

    #[test]
    fn issue_endpoints() {
        let api = define_gitlab_api();

        let list = api.endpoints.iter().find(|e| e.id == "ListIssues").unwrap();
        assert!(
            !list.path.contains("?"),
            "Path should not contain query params"
        );
        let list_params = list.params.as_ref().expect("params should be set");
        assert!(list_params.query.iter().any(|p| p.name == "page"));
        assert!(list_params.query.iter().any(|p| p.name == "state"));

        let get = api.endpoints.iter().find(|e| e.id == "GetIssue").unwrap();
        assert!(get.path.contains("{issue_iid}"));

        let notes = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueNotes")
            .unwrap();
        assert!(notes.path.contains("/notes"));
        assert!(
            !notes.path.contains("?"),
            "Path should not contain query params"
        );
        let notes_params = notes.params.as_ref().expect("params should be set");
        assert!(notes_params.query.iter().any(|p| p.name == "page"));

        let participants = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueParticipants")
            .unwrap();
        assert!(participants.path.contains("/participants"));
    }

    #[test]
    fn tag_and_release_endpoints() {
        let api = define_gitlab_api();

        let tags = api.endpoints.iter().find(|e| e.id == "ListTags").unwrap();
        assert!(tags.path.contains("/repository/tags"));

        let tag = api.endpoints.iter().find(|e| e.id == "GetTag").unwrap();
        assert!(tag.path.contains("{tag_name}"));

        let releases = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListReleases")
            .unwrap();
        assert!(releases.path.contains("/releases"));

        let release = api.endpoints.iter().find(|e| e.id == "GetRelease").unwrap();
        assert!(release.path.contains("/releases/"));

        let latest = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetLatestRelease")
            .unwrap();
        assert!(latest.path.contains("permalink/latest"));
    }

    #[test]
    fn get_branch_endpoint() {
        let api = define_gitlab_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetBranch")
            .expect("GetBranch endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/projects/{id}/repository/branches/{branch}");
        assert!(endpoint.request.is_none());
        match &endpoint.response[0].shape {
            ResponseShape::Json(schema) => {
                assert!(*schema == TypeRef::Named("Branch".into()));
            }
            _ => panic!("Expected JSON response"),
        }
        assert!(matches!(api.auth, AuthStrategy::ApiKey { .. }));
    }

    #[test]
    fn env_mapping_configured() {
        let api = define_gitlab_api();

        let mapping = api.env_mapping.as_ref().expect("env_mapping should be set");
        let api_key = mapping
            .api_key
            .as_ref()
            .expect("api_key mapping should be set");

        assert_eq!(api_key.header, "PRIVATE-TOKEN");
        assert!(api_key.names.names().contains(&"GITLAB_TOKEN".to_string()));
        assert!(
            api_key
                .names
                .names()
                .contains(&"GITLAB_PRIVATE_TOKEN".to_string())
        );
    }

    #[test]
    fn all_endpoints_have_descriptions() {
        let api = define_gitlab_api();

        for endpoint in &api.endpoints {
            assert!(
                !endpoint.description.is_empty(),
                "Endpoint {} missing description",
                endpoint.id
            );
        }
    }

    #[test]
    fn list_endpoints_return_vec_types() {
        let api = define_gitlab_api();

        let list_endpoints = [
            "ListRepositoryTree",
            "ListMergeRequests",
            "ListMergeRequestCommits",
            "ListIssues",
            "ListIssueNotes",
            "ListIssueParticipants",
            "ListTags",
            "ListReleases",
            "ListProjectPipelines",
            "ListGroupProjects",
        ];

        for id in list_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            match &endpoint.response[0].shape {
                ResponseShape::Json(schema) => {
                    assert!(
                        matches!(schema, TypeRef::Array(_)),
                        "Endpoint {} should return Vec type, got {:?}",
                        id,
                        schema
                    );
                }
                _ => panic!("Endpoint {} should return JSON", id),
            }
        }
    }

    #[test]
    fn single_item_endpoints_return_non_vec_types() {
        let api = define_gitlab_api();

        let single_endpoints = [
            "GetRepositoryFile",
            "GetMergeRequest",
            "ListMergeRequestChanges",
            "GetIssue",
            "GetBranch",
            "GetTag",
            "GetRelease",
            "GetLatestRelease",
            "GetProject",
        ];

        for id in single_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            match &endpoint.response[0].shape {
                ResponseShape::Json(schema) => {
                    assert!(
                        !matches!(schema, TypeRef::Array(_)),
                        "Endpoint {} should return single item, not Vec",
                        id
                    );
                }
                _ => panic!("Endpoint {} should return JSON", id),
            }
        }
    }

    #[test]
    fn module_path_set_correctly() {
        let api = define_gitlab_api();
        assert_eq!(api.module_path, Some("gitlab".to_string()));
    }

    #[test]
    fn list_endpoints_use_gitlab_pagination() {
        let api = define_gitlab_api();

        let paginated_endpoints = [
            "ListRepositoryTree",
            "ListMergeRequests",
            "ListMergeRequestCommits",
            "ListIssues",
            "ListIssueNotes",
            "ListTags",
            "ListReleases",
            "ListProjectPipelines",
            "ListGroupProjects",
        ];

        for id in paginated_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            let params = endpoint
                .params
                .as_ref()
                .unwrap_or_else(|| panic!("Endpoint {} should have params", id));

            assert!(
                params.query.iter().any(|p| p.name == "page"),
                "Endpoint {} should have pagination",
                id
            );

            // Verify GitLab pagination style (page + per_page)
            assert!(params.query.iter().any(|p| p.name == "per_page"));
            let resolution = resolve(&api, endpoint);
            assert!(matches!(resolution, Resolution::Paginated { .. }));
            let knobs = schematic_define::pagination::caller_knobs(&resolution);
            assert!(knobs.iter().any(|k| matches!(k, schematic_define::pagination::Knob::StartPage { .. })));
            assert!(knobs.iter().any(|k| matches!(k, schematic_define::pagination::Knob::PageSize { param, .. } if param == "per_page")));
        }
    }

    #[test]
    fn get_project_endpoint() {
        let api = define_gitlab_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetProject")
            .expect("GetProject endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/projects/{id}");

        match &endpoint.response[0].shape {
            ResponseShape::Json(schema) => {
                assert!(*schema == TypeRef::Named("Project".into()));
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[test]
    fn list_project_pipelines_endpoint() {
        let api = define_gitlab_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListProjectPipelines")
            .expect("ListProjectPipelines endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/projects/{id}/pipelines");

        match &endpoint.response[0].shape {
            ResponseShape::Json(schema) => {
                assert!(matches!(schema, TypeRef::Array(_)));
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.query.iter().any(|p| p.name == "page"));
        assert!(params.query.iter().any(|p| p.name == "status"));
        assert!(params.query.iter().any(|p| p.name == "source"));
        assert!(params.query.iter().any(|p| p.name == "git_ref"));
    }

    #[test]
    fn list_group_projects_endpoint() {
        let api = define_gitlab_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListGroupProjects")
            .expect("ListGroupProjects endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/groups/{id}/projects");

        match &endpoint.response[0].shape {
            ResponseShape::Json(schema) => {
                assert!(matches!(schema, TypeRef::Array(_)));
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.query.iter().any(|p| p.name == "page"));
        assert!(params.query.iter().any(|p| p.name == "archived"));
        assert!(params.query.iter().any(|p| p.name == "visibility"));
        assert!(params.query.iter().any(|p| p.name == "include_subgroups"));
    }

    #[test]
    fn no_hardcoded_query_params_in_paths() {
        let api = define_gitlab_api();

        for endpoint in &api.endpoints {

            assert!(
                !endpoint.path.contains("per_page="),
                "Endpoint {} should not have hardcoded per_page in path",
                endpoint.id
            );
            assert!(
                !endpoint.path.contains("state="),
                "Endpoint {} should not have hardcoded state in path",
                endpoint.id
            );
            assert!(
                !endpoint.path.contains("recursive="),
                "Endpoint {} should not have hardcoded recursive in path",
                endpoint.id
            );
        }
    }
    #[test]
    fn typed_module_group_preserves_endpoint_order_and_completeness() {
        let group = module_group().unwrap();
        assert_eq!(group.len(), 1);
        let api = &group[0];
        assert_eq!(api.endpoints.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), vec!["ListRepositoryTree", "GetRepositoryFile", "GetProject", "ListGroupProjects", "GetBranch", "ListMergeRequests", "GetMergeRequest", "ListMergeRequestCommits", "ListMergeRequestChanges", "ListIssues", "GetIssue", "ListIssueNotes", "ListIssueParticipants", "ListTags", "GetTag", "ListReleases", "GetRelease", "GetLatestRelease", "ListProjectPipelines"]);
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["TreeItem", "FileContent", "MergeRequest", "User", "Commit", "MergeRequestChanges", "Issue", "Note", "Branch", "Tag", "Release", "Project", "Pipeline"]);
        api.validate_references().unwrap();
        assert!(api.endpoints.iter().all(|e| !e.path.contains('?')));
    }

}
