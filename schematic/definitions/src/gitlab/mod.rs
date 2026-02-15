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

mod types;

pub use types::*;

use schematic_define::{
    ApiKeyEnv, ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi, RestMethod,
};

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
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::gitlab::define_gitlab_api;
///
/// let api = define_gitlab_api();
/// assert_eq!(api.name, "GitLab");
/// assert_eq!(api.endpoints.len(), 15);
/// ```
pub fn define_gitlab_api() -> RestApi {
    RestApi {
        name: "GitLab".to_string(),
        description: "GitLab REST API v4 for repository, MR, issue, and release workflows"
            .to_string(),
        base_url: "https://gitlab.com/api/v4".to_string(),
        docs_url: Some("https://docs.gitlab.com/api/rest/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "PRIVATE-TOKEN".to_string(),
        },
        env_auth: vec![
            "GITLAB_TOKEN".to_string(),
            "GITLAB_PRIVATE_TOKEN".to_string(),
        ],
        env_username: None,
        headers: vec![(
            "User-Agent".to_string(),
            "schematic-gitlab-client".to_string(),
        )],
        endpoints: vec![
            // =================================================================
            // Repository Tree & Files
            // =================================================================
            Endpoint {
                id: "ListRepositoryTree".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/repository/tree?per_page=100&recursive=true".to_string(),
                description: "List repository tree recursively (files and directories)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("TreeItem"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetRepositoryFile".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/repository/files/{file_path}?ref={git_ref}".to_string(),
                description: "Get file content (Base64 encoded, provide git_ref for branch/tag)"
                    .to_string(),
                request: None,
                response: ApiResponse::json_type("FileContent"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Merge Requests
            // =================================================================
            Endpoint {
                id: "ListMergeRequests".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/merge_requests?state=all&per_page=100".to_string(),
                description: "List merge requests with metadata (all states)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("MergeRequest"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetMergeRequest".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/merge_requests/{merge_request_iid}".to_string(),
                description: "Get a single merge request by IID".to_string(),
                request: None,
                response: ApiResponse::json_type("MergeRequest"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListMergeRequestCommits".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/merge_requests/{merge_request_iid}/commits?per_page=100"
                    .to_string(),
                description: "List commits in a merge request".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("Commit"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListMergeRequestChanges".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/merge_requests/{merge_request_iid}/changes".to_string(),
                description: "Get merge request with file changes/diffs".to_string(),
                request: None,
                response: ApiResponse::json_type("MergeRequestChanges"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Issues
            // =================================================================
            Endpoint {
                id: "ListIssues".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/issues?state=all&per_page=100".to_string(),
                description: "List issues (all states)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("Issue"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetIssue".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/issues/{issue_iid}".to_string(),
                description: "Get a single issue by IID".to_string(),
                request: None,
                response: ApiResponse::json_type("Issue"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListIssueNotes".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/issues/{issue_iid}/notes?per_page=100".to_string(),
                description: "List comments/notes on an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("Note"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListIssueParticipants".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/issues/{issue_iid}/participants".to_string(),
                description: "List participants on an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("User"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Tags and Releases
            // =================================================================
            Endpoint {
                id: "ListTags".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/repository/tags?per_page=100".to_string(),
                description: "List repository tags (check `release` field for release tags)"
                    .to_string(),
                request: None,
                response: ApiResponse::json_vec_type("Tag"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetTag".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/repository/tags/{tag_name}".to_string(),
                description: "Get a single tag by name".to_string(),
                request: None,
                response: ApiResponse::json_type("Tag"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListReleases".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/releases?per_page=100".to_string(),
                description: "List releases".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("Release"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetRelease".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/releases/{tag_name}".to_string(),
                description: "Get a single release by tag name".to_string(),
                request: None,
                response: ApiResponse::json_type("Release"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetLatestRelease".to_string(),
                method: RestMethod::Get,
                path: "/projects/{id}/releases/permalink/latest".to_string(),
                description: "Get the latest release".to_string(),
                request: None,
                response: ApiResponse::json_type("Release"),
                headers: vec![],
                params: None,
            },
        ],
        module_path: Some("gitlab".to_string()),
        request_suffix: None,
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
            AuthStrategy::ApiKey { header } => {
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
    fn api_has_fifteen_endpoints() {
        let api = define_gitlab_api();
        assert_eq!(api.endpoints.len(), 15);
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
        assert!(endpoint.path.contains("recursive=true"));
        assert!(endpoint.request.is_none());
        match &endpoint.response {
            ApiResponse::Json(schema) => {
                assert!(schema.type_name.starts_with("Vec<"));
            }
            _ => panic!("Expected JSON response"),
        }
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
        assert!(list.path.contains("state=all"));
        assert!(list.path.contains("per_page=100"));

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
        assert!(list.path.contains("state=all"));

        let get = api.endpoints.iter().find(|e| e.id == "GetIssue").unwrap();
        assert!(get.path.contains("{issue_iid}"));

        let notes = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueNotes")
            .unwrap();
        assert!(notes.path.contains("/notes"));

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

        let release = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetRelease")
            .unwrap();
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
        assert!(api_key
            .names
            .names()
            .contains(&"GITLAB_PRIVATE_TOKEN".to_string()));
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
        ];

        for id in list_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            match &endpoint.response {
                ApiResponse::Json(schema) => {
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
        ];

        for id in single_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            match &endpoint.response {
                ApiResponse::Json(schema) => {
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
}
