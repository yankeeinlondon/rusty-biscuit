//! Ollama API definitions.
//!
//! This module provides definitions for both the native Ollama API (`/api/*` endpoints)
//! and the OpenAI-compatible API (`/v1/*` endpoints). Ollama runs locally and does not
//! require authentication.
//!
//! ## Native API
//!
//! The native API provides full control over model lifecycle and runtime parameters:
//!
//! - Generation endpoints: `/api/generate`, `/api/chat`, `/api/embeddings`
//! - Model management: `/api/tags`, `/api/show`, `/api/pull`, `/api/push`, `/api/copy`, `/api/delete`, `/api/create`, `/api/ps`
//!
//! Streaming endpoints use NDJSON (newline-delimited JSON) format.
//!
//! ## OpenAI-Compatible API
//!
//! The OpenAI-compatible API allows existing OpenAI clients to work with Ollama:
//!
//! - `/v1/chat/completions` - Chat completions (streaming via SSE)
//! - `/v1/completions` - Text completions (streaming via SSE)
//! - `/v1/embeddings` - Generate embeddings
//! - `/v1/models` - List available models

mod types;

pub use types::*;

use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

/// Creates the native Ollama API definition.
///
/// The native API runs at `http://localhost:11434` and provides full control over
/// model generation, management, and lifecycle.
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | Generate | POST | /api/generate | Generate text completion |
/// | Chat | POST | /api/chat | Generate chat completion |
/// | Embeddings | POST | /api/embeddings | Generate embeddings |
/// | ListModels | GET | /api/tags | List available models |
/// | ShowModel | POST | /api/show | Show model information |
/// | PullModel | POST | /api/pull | Pull a model from registry |
/// | PushModel | POST | /api/push | Push a model to registry |
/// | CopyModel | POST | /api/copy | Copy a model |
/// | DeleteModel | DELETE | /api/delete | Delete a model |
/// | CreateModel | POST | /api/create | Create a model from Modelfile |
/// | ListRunningModels | GET | /api/ps | List running models |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::ollama::define_ollama_native_api;
///
/// let api = define_ollama_native_api();
/// assert_eq!(api.name, "OllamaNative");
/// assert_eq!(api.endpoints.len(), 11);
/// ```
pub fn define_ollama_native_api() -> RestApi {
    RestApi {
        name: "OllamaNative".to_string(),
        description: "Ollama native REST API for local LLM inference and model management"
            .to_string(),
        base_url: "http://localhost:11434".to_string(),
        docs_url: Some("https://github.com/ollama/ollama/blob/main/docs/api.md".to_string()),
        auth: AuthStrategy::None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            // Generation endpoints
            Endpoint {
                id: "Generate".to_string(),
                method: RestMethod::Post,
                path: "/api/generate".to_string(),
                description: "Generate text completion from a prompt (streaming NDJSON by default)"
                    .to_string(),
                request: Some(ApiRequest::json_type("GenerateBody")),
                response: ApiResponse::Binary, // Streaming NDJSON
                headers: vec![],
            },
            Endpoint {
                id: "Chat".to_string(),
                method: RestMethod::Post,
                path: "/api/chat".to_string(),
                description: "Generate chat completion from messages (streaming NDJSON by default)"
                    .to_string(),
                request: Some(ApiRequest::json_type("ChatBody")),
                response: ApiResponse::Binary, // Streaming NDJSON
                headers: vec![],
            },
            Endpoint {
                id: "Embeddings".to_string(),
                method: RestMethod::Post,
                path: "/api/embeddings".to_string(),
                description: "Generate embeddings for text".to_string(),
                request: Some(ApiRequest::json_type("EmbeddingsBody")),
                response: ApiResponse::json_type("EmbeddingsResponse"),
                headers: vec![],
            },
            // Model management endpoints
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/api/tags".to_string(),
                description: "List locally available models".to_string(),
                request: None,
                response: ApiResponse::json_type("ListModelsResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "ShowModel".to_string(),
                method: RestMethod::Post,
                path: "/api/show".to_string(),
                description: "Show detailed information about a model".to_string(),
                request: Some(ApiRequest::json_type("ShowModelBody")),
                response: ApiResponse::json_type("ShowModelResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "PullModel".to_string(),
                method: RestMethod::Post,
                path: "/api/pull".to_string(),
                description:
                    "Pull a model from the Ollama registry (streaming progress by default)"
                        .to_string(),
                request: Some(ApiRequest::json_type("PullModelBody")),
                response: ApiResponse::Binary, // Streaming progress
                headers: vec![],
            },
            Endpoint {
                id: "PushModel".to_string(),
                method: RestMethod::Post,
                path: "/api/push".to_string(),
                description: "Push a model to the Ollama registry (streaming progress by default)"
                    .to_string(),
                request: Some(ApiRequest::json_type("PushModelBody")),
                response: ApiResponse::Binary, // Streaming progress
                headers: vec![],
            },
            Endpoint {
                id: "CopyModel".to_string(),
                method: RestMethod::Post,
                path: "/api/copy".to_string(),
                description: "Copy a model to a new name".to_string(),
                request: Some(ApiRequest::json_type("CopyModelBody")),
                response: ApiResponse::Empty,
                headers: vec![],
            },
            Endpoint {
                id: "DeleteModel".to_string(),
                method: RestMethod::Delete,
                path: "/api/delete".to_string(),
                description: "Delete a model".to_string(),
                request: Some(ApiRequest::json_type("DeleteModelBody")),
                response: ApiResponse::Empty,
                headers: vec![],
            },
            Endpoint {
                id: "CreateModel".to_string(),
                method: RestMethod::Post,
                path: "/api/create".to_string(),
                description: "Create a model from a Modelfile (streaming progress by default)"
                    .to_string(),
                request: Some(ApiRequest::json_type("CreateModelBody")),
                response: ApiResponse::Binary, // Streaming progress
                headers: vec![],
            },
            Endpoint {
                id: "ListRunningModels".to_string(),
                method: RestMethod::Get,
                path: "/api/ps".to_string(),
                description: "List models currently loaded in memory".to_string(),
                request: None,
                response: ApiResponse::json_type("ListRunningModelsResponse"),
                headers: vec![],
            },
        ],
        module_path: None,
        request_suffix: None,
    }
}

/// Creates the OpenAI-compatible Ollama API definition.
///
/// The OpenAI-compatible API allows existing OpenAI clients to work with Ollama
/// by using the same endpoint structure and request/response formats.
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | ChatCompletions | POST | /v1/chat/completions | Chat completions (SSE streaming) |
/// | Completions | POST | /v1/completions | Text completions (SSE streaming) |
/// | Embeddings | POST | /v1/embeddings | Generate embeddings |
/// | ListModels | GET | /v1/models | List available models |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::ollama::define_ollama_openai_api;
///
/// let api = define_ollama_openai_api();
/// assert_eq!(api.name, "OllamaOpenAI");
/// assert_eq!(api.endpoints.len(), 4);
/// ```
pub fn define_ollama_openai_api() -> RestApi {
    RestApi {
        name: "OllamaOpenAI".to_string(),
        description: "Ollama OpenAI-compatible REST API for drop-in replacement of OpenAI clients"
            .to_string(),
        base_url: "http://localhost:11434".to_string(),
        docs_url: Some("https://github.com/ollama/ollama/blob/main/docs/openai.md".to_string()),
        auth: AuthStrategy::None, // Ollama ignores API keys but accepts them
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            Endpoint {
                id: "ChatCompletions".to_string(),
                method: RestMethod::Post,
                path: "/v1/chat/completions".to_string(),
                description: "Create chat completion (SSE streaming when stream=true)".to_string(),
                request: Some(ApiRequest::json_type("OpenAIChatCompletionRequest")),
                response: ApiResponse::Binary, // SSE streaming
                headers: vec![],
            },
            Endpoint {
                id: "Completions".to_string(),
                method: RestMethod::Post,
                path: "/v1/completions".to_string(),
                description: "Create text completion (SSE streaming when stream=true)".to_string(),
                request: Some(ApiRequest::json_type("OpenAICompletionRequest")),
                response: ApiResponse::Binary, // SSE streaming
                headers: vec![],
            },
            Endpoint {
                id: "Embeddings".to_string(),
                method: RestMethod::Post,
                path: "/v1/embeddings".to_string(),
                description: "Generate embeddings for text".to_string(),
                request: Some(ApiRequest::json_type("OpenAIEmbeddingRequest")),
                response: ApiResponse::json_type("OpenAIEmbeddingResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/v1/models".to_string(),
                description: "List available models in OpenAI format".to_string(),
                request: None,
                response: ApiResponse::json_type("OpenAIListModelsResponse"),
                headers: vec![],
            },
        ],
        module_path: None,
        request_suffix: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Native API Tests
    // =========================================================================

    #[test]
    fn native_api_has_correct_metadata() {
        let api = define_ollama_native_api();

        assert_eq!(api.name, "OllamaNative");
        assert_eq!(api.base_url, "http://localhost:11434");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn native_api_uses_no_auth() {
        let api = define_ollama_native_api();

        assert!(matches!(api.auth, AuthStrategy::None));
        assert!(api.env_auth.is_empty());
    }

    #[test]
    fn native_api_has_eleven_endpoints() {
        let api = define_ollama_native_api();
        assert_eq!(api.endpoints.len(), 11);
    }

    #[test]
    fn native_generate_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Generate").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/generate");
        assert!(endpoint.request.is_some());
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn native_chat_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Chat").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/chat");
        assert!(endpoint.request.is_some());
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn native_embeddings_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Embeddings").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/embeddings");
        assert!(endpoint.request.is_some());
        // Non-streaming JSON response
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn native_list_models_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListModels").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/api/tags");
        assert!(endpoint.request.is_none());
    }

    #[test]
    fn native_show_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ShowModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/show");
        assert!(endpoint.request.is_some());
    }

    #[test]
    fn native_pull_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "PullModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/pull");
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn native_push_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "PushModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/push");
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn native_copy_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "CopyModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/copy");
        assert!(matches!(endpoint.response, ApiResponse::Empty));
    }

    #[test]
    fn native_delete_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "DeleteModel")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Delete);
        assert_eq!(endpoint.path, "/api/delete");
        // DELETE with JSON body (not path parameter)
        assert!(endpoint.request.is_some());
    }

    #[test]
    fn native_create_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "CreateModel")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/create");
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn native_list_running_models_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListRunningModels")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/api/ps");
        assert!(endpoint.request.is_none());
    }

    // =========================================================================
    // OpenAI-Compatible API Tests
    // =========================================================================

    #[test]
    fn openai_api_has_correct_metadata() {
        let api = define_ollama_openai_api();

        assert_eq!(api.name, "OllamaOpenAI");
        assert_eq!(api.base_url, "http://localhost:11434");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn openai_api_uses_no_auth() {
        let api = define_ollama_openai_api();

        // Ollama accepts but ignores API keys
        assert!(matches!(api.auth, AuthStrategy::None));
        assert!(api.env_auth.is_empty());
    }

    #[test]
    fn openai_api_has_four_endpoints() {
        let api = define_ollama_openai_api();
        assert_eq!(api.endpoints.len(), 4);
    }

    #[test]
    fn openai_chat_completions_endpoint() {
        let api = define_ollama_openai_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ChatCompletions")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/v1/chat/completions");
        assert!(endpoint.request.is_some());
        // SSE streaming
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn openai_completions_endpoint() {
        let api = define_ollama_openai_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "Completions")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/v1/completions");
        assert!(endpoint.request.is_some());
        // SSE streaming
        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn openai_embeddings_endpoint() {
        let api = define_ollama_openai_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Embeddings").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/v1/embeddings");
        assert!(endpoint.request.is_some());
        // Non-streaming JSON response
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn openai_list_models_endpoint() {
        let api = define_ollama_openai_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListModels").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/v1/models");
        assert!(endpoint.request.is_none());
    }

    // =========================================================================
    // Cross-API Tests
    // =========================================================================

    #[test]
    fn both_apis_share_base_url() {
        let native = define_ollama_native_api();
        let openai = define_ollama_openai_api();

        assert_eq!(native.base_url, openai.base_url);
    }

    #[test]
    fn apis_have_distinct_names() {
        let native = define_ollama_native_api();
        let openai = define_ollama_openai_api();

        assert_ne!(native.name, openai.name);
    }
}
