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

use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError};
#[cfg(test)]
use schematic_define::{AuthStrategy, ResponseShape, RestMethod, StreamEnd, StreamFraming, SuccessShape, TypeRef};
use schematic_define::{
    Endpoint, EnvMapping, RestApi, Success,
};

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
    module_group().expect("valid ollama definitions").remove(0)
}

fn build_ollama_native_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = vec![

            Endpoint::post("Generate", "/api/generate")
                .describes("Generate text completion from a prompt (streaming NDJSON by default)")
                .request_json::<GenerateBody>(ctx)
                .returns(
                    Success::ndjson(ctx.register::<GenerateStreamItem>()?)
                        .note("returned when `stream` is true or omitted")
                        .ends_when_field("done", true)
                        .keep_end(),
                )
                .returns(Success::json(ctx.register::<GenerateCompletion>()?).note("returned when `stream` is false"))
                .build()?,
            Endpoint::post("Chat", "/api/chat")
                .describes("Generate chat completion from messages (streaming NDJSON by default)")
                .request_json::<ChatBody>(ctx)
                .returns(
                    Success::ndjson(ctx.register::<ChatStreamItem>()?)
                        .note("returned when `stream` is true or omitted")
                        .ends_when_field("done", true)
                        .keep_end(),
                )
                .returns(Success::json(ctx.register::<ChatCompletion>()?).note("returned when `stream` is false"))
                .build()?,
            Endpoint::post("Embeddings", "/api/embeddings")
                .describes("Generate embeddings for text")
                .request_json::<EmbeddingsBody>(ctx)
                .returns_json::<EmbeddingsResponse>(ctx)
                .build()?,

            Endpoint::get("ListModels", "/api/tags")
                .describes("List locally available models")
                .returns_json::<ListModelsResponse>(ctx)
                .build()?,
            Endpoint::post("ShowModel", "/api/show")
                .describes("Show detailed information about a model")
                .request_json::<ShowModelBody>(ctx)
                .returns_json::<ShowModelResponse>(ctx)
                .build()?,
            Endpoint::post("PullModel", "/api/pull")
                .describes("Pull a model from the Ollama registry (streaming progress by default)")
                .request_json::<PullModelBody>(ctx)
                .returns(
                    Success::ndjson(ctx.register::<PullModelStreamItem>()?)
                        .note("returned when `stream` is true or omitted")
                        .ends_when_field("status", "success")
                        .keep_end(),
                )
                .returns(Success::json(ctx.register::<StatusResponse>()?).note("returned when `stream` is false"))
                .build()?,
            Endpoint::post("PushModel", "/api/push")
                .describes("Push a model to the Ollama registry (streaming progress by default)")
                .request_json::<PushModelBody>(ctx)
                .returns(
                    Success::ndjson(ctx.register::<PullModelStreamItem>()?)
                        .note("returned when `stream` is true or omitted")
                        .ends_when_field("status", "success")
                        .keep_end(),
                )
                .returns(Success::json(ctx.register::<StatusResponse>()?).note("returned when `stream` is false"))
                .build()?,
            Endpoint::post("CopyModel", "/api/copy")
                .describes("Copy a model to a new name")
                .request_json::<CopyModelBody>(ctx)
                .returns_empty()
                .build()?,
            Endpoint::delete("DeleteModel", "/api/delete")
                .describes("Delete a model")
                .request_json::<DeleteModelBody>(ctx)
                .returns_empty()
                .build()?,
            Endpoint::post("CreateModel", "/api/create")
                .describes("Create a model from a Modelfile (streaming progress by default)")
                .request_json::<CreateModelBody>(ctx)
                .returns(
                    Success::ndjson(ctx.register::<CreateModelStreamItem>()?)
                        .note("returned when `stream` is true or omitted")
                        .ends_when_field("status", "success")
                        .keep_end(),
                )
                .returns(Success::json(ctx.register::<StatusResponse>()?).note("returned when `stream` is false"))
                .build()?,
            Endpoint::get("ListRunningModels", "/api/ps")
                .describes("List models currently loaded in memory")
                .returns_json::<ListRunningModelsResponse>(ctx)
                .build()?,
        ];
    RestApi::builder(ctx, "OllamaNative", "http://localhost:11434")
        .describes("Ollama native REST API for local LLM inference and model management")
        .no_auth()
        .docs_url("https://github.com/ollama/ollama/blob/main/docs/api.md")
        .module_path("ollama")
        .request_suffix("NativeRequest")
        .env_mapping(EnvMapping::default())
        .endpoints(endpoints)
        .build()
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
    module_group().expect("valid ollama definitions").remove(1)
}

fn build_ollama_openai_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = vec![
            Endpoint::post("ChatCompletions", "/v1/chat/completions")
                .describes("Create chat completion (SSE streaming when stream=true)")
                .request_json::<OpenAIChatCompletionRequest>(ctx)
                .returns(
                    Success::json(ctx.register::<OpenAIChatCompletionResponse>()?)
                        .note("returned when `stream` is false or omitted"),
                )
                .returns(
                    Success::sse(ctx.register::<OpenAIChatStreamChunk>()?)
                        .note("returned when `stream` is true")
                        .ends_at_data_line("[DONE]"),
                )
                .build()?,
            Endpoint::post("Completions", "/v1/completions")
                .describes("Create text completion (SSE streaming when stream=true)")
                .request_json::<OpenAICompletionRequest>(ctx)
                .returns(
                    Success::json(ctx.register::<OpenAICompletionResponse>()?)
                        .note("returned when `stream` is false or omitted"),
                )
                .returns(
                    Success::sse(ctx.register::<OpenAICompletionStreamChunk>()?)
                        .note("returned when `stream` is true")
                        .ends_at_data_line("[DONE]"),
                )
                .build()?,
            Endpoint::post("Embeddings", "/v1/embeddings")
                .describes("Generate embeddings for text")
                .request_json::<OpenAIEmbeddingRequest>(ctx)
                .returns_json::<OpenAIEmbeddingResponse>(ctx)
                .build()?,
            Endpoint::get("ListModels", "/v1/models")
                .describes("List available models in OpenAI format")
                .returns_json::<OpenAIListModelsResponse>(ctx)
                .build()?,
        ];
    RestApi::builder(ctx, "OllamaOpenAI", "http://localhost:11434")
        .describes("Ollama OpenAI-compatible REST API for drop-in replacement of OpenAI clients")
        .no_auth()
        .docs_url("https://github.com/ollama/ollama/blob/main/docs/openai.md")
        .module_path("ollama")
        .request_suffix("OaiRequest")
        .env_mapping(EnvMapping::default())
        .endpoints(endpoints)
        .build()
}

/// Assemble sibling APIs and their ordered nominal schemas in one context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::ollama", |ctx| {
        ctx.register::<GenerateBody>()?;
        ctx.register::<ChatBody>()?;
        ctx.register::<EmbeddingsBody>()?;
        ctx.register::<ShowModelBody>()?;
        ctx.register::<PullModelBody>()?;
        ctx.register::<PushModelBody>()?;
        ctx.register::<CopyModelBody>()?;
        ctx.register::<DeleteModelBody>()?;
        ctx.register::<CreateModelBody>()?;
        ctx.register::<Message>()?;
        ctx.register::<EmbeddingsResponse>()?;
        ctx.register::<ListModelsResponse>()?;
        ctx.register::<ModelInfo>()?;
        ctx.register::<ModelDetails>()?;
        ctx.register::<ShowModelResponse>()?;
        ctx.register::<ListRunningModelsResponse>()?;
        ctx.register::<RunningModel>()?;
        ctx.register::<OpenAIChatCompletionRequest>()?;
        ctx.register::<OpenAICompletionRequest>()?;
        ctx.register::<OpenAIEmbeddingRequest>()?;
        ctx.register::<OpenAIMessage>()?;
        ctx.register::<OpenAIEmbeddingResponse>()?;
        ctx.register::<OpenAIEmbeddingData>()?;
        ctx.register::<OpenAIUsage>()?;
        ctx.register::<OpenAIListModelsResponse>()?;
        ctx.register::<OpenAIModel>()?;
        ctx.register::<OpenAIChatCompletionChoice>()?;
        ctx.register::<OpenAIChatCompletionResponse>()?;
        ctx.register::<OpenAIDelta>()?;
        ctx.register::<OpenAIStreamChoice>()?;
        ctx.register::<OpenAIChatStreamChunk>()?;
        ctx.register::<OpenAICompletionChoice>()?;
        ctx.register::<OpenAICompletionResponse>()?;
        ctx.register::<OpenAICompletionStreamChunk>()?;
        ctx.register::<StreamErrorLine>()?;
        ctx.register::<GenerateStreamChunk>()?;
        ctx.register::<GenerateStreamItem>()?;
        ctx.register::<GenerateCompletion>()?;
        ctx.register::<ChatStreamChunk>()?;
        ctx.register::<ChatStreamItem>()?;
        ctx.register::<ChatCompletion>()?;
        ctx.register::<PullModelProgress>()?;
        ctx.register::<PullModelStreamItem>()?;
        ctx.register::<CreateModelProgress>()?;
        ctx.register::<CreateModelStreamItem>()?;
        ctx.register::<StatusResponse>()?;
        Ok(vec![
            build_ollama_native_api(ctx)?,
            build_ollama_openai_api(ctx)?,
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAM_NOTE: &str = "returned when `stream` is true or omitted";
    const JSON_NOTE: &str = "returned when `stream` is false";

    fn named(name: &str) -> TypeRef {
        TypeRef::Named(name.to_string())
    }

    /// Asserts a native streaming endpoint's two shapes: an NDJSON stream of
    /// `item` ending at the kept `field == value` line, then a JSON `reply`.
    fn assert_native_stream_shapes(response: &[SuccessShape], item: &str, reply: &str, field: &str, value: serde_json::Value) {
        assert_eq!(response.len(), 2);
        let (stream, json) = (&response[0], &response[1]);
        assert_eq!(
            stream.shape,
            ResponseShape::Stream {
                framing: StreamFraming::Ndjson(named(item)),
                end: StreamEnd::FieldEquals { field: field.to_string(), value },
                keep: true,
            }
        );
        assert_eq!(stream.note.as_deref(), Some(STREAM_NOTE));
        assert_eq!(stream.media_type().as_deref(), Some("application/x-ndjson"));
        assert_eq!(json.shape, ResponseShape::Json(named(reply)));
        assert_eq!(json.note.as_deref(), Some(JSON_NOTE));
        assert_eq!(json.media_type().as_deref(), Some("application/json"));
        for shape in response {
            assert_eq!(shape.content_type, None);
            assert_eq!(shape.variant, None, "variant names are derived (`Stream`, `Json`)");
            assert!(shape.statuses.is_empty());
        }
    }

    /// Asserts an OpenAI-compatible endpoint's two shapes: a JSON `reply`, then
    /// an SSE stream of `item` ending at a consumed `[DONE]` data line.
    fn assert_openai_shapes(response: &[SuccessShape], reply: &str, item: &str) {
        assert_eq!(response.len(), 2);
        let (json, stream) = (&response[0], &response[1]);
        assert_eq!(json.shape, ResponseShape::Json(named(reply)));
        assert_eq!(json.note.as_deref(), Some("returned when `stream` is false or omitted"));
        assert_eq!(json.media_type().as_deref(), Some("application/json"));
        assert_eq!(
            stream.shape,
            ResponseShape::Stream {
                framing: StreamFraming::Sse(named(item)),
                end: StreamEnd::DataLineEquals("[DONE]".to_string()),
                keep: false,
            }
        );
        assert_eq!(stream.note.as_deref(), Some("returned when `stream` is true"));
        assert_eq!(stream.media_type().as_deref(), Some("text/event-stream"));
        for shape in response {
            assert_eq!(shape.content_type, None);
            assert_eq!(shape.variant, None, "variant names are derived (`Json`, `Stream`)");
            assert!(shape.statuses.is_empty());
        }
    }

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
        assert_native_stream_shapes(&endpoint.response, "GenerateStreamItem", "GenerateCompletion", "done", true.into());
    }

    #[test]
    fn native_chat_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Chat").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/chat");
        assert!(endpoint.request.is_some());
        assert_native_stream_shapes(&endpoint.response, "ChatStreamItem", "ChatCompletion", "done", true.into());
    }

    #[test]
    fn native_embeddings_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Embeddings").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/embeddings");
        assert!(endpoint.request.is_some());
        // Non-streaming JSON response
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Json { .. }));
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
        assert_native_stream_shapes(&endpoint.response, "PullModelStreamItem", "StatusResponse", "status", "success".into());
    }

    #[test]
    fn native_push_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "PushModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/push");
        assert_native_stream_shapes(&endpoint.response, "PullModelStreamItem", "StatusResponse", "status", "success".into());
    }

    #[test]
    fn native_copy_model_endpoint() {
        let api = define_ollama_native_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "CopyModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/copy");
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Empty));
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
        assert_native_stream_shapes(&endpoint.response, "CreateModelStreamItem", "StatusResponse", "status", "success".into());
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
        assert_openai_shapes(&endpoint.response, "OpenAIChatCompletionResponse", "OpenAIChatStreamChunk");
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
        assert_openai_shapes(&endpoint.response, "OpenAICompletionResponse", "OpenAICompletionStreamChunk");
    }

    #[test]
    fn openai_embeddings_endpoint() {
        let api = define_ollama_openai_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Embeddings").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/v1/embeddings");
        assert!(endpoint.request.is_some());
        // Non-streaming JSON response
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Json { .. }));
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

    #[test]
    fn native_api_has_module_path() {
        let api = define_ollama_native_api();
        assert_eq!(api.module_path, Some("ollama".to_string()));
    }

    #[test]
    fn native_api_has_request_suffix() {
        let api = define_ollama_native_api();
        assert_eq!(api.request_suffix, Some("NativeRequest".to_string()));
    }

    #[test]
    fn openai_api_has_module_path() {
        let api = define_ollama_openai_api();
        assert_eq!(api.module_path, Some("ollama".to_string()));
    }

    #[test]
    fn openai_api_has_request_suffix() {
        let api = define_ollama_openai_api();
        assert_eq!(api.request_suffix, Some("OaiRequest".to_string()));
    }

    #[test]
    fn apis_have_different_request_suffixes() {
        let native = define_ollama_native_api();
        let openai = define_ollama_openai_api();

        assert_ne!(native.request_suffix, openai.request_suffix);
    }

    #[test]
    fn apis_share_module_path() {
        let native = define_ollama_native_api();
        let openai = define_ollama_openai_api();

        assert_eq!(native.module_path, openai.module_path);
    }

    #[test]
    fn module_group_preserves_endpoint_and_registry_inventory() {
        let group = module_group().unwrap();
        assert_eq!(
            group.iter().flat_map(|api| api.endpoints.iter().map(|endpoint| endpoint.id.as_str())).collect::<Vec<_>>(),
            vec!["Generate", "Chat", "Embeddings", "ListModels", "ShowModel", "PullModel", "PushModel", "CopyModel", "DeleteModel", "CreateModel", "ListRunningModels", "ChatCompletions", "Completions", "Embeddings", "ListModels"]
        );
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["GenerateBody", "ChatBody", "Message", "EmbeddingsBody", "ShowModelBody", "PullModelBody", "PushModelBody", "CopyModelBody", "DeleteModelBody", "CreateModelBody", "EmbeddingsResponse", "ListModelsResponse", "ModelDetails", "ModelInfo", "ShowModelResponse", "ListRunningModelsResponse", "RunningModel", "OpenAIChatCompletionRequest", "OpenAIMessage", "OpenAICompletionRequest", "OpenAIEmbeddingRequest", "OpenAIEmbeddingResponse", "OpenAIEmbeddingData", "OpenAIUsage", "OpenAIListModelsResponse", "OpenAIModel", "OpenAIChatCompletionChoice", "OpenAIChatCompletionResponse", "OpenAIDelta", "OpenAIStreamChoice", "OpenAIChatStreamChunk", "OpenAICompletionChoice", "OpenAICompletionResponse", "OpenAICompletionStreamChunk", "StreamErrorLine", "GenerateStreamChunk", "GenerateStreamItem", "GenerateCompletion", "ChatStreamChunk", "ChatStreamItem", "ChatCompletion", "PullModelProgress", "PullModelStreamItem", "CreateModelProgress", "CreateModelStreamItem", "StatusResponse"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
    }

}
