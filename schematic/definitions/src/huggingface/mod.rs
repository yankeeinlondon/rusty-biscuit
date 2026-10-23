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

use crate as schematic_definitions;
#[cfg(test)]
use schematic_define::ResponseShape;
use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError, rust_path};
use schematic_define::{
    AuthStrategy, CrossOrigin, Endpoint, EnvList, EnvMapping, RestApi,
    RestMethod,
    pagination::{Collection as ListCollection},
};

/// Assemble the APIs and ordered schemas in one destination context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::huggingface", |ctx| {
        ctx.register::<CreateRepoBody>()?;
        ctx.register::<DeleteRepoBody>()?;
        ctx.register::<MoveRepoBody>()?;
        ctx.register::<UpdateRepoSettingsBody>()?;
        ctx.register::<Vec<ModelInfo>>()?;
        ctx.register::<ModelInfo>()?;
        ctx.register::<Vec<RepoFile>>()?;
        ctx.register::<FileMetadata>()?;
        ctx.register_as::<CommitInfo>(rust_path!(schematic_definitions::huggingface::Commit).schema_name("Commit"))?;
        ctx.register::<DiscussionList>()?;
        ctx.register::<Vec<DatasetInfo>>()?;
        ctx.register::<DatasetInfo>()?;
        ctx.register::<Vec<SpaceInfo>>()?;
        ctx.register::<SpaceInfo>()?;
        ctx.register::<RepoUrl>()?;
        ctx.register::<StatusResponse>()?;
        ctx.register::<UserInfo>()?;
        ctx.register::<Vec<RepoInfo>>()?;
        ctx.register::<Vec<Collection>>()?;
        Ok(vec![build_huggingface_hub_api(ctx)?])
    })
}

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
    module_group().expect("valid shipped API").remove(0)
}

fn build_huggingface_hub_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    {
        let endpoints = vec![
            // =================================================================
            // Models Endpoints
            // =================================================================
            Endpoint::new("ListModels", RestMethod::Get, "/models")
            .describes("Lists models with optional filtering. Query params: search, author, filter, sort, direction, limit, full, config")
            .returns_json::<Vec<ModelInfo>>(ctx)
            .build()?,
            Endpoint::new("GetModel", RestMethod::Get, "/models/{+repo_id}")
            .describes("Gets detailed information about a specific model")
            .returns_json::<ModelInfo>(ctx)
            .build()?,
            Endpoint::new("ListModelFiles", RestMethod::Get, "/models/{+repo_id}/tree/{revision}")
            .describes("Lists files in a model repository at a specific revision")
            .returns_json::<Vec<RepoFile>>(ctx)
            .build()?,
            Endpoint::new("GetModelFile", RestMethod::Get, "/models/{+repo_id}/blob/{revision}/{+path}")
            .describes("Gets file metadata for a specific file in a model repository")
            .returns_json::<FileMetadata>(ctx)
            .build()?,
            Endpoint::new("ListModelCommits", RestMethod::Get, "/models/{+repo_id}/commits/{revision}")
            .describes("Lists commits for a model repository")
            .returns_json::<Vec<Commit>>(ctx)
            .build()?,
            Endpoint::new("GetModelReadme", RestMethod::Get, "/models/{+repo_id}/resolve/{revision}/README.md")
            .describes("Gets the README file content for a model")
            .returns_text()
            .build()?,
            Endpoint::new("ListModelDiscussions", RestMethod::Get, "/models/{+repo_id}/discussions")
            .describes("Lists discussions for a model repository")
            .returns_json::<DiscussionList>(ctx)
            .pagination(ListCollection::at("discussions").item_type("Discussion").unpaginated())
            .build()?,
            Endpoint::new("GetModelCard", RestMethod::Get, "/models/{+repo_id}/resolve/{revision}/model_card.md")
            .describes("Gets the model card file content")
            .returns_text()
            .build()?,

            // =================================================================
            // Datasets Endpoints
            // =================================================================
            Endpoint::new("ListDatasets", RestMethod::Get, "/datasets")
            .describes("Lists datasets with optional filtering. Query params: search, author, filter, sort, direction, limit, full")
            .returns_json::<Vec<DatasetInfo>>(ctx)
            .build()?,
            Endpoint::new("GetDataset", RestMethod::Get, "/datasets/{+repo_id}")
            .describes("Gets detailed information about a specific dataset")
            .returns_json::<DatasetInfo>(ctx)
            .build()?,
            Endpoint::new("ListDatasetFiles", RestMethod::Get, "/datasets/{+repo_id}/tree/{revision}")
            .describes("Lists files in a dataset repository at a specific revision")
            .returns_json::<Vec<RepoFile>>(ctx)
            .build()?,
            Endpoint::new("GetDatasetFile", RestMethod::Get, "/datasets/{+repo_id}/blob/{revision}/{+path}")
            .describes("Gets file metadata for a specific file in a dataset repository")
            .returns_json::<FileMetadata>(ctx)
            .build()?,
            Endpoint::new("ListDatasetCommits", RestMethod::Get, "/datasets/{+repo_id}/commits/{revision}")
            .describes("Lists commits for a dataset repository")
            .returns_json::<Vec<Commit>>(ctx)
            .build()?,
            Endpoint::new("GetDatasetReadme", RestMethod::Get, "/datasets/{+repo_id}/resolve/{revision}/README.md")
            .describes("Gets the README file content for a dataset")
            .returns_text()
            .build()?,

            // =================================================================
            // Spaces Endpoints
            // =================================================================
            Endpoint::new("ListSpaces", RestMethod::Get, "/spaces")
            .describes("Lists spaces with optional filtering. Query params: search, author, filter, sort, direction, limit")
            .returns_json::<Vec<SpaceInfo>>(ctx)
            .build()?,
            Endpoint::new("GetSpace", RestMethod::Get, "/spaces/{+repo_id}")
            .describes("Gets detailed information about a specific space")
            .returns_json::<SpaceInfo>(ctx)
            .build()?,
            Endpoint::new("ListSpaceFiles", RestMethod::Get, "/spaces/{+repo_id}/tree/{revision}")
            .describes("Lists files in a space repository at a specific revision")
            .returns_json::<Vec<RepoFile>>(ctx)
            .build()?,
            Endpoint::new("GetSpaceFile", RestMethod::Get, "/spaces/{+repo_id}/blob/{revision}/{+path}")
            .describes("Gets file metadata for a specific file in a space repository")
            .returns_json::<FileMetadata>(ctx)
            .build()?,

            // =================================================================
            // Repository Management Endpoints
            // =================================================================
            Endpoint::new("CreateRepo", RestMethod::Post, "/repos/create")
            .describes("Creates a new repository (model, dataset, or space)")
            .request_json::<CreateRepoBody>(ctx)
            .returns_json::<RepoUrl>(ctx)
            .build()?,
            Endpoint::new("DeleteRepo", RestMethod::Delete, "/repos/delete")
            .describes("Deletes a repository")
            .request_json::<DeleteRepoBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
            Endpoint::new("UpdateRepoSettings", RestMethod::Put, "/repos/{repo_type}/{+repo_id}/settings")
            .describes("Updates repository settings (visibility, gated access, etc.)")
            .request_json::<UpdateRepoSettingsBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
            Endpoint::new("MoveRepo", RestMethod::Post, "/repos/move")
            .describes("Moves/renames a repository")
            .request_json::<MoveRepoBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,

            // =================================================================
            // User Endpoints
            // =================================================================
            Endpoint::new("WhoAmI", RestMethod::Get, "/whoami-v2")
            .describes("Gets information about the authenticated user")
            .returns_json::<UserInfo>(ctx)
            .build()?,
            Endpoint::new("GetUser", RestMethod::Get, "/users/{username}")
            .describes("Gets public information about a user")
            .returns_json::<UserInfo>(ctx)
            .build()?,
            Endpoint::new("ListUserRepos", RestMethod::Get, "/users/{username}/repos")
            .describes("Lists repositories owned by a user")
            .returns_json::<Vec<RepoInfo>>(ctx)
            .build()?,
            Endpoint::new("GetUserCollections", RestMethod::Get, "/users/{username}/collections")
            .describes("Gets collections created by a user")
            .returns_json::<Vec<Collection>>(ctx)
            .build()?,
        ];
        RestApi::builder(ctx, "HuggingFaceHub", "https://huggingface.co/api")
            .describes("Hugging Face Hub API for model discovery, dataset management, spaces, and repository operations")
            .authentication(AuthStrategy::BearerToken { header: None }, vec![
            "HF_TOKEN".to_string(),
            "HUGGING_FACE_API_KEY".to_string(),
            "HF_API_KEY".to_string(),
        ])
            .env_mapping(EnvMapping {
            bearer_token: Some(EnvList::new(vec![
                "HF_TOKEN".to_string(),
                "HUGGING_FACE_API_KEY".to_string(),
                "HF_API_KEY".to_string(),
            ])),
            ..Default::default()
        })
            .docs_url("https://huggingface.co/docs/hub/api".to_string())
            .module_path("huggingface".to_string())
            .cross_origin(CrossOrigin::same_origin())
            .endpoints(endpoints)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{RestMethod, TypeRef};

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
                matches!(endpoint.response[0].shape, ResponseShape::Text),
                "Endpoint {} should have Text response",
                id
            );
        }
    }

    #[test]
    fn path_parameters_use_correct_syntax() {
        let api = define_huggingface_hub_api();

        // Check that path parameters use {param} syntax. `repo_id` is a
        // slash-bearing identifier, so it carries the `{+repo_id}`
        // reserved-expansion marker to opt out of percent-encoding.
        let get_model = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetModel")
            .expect("GetModel endpoint missing");
        assert!(get_model.path.contains("{+repo_id}"));

        let list_model_files = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListModelFiles")
            .expect("ListModelFiles endpoint missing");
        assert!(list_model_files.path.contains("{+repo_id}"));
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
    #[test]
    fn typed_module_group_preserves_endpoint_order_and_inline_components() {
        let group = module_group().unwrap();
        let ids = |api: &RestApi| api.endpoints.iter().map(|ep| ep.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&group[0]), ["ListModels", "GetModel", "ListModelFiles", "GetModelFile", "ListModelCommits", "GetModelReadme", "ListModelDiscussions", "GetModelCard", "ListDatasets", "GetDataset", "ListDatasetFiles", "GetDatasetFile", "ListDatasetCommits", "GetDatasetReadme", "ListSpaces", "GetSpace", "ListSpaceFiles", "GetSpaceFile", "CreateRepo", "DeleteRepo", "UpdateRepoSettings", "MoveRepo", "WhoAmI", "GetUser", "ListUserRepos", "GetUserCollections"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
        for name in ["Vec<ModelInfo>", "Vec<RepoFile>", "Vec<Commit>", "Vec<DatasetInfo>", "Vec<SpaceInfo>", "Vec<RepoInfo>", "Vec<Collection>"] {
            assert!(group[0].component(name).is_none(), "container component {name} must be inline");
        }
        assert!(group[0].component("Commit").is_some());
        let commits = group[0].endpoints.iter().find(|ep| ep.id == "ListModelCommits").unwrap();
        let ResponseShape::Json(schema) = &commits.response[0].shape else { panic!("JSON commits"); };
        assert!(*schema == TypeRef::Array(Box::new(TypeRef::Named("Commit".into()))));
    }

}
