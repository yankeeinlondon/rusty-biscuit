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
//! | Tags/Releases | `ListTags`, `GetTag`, `ListReleases`, `GetRelease`, `GetLatestRelease` |
//! | Projects | `GetProject` |
//! | Pipelines | `ListProjectPipelines` |
//! | Groups | `ListGroupProjects` |

mod endpoints;
mod types;

pub use types::*;

use crate::registry::SchemaRegistry;
#[cfg(test)]
use schematic_define::RestMethod;
use schematic_define::{ApiKeyEnv, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi};

/// Creates a schema registry containing all GitLab response types.
///
/// This registry can be used to generate OpenAPI schemas for the GitLab API.
/// All response types used by the API endpoints are registered.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::gitlab::{openapi_registry, define_gitlab_api};
///
/// let registry = openapi_registry();
/// let api = define_gitlab_api();
///
/// // Registry contains all response types
/// assert!(registry.get("Project").is_some());
/// assert!(registry.get("MergeRequest").is_some());
/// assert!(registry.get("Issue").is_some());
///
/// // Registry is complete for the API
/// assert!(registry.validate_completeness(&api).is_ok());
/// ```
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        .register::<TreeItem>("TreeItem")
        .register::<Vec<TreeItem>>("Vec<TreeItem>")
        .register::<FileContent>("FileContent")
        .register::<MergeRequest>("MergeRequest")
        .register::<Vec<MergeRequest>>("Vec<MergeRequest>")
        .register::<Commit>("Commit")
        .register::<Vec<Commit>>("Vec<Commit>")
        .register::<MergeRequestChanges>("MergeRequestChanges")
        .register::<Issue>("Issue")
        .register::<Vec<Issue>>("Vec<Issue>")
        .register::<Note>("Note")
        .register::<Vec<Note>>("Vec<Note>")
        .register::<User>("User")
        .register::<Vec<User>>("Vec<User>")
        .register::<Tag>("Tag")
        .register::<Vec<Tag>>("Vec<Tag>")
        .register::<Release>("Release")
        .register::<Vec<Release>>("Vec<Release>")
        .register::<Project>("Project")
        .register::<Vec<Project>>("Vec<Project>")
        .register::<Pipeline>("Pipeline")
        .register::<Vec<Pipeline>>("Vec<Pipeline>")
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
/// assert_eq!(api.endpoints.len(), 18);
/// ```
pub fn define_gitlab_api() -> RestApi {
    let endpoints: Vec<Endpoint> = vec![
        endpoints::projects::all(),
        endpoints::merge_requests::all(),
        endpoints::issues::all(),
        endpoints::releases::all(),
        endpoints::pipelines::all(),
    ]
    .into_iter()
    .flatten()
    .collect();

    RestApi {
        name: "GitLab".to_string(),
        description: "GitLab REST API v4 for repository, MR, issue, and release workflows"
            .to_string(),
        base_url: "https://gitlab.com/api/v4".to_string(),
        docs_url: Some("https://docs.gitlab.com/api/rest/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "PRIVATE-TOKEN".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: vec![
            "GITLAB_TOKEN".to_string(),
            "GITLAB_PRIVATE_TOKEN".to_string(),
        ],
        env_username: None,
        headers: vec![(
            "User-Agent".to_string(),
            "schematic-gitlab-client".to_string(),
        )],
        endpoints,
        module_path: Some("gitlab".to_string()),
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                header: "PRIVATE-TOKEN".to_string(),
                names: EnvList::new(vec![
                    "GITLAB_TOKEN".to_string(),
                    "GITLAB_PRIVATE_TOKEN".to_string(),
                ]),
            }),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================
    // openapi_registry() tests
    // =============================================

    #[test]
    fn openapi_registry_contains_all_response_types() {
        let registry = openapi_registry();

        assert!(registry.get("Project").is_some());
        assert!(registry.get("Vec<Project>").is_some());
        assert!(registry.get("TreeItem").is_some());
        assert!(registry.get("Vec<TreeItem>").is_some());
        assert!(registry.get("FileContent").is_some());
        assert!(registry.get("MergeRequest").is_some());
        assert!(registry.get("Vec<MergeRequest>").is_some());
        assert!(registry.get("Issue").is_some());
        assert!(registry.get("Tag").is_some());
        assert!(registry.get("Release").is_some());
        assert!(registry.get("Pipeline").is_some());
        assert_eq!(registry.len(), 22);
    }

    #[test]
    fn openapi_registry_validates_against_api() {
        let registry = openapi_registry();
        let api = define_gitlab_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    #[test]
    fn openapi_registry_converts_to_openapi_schemas() {
        let registry = openapi_registry();
        let openapi_schemas = registry.to_openapi_schemas();

        assert!(
            openapi_schemas.len() >= 22,
            "expected at least 22 schemas, got {}",
            openapi_schemas.len()
        );
        assert!(openapi_schemas.contains_key("Project"));
        assert!(openapi_schemas.contains_key("Vec<Project>"));
        assert!(openapi_schemas.contains_key("MergeRequest"));
        assert!(openapi_schemas.contains_key("Vec<MergeRequest>"));
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
    fn api_has_eighteen_endpoints() {
        let api = define_gitlab_api();
        assert_eq!(api.endpoints.len(), 18);
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
        match &endpoint.response {
            schematic_define::ApiResponse::Json(schema) => {
                assert!(schema.type_name.starts_with("Vec<"));
            }
            _ => panic!("Expected JSON response"),
        }

        // Verify pagination and query params are in params
        let params = endpoint.params.as_ref().expect("params should be set");
        assert!(params.has_pagination());
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
        assert!(endpoint.path.contains("ref={git_ref}"));
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
        assert!(list_params.has_pagination());
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
        assert!(commits_params.has_pagination());

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
        assert!(list_params.has_pagination());
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
        assert!(notes_params.has_pagination());

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
            match &endpoint.response {
                schematic_define::ApiResponse::Json(schema) => {
                    assert!(
                        schema.type_name.starts_with("Vec<"),
                        "Endpoint {} should return Vec type, got {}",
                        id,
                        schema.type_name
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
            "GetTag",
            "GetRelease",
            "GetLatestRelease",
            "GetProject",
        ];

        for id in single_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            match &endpoint.response {
                schematic_define::ApiResponse::Json(schema) => {
                    assert!(
                        !schema.type_name.starts_with("Vec<"),
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
        use schematic_define::params::PaginationStyle;

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
                params.has_pagination(),
                "Endpoint {} should have pagination",
                id
            );

            // Verify GitLab pagination style (page + per_page)
            match &params.pagination {
                Some(PaginationStyle::PageNumber {
                    page_param,
                    per_page_param,
                    default_per_page,
                    max_per_page,
                }) => {
                    assert_eq!(page_param, "page", "Endpoint {} page_param", id);
                    assert_eq!(per_page_param, "per_page", "Endpoint {} per_page_param", id);
                    assert_eq!(*default_per_page, 100, "Endpoint {} default_per_page", id);
                    assert_eq!(*max_per_page, 100, "Endpoint {} max_per_page", id);
                }
                _ => panic!("Endpoint {} should use PageNumber pagination", id),
            }
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

        match &endpoint.response {
            schematic_define::ApiResponse::Json(schema) => {
                assert_eq!(schema.type_name, "Project");
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

        match &endpoint.response {
            schematic_define::ApiResponse::Json(schema) => {
                assert!(schema.type_name.starts_with("Vec<"));
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.has_pagination());
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

        match &endpoint.response {
            schematic_define::ApiResponse::Json(schema) => {
                assert!(schema.type_name.starts_with("Vec<"));
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.has_pagination());
        assert!(params.query.iter().any(|p| p.name == "archived"));
        assert!(params.query.iter().any(|p| p.name == "visibility"));
        assert!(params.query.iter().any(|p| p.name == "include_subgroups"));
    }

    #[test]
    fn no_hardcoded_query_params_in_paths() {
        let api = define_gitlab_api();

        for endpoint in &api.endpoints {
            // GetRepositoryFile is allowed to have ?ref= as a path parameter
            if endpoint.id == "GetRepositoryFile" {
                continue;
            }

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
}
