//! LM Studio API definitions.
//!
//! This module provides definitions for the LM Studio v1 native API (`/api/v1/*` endpoints).
//! LM Studio runs locally and supports optional bearer token authentication.
//!
//! ## Native API
//!
//! The v1 REST API provides enhanced features:
//!
//! - Chat inference with SSE streaming
//! - Model management: load, unload, list
//! - Model downloads with progress tracking
//! - Optional bearer token authentication
//!
//! ## Examples
//!
//! ```rust
//! use schematic_definitions::lmstudio::define_lmstudio_api;
//!
//! let api = define_lmstudio_api();
//! assert_eq!(api.name, "LmStudio");
//! assert_eq!(api.endpoints.len(), 6);
//! ```

mod types;

pub use types::*;

use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError};
#[cfg(test)]
use schematic_define::{ApiResponse, RestMethod};
use schematic_define::{
    AuthStrategy, Endpoint, EnvMapping, RestApi,
};

/// Creates the LM Studio v1 native API definition.
///
/// The native API runs at `http://localhost:1234` by default and provides
/// full control over model inference, loading, and downloading.
///
/// ## Authentication
///
/// LM Studio supports optional bearer token authentication. Set the `LM_API_TOKEN`
/// environment variable to enable authentication.
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | Chat | POST | /api/v1/chat | Chat completion with SSE streaming |
/// | ListModels | GET | /api/v1/models | List available models |
/// | LoadModel | POST | /api/v1/models/load | Load a model into memory |
/// | UnloadModel | POST | /api/v1/models/unload | Unload a model from memory |
/// | DownloadModel | POST | /api/v1/models/download | Download a model from HuggingFace |
/// | GetDownloadStatus | POST | /api/v1/models/download/status | Check download job status |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::lmstudio::define_lmstudio_api;
///
/// let api = define_lmstudio_api();
/// assert_eq!(api.name, "LmStudio");
/// assert_eq!(api.base_url, "http://localhost:1234");
/// assert_eq!(api.endpoints.len(), 6);
/// ```
pub fn define_lmstudio_api() -> RestApi {
    module_group().expect("valid lmstudio definitions").remove(0)
}

fn build_lmstudio_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = vec![

            Endpoint::post("Chat", "/api/v1/chat")
                .describes("Chat completion with SSE streaming support")
                .request_json::<ChatBody>(ctx)
                .returns_binary()
                .build()?,

            Endpoint::get("ListModels", "/api/v1/models")
                .describes("List available models")
                .returns_json::<ListModelsResponse>(ctx)
                .build()?,

            Endpoint::post("LoadModel", "/api/v1/models/load")
                .describes("Load a model into memory")
                .request_json::<LoadModelBody>(ctx)
                .returns_json::<LoadModelResponse>(ctx)
                .build()?,

            Endpoint::post("UnloadModel", "/api/v1/models/unload")
                .describes("Unload a model from memory")
                .request_json::<UnloadModelBody>(ctx)
                .returns_json::<UnloadModelResponse>(ctx)
                .build()?,

            Endpoint::post("DownloadModel", "/api/v1/models/download")
                .describes("Download a model from HuggingFace with SSE progress")
                .request_json::<DownloadModelBody>(ctx)
                .returns_binary()
                .build()?,

            Endpoint::post("GetDownloadStatus", "/api/v1/models/download/status")
                .describes("Check the status of a download job")
                .request_json::<DownloadStatusBody>(ctx)
                .returns_json::<DownloadStatusResponse>(ctx)
                .build()?,
        ];
    RestApi::builder(ctx, "LmStudio", "http://localhost:1234")
        .describes("LM Studio v1 native REST API for local LLM inference and model management")
        .authentication(AuthStrategy::BearerToken { header: None }, vec!["LM_API_TOKEN".to_string()])
        .docs_url("https://lmstudio.ai/docs/developer/rest")
        .endpoints(endpoints)
        .env_mapping(EnvMapping::default())
        .build()
}

/// Assemble sibling APIs and their ordered nominal schemas in one context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::lmstudio", |ctx| {
        ctx.register::<ChatBody>()?;
        ctx.register::<LoadModelBody>()?;
        ctx.register::<UnloadModelBody>()?;
        ctx.register::<DownloadModelBody>()?;
        ctx.register::<DownloadStatusBody>()?;
        ctx.register::<ListModelsResponse>()?;
        ctx.register::<ModelInfo>()?;
        ctx.register::<Quantization>()?;
        ctx.register::<Capabilities>()?;
        ctx.register::<LoadedInstance>()?;
        ctx.register::<Stats>()?;
        ctx.register::<LoadModelResponse>()?;
        ctx.register::<UnloadModelResponse>()?;
        ctx.register::<DownloadStatusResponse>()?;
        ctx.register::<OutputItem>()?;
        Ok(vec![
            build_lmstudio_api(ctx)?,
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_has_correct_metadata() {
        let api = define_lmstudio_api();

        assert_eq!(api.name, "LmStudio");
        assert_eq!(api.base_url, "http://localhost:1234");
        assert!(api.docs_url.is_some());
        assert_eq!(
            api.docs_url.as_ref().unwrap(),
            "https://lmstudio.ai/docs/developer/rest"
        );
    }

    #[test]
    fn api_uses_bearer_token_auth() {
        let api = define_lmstudio_api();

        assert!(matches!(
            api.auth,
            AuthStrategy::BearerToken { header: None }
        ));
        assert_eq!(api.env_auth, vec!["LM_API_TOKEN"]);
    }

    #[test]
    fn api_has_six_endpoints() {
        let api = define_lmstudio_api();
        assert_eq!(api.endpoints.len(), 6);
    }

    #[test]
    fn chat_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Chat").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/v1/chat");
        assert!(endpoint.request.is_some());
        // SSE streaming
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn list_models_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListModels").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/api/v1/models");
        assert!(endpoint.request.is_none());
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn load_model_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "LoadModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/v1/models/load");
        assert!(endpoint.request.is_some());
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn unload_model_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "UnloadModel")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/v1/models/unload");
        assert!(endpoint.request.is_some());
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn download_model_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "DownloadModel")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/v1/models/download");
        assert!(endpoint.request.is_some());
        // SSE streaming for progress
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn get_download_status_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetDownloadStatus")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/v1/models/download/status");
        assert!(endpoint.request.is_some());
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn all_endpoints_have_required_fields() {
        let api = define_lmstudio_api();

        for endpoint in &api.endpoints {
            assert!(!endpoint.id.is_empty(), "Endpoint must have an ID");
            assert!(!endpoint.path.is_empty(), "Endpoint must have a path");
            assert!(
                !endpoint.description.is_empty(),
                "Endpoint must have a description"
            );
        }
    }

    #[test]
    fn module_group_preserves_endpoint_and_registry_inventory() {
        let group = module_group().unwrap();
        assert_eq!(
            group.iter().flat_map(|api| api.endpoints.iter().map(|endpoint| endpoint.id.as_str())).collect::<Vec<_>>(),
            vec!["Chat", "ListModels", "LoadModel", "UnloadModel", "DownloadModel", "GetDownloadStatus"]
        );
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["ChatBody", "LoadModelBody", "UnloadModelBody", "DownloadModelBody", "DownloadStatusBody", "ListModelsResponse", "Capabilities", "LoadedInstance", "ModelInfo", "Quantization", "Stats", "LoadModelResponse", "UnloadModelResponse", "DownloadStatusResponse", "OutputItem"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
    }

}
