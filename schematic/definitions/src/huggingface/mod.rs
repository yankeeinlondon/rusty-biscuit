//! Hugging Face Hub API definition.
//!
//! This module provides a complete definition of the Hugging Face Hub API,
//! including REST endpoints for model discovery, dataset management, spaces,
//! repository operations, and user information.
//!
//! ## Endpoints
//!
//! ### Models (8)
//! - `ListModels` - GET /models
//! - `GetModel` - GET /models/{repo_id}
//! - `ListModelFiles` - GET /models/{repo_id}/tree/{revision}
//! - `GetModelFile` - GET /models/{repo_id}/blob/{revision}/{path}
//! - `ListModelCommits` - GET /models/{repo_id}/commits/{revision}
//! - `GetModelReadme` - GET /models/{repo_id}/resolve/{revision}/README.md
//! - `ListModelDiscussions` - GET /models/{repo_id}/discussions
//! - `GetModelCard` - GET /models/{repo_id}/resolve/{revision}/model_card.md
//!
//! ### Datasets (6)
//! - `ListDatasets` - GET /datasets
//! - `GetDataset` - GET /datasets/{repo_id}
//! - `ListDatasetFiles` - GET /datasets/{repo_id}/tree/{revision}
//! - `GetDatasetFile` - GET /datasets/{repo_id}/blob/{revision}/{path}
//! - `ListDatasetCommits` - GET /datasets/{repo_id}/commits/{revision}
//! - `GetDatasetReadme` - GET /datasets/{repo_id}/resolve/{revision}/README.md
//!
//! ### Spaces (4)
//! - `ListSpaces` - GET /spaces
//! - `GetSpace` - GET /spaces/{repo_id}
//! - `ListSpaceFiles` - GET /spaces/{repo_id}/tree/{revision}
//! - `GetSpaceFile` - GET /spaces/{repo_id}/blob/{revision}/{path}
//!
//! ### Repos (4)
//! - `CreateRepo` - POST /repos/create
//! - `DeleteRepo` - DELETE /repos/delete
//! - `UpdateRepoSettings` - PUT /repos/{repo_type}/{repo_id}/settings
//! - `MoveRepo` - POST /repos/move
//!
//! ### User (4)
//! - `WhoAmI` - GET /whoami-v2
//! - `GetUser` - GET /users/{username}
//! - `ListUserRepos` - GET /users/{username}/repos
//! - `GetUserCollections` - GET /users/{username}/collections
//!
//! ## Examples
//!
//! ```rust
//! use schematic_definitions::huggingface::define_huggingface_hub_api;
//!
//! let api = define_huggingface_hub_api();
//! assert_eq!(api.name, "HuggingFaceHub");
//! assert!(api.endpoints.len() >= 26);
//! ```

mod types;

pub use types::*;

use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

/// Creates the Hugging Face Hub API definition.
///
/// This defines the Hugging Face Hub REST API with endpoints for model
/// discovery, dataset management, spaces, repository operations, and
/// user information.
///
/// ## Endpoints
///
/// - **Models**: 8 endpoints (list, get, files, commits, readme, discussions, card)
/// - **Datasets**: 6 endpoints (list, get, files, commits, readme)
/// - **Spaces**: 4 endpoints (list, get, files)
/// - **Repos**: 4 endpoints (create, delete, settings, move)
/// - **User**: 4 endpoints (whoami, get, repos, collections)
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::huggingface::define_huggingface_hub_api;
///
/// let api = define_huggingface_hub_api();
/// assert_eq!(api.name, "HuggingFaceHub");
/// assert_eq!(api.base_url, "https://huggingface.co/api");
/// ```
pub fn define_huggingface_hub_api() -> RestApi {
    RestApi {
        name: "HuggingFaceHub".to_string(),
        description: "Hugging Face Hub API for model discovery, dataset management, spaces, and repository operations".to_string(),
        base_url: "https://huggingface.co/api".to_string(),
        docs_url: Some("https://huggingface.co/docs/hub/api".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        env_auth: vec![
            "HF_TOKEN".to_string(),
            "HUGGING_FACE_API_KEY".to_string(),
            "HF_API_KEY".to_string(),
        ],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            // =================================================================
            // Models Endpoints
            // =================================================================
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: "Lists models with optional filtering. Query params: search, author, filter, sort, direction, limit, full, config".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<ModelInfo>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{repo_id}".to_string(),
                description: "Gets detailed information about a specific model".to_string(),
                request: None,
                response: ApiResponse::json_type("ModelInfo"),
                headers: vec![],
            },
            Endpoint {
                id: "ListModelFiles".to_string(),
                method: RestMethod::Get,
                path: "/models/{repo_id}/tree/{revision}".to_string(),
                description: "Lists files in a model repository at a specific revision".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<RepoFile>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetModelFile".to_string(),
                method: RestMethod::Get,
                path: "/models/{repo_id}/blob/{revision}/{path}".to_string(),
                description: "Gets file metadata for a specific file in a model repository".to_string(),
                request: None,
                response: ApiResponse::json_type("FileMetadata"),
                headers: vec![],
            },
            Endpoint {
                id: "ListModelCommits".to_string(),
                method: RestMethod::Get,
                path: "/models/{repo_id}/commits/{revision}".to_string(),
                description: "Lists commits for a model repository".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<Commit>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetModelReadme".to_string(),
                method: RestMethod::Get,
                path: "/models/{repo_id}/resolve/{revision}/README.md".to_string(),
                description: "Gets the README file content for a model".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
            },
            Endpoint {
                id: "ListModelDiscussions".to_string(),
                method: RestMethod::Get,
                path: "/models/{repo_id}/discussions".to_string(),
                description: "Lists discussions for a model repository".to_string(),
                request: None,
                response: ApiResponse::json_type("DiscussionList"),
                headers: vec![],
            },
            Endpoint {
                id: "GetModelCard".to_string(),
                method: RestMethod::Get,
                path: "/models/{repo_id}/resolve/{revision}/model_card.md".to_string(),
                description: "Gets the model card file content".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
            },

            // =================================================================
            // Datasets Endpoints
            // =================================================================
            Endpoint {
                id: "ListDatasets".to_string(),
                method: RestMethod::Get,
                path: "/datasets".to_string(),
                description: "Lists datasets with optional filtering. Query params: search, author, filter, sort, direction, limit, full".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<DatasetInfo>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetDataset".to_string(),
                method: RestMethod::Get,
                path: "/datasets/{repo_id}".to_string(),
                description: "Gets detailed information about a specific dataset".to_string(),
                request: None,
                response: ApiResponse::json_type("DatasetInfo"),
                headers: vec![],
            },
            Endpoint {
                id: "ListDatasetFiles".to_string(),
                method: RestMethod::Get,
                path: "/datasets/{repo_id}/tree/{revision}".to_string(),
                description: "Lists files in a dataset repository at a specific revision".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<RepoFile>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetDatasetFile".to_string(),
                method: RestMethod::Get,
                path: "/datasets/{repo_id}/blob/{revision}/{path}".to_string(),
                description: "Gets file metadata for a specific file in a dataset repository".to_string(),
                request: None,
                response: ApiResponse::json_type("FileMetadata"),
                headers: vec![],
            },
            Endpoint {
                id: "ListDatasetCommits".to_string(),
                method: RestMethod::Get,
                path: "/datasets/{repo_id}/commits/{revision}".to_string(),
                description: "Lists commits for a dataset repository".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<Commit>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetDatasetReadme".to_string(),
                method: RestMethod::Get,
                path: "/datasets/{repo_id}/resolve/{revision}/README.md".to_string(),
                description: "Gets the README file content for a dataset".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
            },

            // =================================================================
            // Spaces Endpoints
            // =================================================================
            Endpoint {
                id: "ListSpaces".to_string(),
                method: RestMethod::Get,
                path: "/spaces".to_string(),
                description: "Lists spaces with optional filtering. Query params: search, author, filter, sort, direction, limit".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<SpaceInfo>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetSpace".to_string(),
                method: RestMethod::Get,
                path: "/spaces/{repo_id}".to_string(),
                description: "Gets detailed information about a specific space".to_string(),
                request: None,
                response: ApiResponse::json_type("SpaceInfo"),
                headers: vec![],
            },
            Endpoint {
                id: "ListSpaceFiles".to_string(),
                method: RestMethod::Get,
                path: "/spaces/{repo_id}/tree/{revision}".to_string(),
                description: "Lists files in a space repository at a specific revision".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<RepoFile>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetSpaceFile".to_string(),
                method: RestMethod::Get,
                path: "/spaces/{repo_id}/blob/{revision}/{path}".to_string(),
                description: "Gets file metadata for a specific file in a space repository".to_string(),
                request: None,
                response: ApiResponse::json_type("FileMetadata"),
                headers: vec![],
            },

            // =================================================================
            // Repository Management Endpoints
            // =================================================================
            Endpoint {
                id: "CreateRepo".to_string(),
                method: RestMethod::Post,
                path: "/repos/create".to_string(),
                description: "Creates a new repository (model, dataset, or space)".to_string(),
                request: Some(ApiRequest::json_type("CreateRepoBody")),
                response: ApiResponse::json_type("RepoUrl"),
                headers: vec![],
            },
            Endpoint {
                id: "DeleteRepo".to_string(),
                method: RestMethod::Delete,
                path: "/repos/delete".to_string(),
                description: "Deletes a repository".to_string(),
                request: Some(ApiRequest::json_type("DeleteRepoBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "UpdateRepoSettings".to_string(),
                method: RestMethod::Put,
                path: "/repos/{repo_type}/{repo_id}/settings".to_string(),
                description: "Updates repository settings (visibility, gated access, etc.)".to_string(),
                request: Some(ApiRequest::json_type("UpdateRepoSettingsBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "MoveRepo".to_string(),
                method: RestMethod::Post,
                path: "/repos/move".to_string(),
                description: "Moves/renames a repository".to_string(),
                request: Some(ApiRequest::json_type("MoveRepoBody")),
                response: ApiResponse::json_type("StatusResponse"),
                headers: vec![],
            },

            // =================================================================
            // User Endpoints
            // =================================================================
            Endpoint {
                id: "WhoAmI".to_string(),
                method: RestMethod::Get,
                path: "/whoami-v2".to_string(),
                description: "Gets information about the authenticated user".to_string(),
                request: None,
                response: ApiResponse::json_type("UserInfo"),
                headers: vec![],
            },
            Endpoint {
                id: "GetUser".to_string(),
                method: RestMethod::Get,
                path: "/users/{username}".to_string(),
                description: "Gets public information about a user".to_string(),
                request: None,
                response: ApiResponse::json_type("UserInfo"),
                headers: vec![],
            },
            Endpoint {
                id: "ListUserRepos".to_string(),
                method: RestMethod::Get,
                path: "/users/{username}/repos".to_string(),
                description: "Lists repositories owned by a user".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<RepoInfo>"),
                headers: vec![],
            },
            Endpoint {
                id: "GetUserCollections".to_string(),
                method: RestMethod::Get,
                path: "/users/{username}/collections".to_string(),
                description: "Gets collections created by a user".to_string(),
                request: None,
                response: ApiResponse::json_type("Vec<Collection>"),
                headers: vec![],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::RestMethod;

    // =========================================================================
    // REST API Tests
    // =========================================================================

    #[test]
    fn api_has_correct_metadata() {
        let api = define_huggingface_hub_api();

        assert_eq!(api.name, "HuggingFaceHub");
        assert_eq!(api.base_url, "https://huggingface.co/api");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn api_uses_bearer_token_auth() {
        let api = define_huggingface_hub_api();

        match &api.auth {
            AuthStrategy::BearerToken { header } => {
                assert!(header.is_none(), "Should use default Authorization header");
            }
            _ => panic!("Expected BearerToken auth strategy"),
        }
    }

    #[test]
    fn api_has_correct_env_auth_order() {
        let api = define_huggingface_hub_api();

        assert_eq!(api.env_auth.len(), 3);
        assert_eq!(api.env_auth[0], "HF_TOKEN");
        assert_eq!(api.env_auth[1], "HUGGING_FACE_API_KEY");
        assert_eq!(api.env_auth[2], "HF_API_KEY");
    }

    #[test]
    fn api_has_minimum_endpoints() {
        let api = define_huggingface_hub_api();
        assert!(
            api.endpoints.len() >= 26,
            "Expected at least 26 endpoints, got {}",
            api.endpoints.len()
        );
    }

    #[test]
    fn model_endpoints_exist() {
        let api = define_huggingface_hub_api();

        let model_endpoints = [
            "ListModels",
            "GetModel",
            "ListModelFiles",
            "GetModelFile",
            "ListModelCommits",
            "GetModelReadme",
            "ListModelDiscussions",
            "GetModelCard",
        ];

        for id in &model_endpoints {
            assert!(
                api.endpoints.iter().any(|e| &e.id == id),
                "Missing model endpoint: {}",
                id
            );
        }
    }

    #[test]
    fn dataset_endpoints_exist() {
        let api = define_huggingface_hub_api();

        let dataset_endpoints = [
            "ListDatasets",
            "GetDataset",
            "ListDatasetFiles",
            "GetDatasetFile",
            "ListDatasetCommits",
            "GetDatasetReadme",
        ];

        for id in &dataset_endpoints {
            assert!(
                api.endpoints.iter().any(|e| &e.id == id),
                "Missing dataset endpoint: {}",
                id
            );
        }
    }

    #[test]
    fn space_endpoints_exist() {
        let api = define_huggingface_hub_api();

        let space_endpoints = ["ListSpaces", "GetSpace", "ListSpaceFiles", "GetSpaceFile"];

        for id in &space_endpoints {
            assert!(
                api.endpoints.iter().any(|e| &e.id == id),
                "Missing space endpoint: {}",
                id
            );
        }
    }

    #[test]
    fn repo_endpoints_exist() {
        let api = define_huggingface_hub_api();

        let repo_endpoints = ["CreateRepo", "DeleteRepo", "UpdateRepoSettings", "MoveRepo"];

        for id in &repo_endpoints {
            assert!(
                api.endpoints.iter().any(|e| &e.id == id),
                "Missing repo endpoint: {}",
                id
            );
        }
    }

    #[test]
    fn user_endpoints_exist() {
        let api = define_huggingface_hub_api();

        let user_endpoints = ["WhoAmI", "GetUser", "ListUserRepos", "GetUserCollections"];

        for id in &user_endpoints {
            assert!(
                api.endpoints.iter().any(|e| &e.id == id),
                "Missing user endpoint: {}",
                id
            );
        }
    }

    #[test]
    fn repo_endpoints_use_correct_methods() {
        let api = define_huggingface_hub_api();

        let create_repo = api
            .endpoints
            .iter()
            .find(|e| e.id == "CreateRepo")
            .expect("CreateRepo endpoint missing");
        assert_eq!(create_repo.method, RestMethod::Post);

        let delete_repo = api
            .endpoints
            .iter()
            .find(|e| e.id == "DeleteRepo")
            .expect("DeleteRepo endpoint missing");
        assert_eq!(delete_repo.method, RestMethod::Delete);

        let update_settings = api
            .endpoints
            .iter()
            .find(|e| e.id == "UpdateRepoSettings")
            .expect("UpdateRepoSettings endpoint missing");
        assert_eq!(update_settings.method, RestMethod::Put);

        let move_repo = api
            .endpoints
            .iter()
            .find(|e| e.id == "MoveRepo")
            .expect("MoveRepo endpoint missing");
        assert_eq!(move_repo.method, RestMethod::Post);
    }

    #[test]
    fn text_response_endpoints() {
        let api = define_huggingface_hub_api();

        let text_endpoints = ["GetModelReadme", "GetModelCard", "GetDatasetReadme"];

        for id in &text_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| &e.id == id)
                .unwrap_or_else(|| panic!("Missing endpoint: {}", id));
            assert!(
                matches!(endpoint.response, ApiResponse::Text),
                "Endpoint {} should have Text response",
                id
            );
        }
    }

    #[test]
    fn path_parameters_use_correct_syntax() {
        let api = define_huggingface_hub_api();

        // Check that path parameters use {param} syntax
        let get_model = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetModel")
            .expect("GetModel endpoint missing");
        assert!(get_model.path.contains("{repo_id}"));

        let list_model_files = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListModelFiles")
            .expect("ListModelFiles endpoint missing");
        assert!(list_model_files.path.contains("{repo_id}"));
        assert!(list_model_files.path.contains("{revision}"));

        let get_user = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetUser")
            .expect("GetUser endpoint missing");
        assert!(get_user.path.contains("{username}"));
    }

    #[test]
    fn write_endpoints_have_request_bodies() {
        let api = define_huggingface_hub_api();

        let write_endpoints = ["CreateRepo", "DeleteRepo", "UpdateRepoSettings", "MoveRepo"];

        for id in &write_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| &e.id == id)
                .unwrap_or_else(|| panic!("Missing endpoint: {}", id));
            assert!(
                endpoint.request.is_some(),
                "Endpoint {} should have a request body",
                id
            );
        }
    }

    #[test]
    fn get_endpoints_have_no_request_bodies() {
        let api = define_huggingface_hub_api();

        for endpoint in &api.endpoints {
            if endpoint.method == RestMethod::Get {
                assert!(
                    endpoint.request.is_none(),
                    "GET endpoint {} should not have a request body",
                    endpoint.id
                );
            }
        }
    }
}
