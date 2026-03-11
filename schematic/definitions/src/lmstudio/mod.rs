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

use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, EnvMapping, RestApi, RestMethod,
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
    RestApi {
        name: "LmStudio".to_string(),
        description: "LM Studio v1 native REST API for local LLM inference and model management"
            .to_string(),
        base_url: "http://localhost:1234".to_string(),
        docs_url: Some("https://lmstudio.ai/docs/developer/rest".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        auth_policy: None,
        env_auth: vec!["LM_API_TOKEN".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            // Chat endpoint
            Endpoint {
                id: "Chat".to_string(),
                method: RestMethod::Post,
                path: "/api/v1/chat".to_string(),
                description: "Chat completion with SSE streaming support".to_string(),
                request: Some(ApiRequest::json_type("ChatBody")),
                response: ApiResponse::Binary, // SSE streaming
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // Model listing
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/api/v1/models".to_string(),
                description: "List available models".to_string(),
                request: None,
                response: ApiResponse::json_type("ListModelsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // Model loading
            Endpoint {
                id: "LoadModel".to_string(),
                method: RestMethod::Post,
                path: "/api/v1/models/load".to_string(),
                description: "Load a model into memory".to_string(),
                request: Some(ApiRequest::json_type("LoadModelBody")),
                response: ApiResponse::json_type("LoadModelResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // Model unloading
            Endpoint {
                id: "UnloadModel".to_string(),
                method: RestMethod::Post,
                path: "/api/v1/models/unload".to_string(),
                description: "Unload a model from memory".to_string(),
                request: Some(ApiRequest::json_type("UnloadModelBody")),
                response: ApiResponse::json_type("UnloadModelResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // Model download
            Endpoint {
                id: "DownloadModel".to_string(),
                method: RestMethod::Post,
                path: "/api/v1/models/download".to_string(),
                description: "Download a model from HuggingFace with SSE progress".to_string(),
                request: Some(ApiRequest::json_type("DownloadModelBody")),
                response: ApiResponse::Binary, // SSE streaming for progress
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // Download status check
            Endpoint {
                id: "GetDownloadStatus".to_string(),
                method: RestMethod::Post,
                path: "/api/v1/models/download/status".to_string(),
                description: "Check the status of a download job".to_string(),
                request: Some(ApiRequest::json_type("DownloadStatusBody")),
                response: ApiResponse::json_type("DownloadStatusResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
        ],
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping::default()),
    }
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
}
