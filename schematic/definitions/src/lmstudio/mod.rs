//! LM Studio API definitions.
//!
//! This module provides definitions for the LM Studio v1 native API (`/api/v1/*` endpoints).
//! LM Studio runs locally and supports optional bearer token authentication.
//!
//! ## Native API
//!
//! The v1 REST API provides enhanced features:
//!
//! - Chat inference as one JSON reply, or SSE events when `stream` is true
//! - Model management: load, unload, list
//! - Model downloads as jobs whose progress is polled
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
use schematic_define::{ResponseShape, RestMethod};
use schematic_define::{
    AuthStrategy, Endpoint, EnvMapping, RestApi, Success,
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
/// | Chat | POST | /api/v1/chat | Chat completion, JSON or SSE when `stream` is true |
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
                .returns(
                    Success::json(ctx.register::<ChatResult>()?)
                        .note("returned when `stream` is false or omitted"),
                )
                .returns(
                    Success::sse(ctx.register::<ChatStreamEvent>()?)
                        .ends_at_event("chat.end")
                        // `chat.end` carries the aggregated result.
                        .keep_end()
                        .note("returned when `stream` is true"),
                )
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
                .describes("Download a model from HuggingFace")
                .request_json::<DownloadModelBody>(ctx)
                // The provider replies with one JSON job object; progress is
                // polled through `GetDownloadStatus`, not streamed.
                .returns_json::<DownloadModelResponse>(ctx)
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
    use schematic_define::{StreamEnd, StreamFraming, SuccessShape, TypeRef};

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
    }

    #[test]
    fn chat_returns_json_result_or_sse_events_ending_at_kept_chat_end() {
        let api = define_lmstudio_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Chat").unwrap();

        assert_eq!(endpoint.response.len(), 2);
        let (json, sse) = (&endpoint.response[0], &endpoint.response[1]);

        assert_eq!(json.shape, ResponseShape::Json(TypeRef::Named("ChatResult".into())));
        assert_eq!(json.note.as_deref(), Some("returned when `stream` is false or omitted"));
        assert_eq!(json.content_type, None);
        assert_eq!(json.media_type().as_deref(), Some("application/json"));

        assert_eq!(
            sse.shape,
            ResponseShape::Stream {
                framing: StreamFraming::Sse(TypeRef::Named("ChatStreamEvent".into())),
                end: StreamEnd::EventNamed("chat.end".into()),
                keep: true,
            }
        );
        assert_eq!(sse.note.as_deref(), Some("returned when `stream` is true"));
        assert_eq!(sse.content_type, None);
        assert_eq!(sse.media_type().as_deref(), Some("text/event-stream"));

        for shape in &endpoint.response {
            assert!(shape.statuses.is_empty());
            assert!(shape.json_tag.is_none());
            assert!(!shape.fallback);
            assert!(shape.variant.is_none());
        }
    }

    #[test]
    fn list_models_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListModels").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/api/v1/models");
        assert!(endpoint.request.is_none());
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Json { .. }));
    }

    #[test]
    fn load_model_endpoint() {
        let api = define_lmstudio_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "LoadModel").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/api/v1/models/load");
        assert!(endpoint.request.is_some());
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Json { .. }));
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
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Json { .. }));
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
    }

    #[test]
    fn download_model_returns_one_json_job_object() {
        let api = define_lmstudio_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "DownloadModel")
            .unwrap();

        assert_eq!(
            endpoint.response,
            vec![SuccessShape::json_type("DownloadModelResponse")]
        );
        assert_eq!(endpoint.response[0].media_type().as_deref(), Some("application/json"));
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
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Json { .. }));
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
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["ChatBody", "LoadModelBody", "UnloadModelBody", "DownloadModelBody", "DownloadStatusBody", "ListModelsResponse", "Capabilities", "LoadedInstance", "ModelInfo", "Quantization", "Stats", "LoadModelResponse", "UnloadModelResponse", "DownloadStatusResponse", "OutputItem", "ChatResult", "ChatStreamEvent", "DownloadModelResponse"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
    }

    #[test]
    fn chat_result_decodes_non_streaming_reply() {
        let json = r#"{
            "model_instance_id": "ibm/granite-4-micro",
            "output": [
                {"type": "reasoning", "content": "Need to answer briefly."},
                {
                    "type": "tool_call",
                    "tool": "model_search",
                    "arguments": {"sort": "trendingScore", "limit": 1},
                    "output": "[{\"id\":\"openai/gpt-oss-20b\"}]",
                    "provider_info": {"type": "ephemeral_mcp", "server_label": "huggingface"}
                },
                {
                    "type": "invalid_tool_call",
                    "reason": "Cannot find tool with name open_browser.",
                    "metadata": {"type": "invalid_name", "tool_name": "open_browser"}
                },
                {"type": "message", "content": "The trending model is gpt-oss-20b."}
            ],
            "stats": {
                "input_tokens": 329,
                "total_output_tokens": 268,
                "reasoning_output_tokens": 5,
                "tokens_per_second": 43.73,
                "time_to_first_token_seconds": 0.781,
                "model_load_time_seconds": 12.34
            },
            "response_id": "resp_02b2017dbc06c12bfc353a2ed6c2b802f8cc682884bb5716"
        }"#;

        let result: ChatResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.model_instance_id, "ibm/granite-4-micro");
        assert_eq!(result.output.len(), 4);
        assert!(matches!(
            &result.output[1],
            ChatOutputItem::ToolCall { provider_info: ToolProviderInfo::EphemeralMcp { server_label }, .. }
                if server_label == "huggingface"
        ));
        assert!(matches!(
            &result.output[2],
            ChatOutputItem::InvalidToolCall { metadata, .. } if metadata.type_ == "invalid_name" && metadata.arguments.is_none()
        ));
        assert_eq!(result.stats.input_tokens, 329);
        assert_eq!(result.stats.model_load_time_seconds, Some(12.34));
        assert!(result.response_id.unwrap().starts_with("resp_"));
    }

    #[test]
    fn chat_stream_event_decodes_progress_and_delta_events() {
        let decode = |data: &str| serde_json::from_str::<ChatStreamEvent>(data).unwrap();

        assert_eq!(
            decode(r#"{"type":"chat.start","model_instance_id":"openai/gpt-oss-20b"}"#),
            ChatStreamEvent::ChatStart { model_instance_id: "openai/gpt-oss-20b".into() }
        );
        assert_eq!(
            decode(r#"{"type":"model_load.progress","model_instance_id":"openai/gpt-oss-20b","progress":0.65}"#),
            ChatStreamEvent::ModelLoadProgress { model_instance_id: "openai/gpt-oss-20b".into(), progress: 0.65 }
        );
        assert_eq!(decode(r#"{"type":"prompt_processing.start"}"#), ChatStreamEvent::PromptProcessingStart);
        assert_eq!(
            decode(r#"{"type":"reasoning.delta","content":"Need to"}"#),
            ChatStreamEvent::ReasoningDelta { content: "Need to".into() }
        );
        assert_eq!(
            decode(r#"{"type":"message.delta","content":"The current"}"#),
            ChatStreamEvent::MessageDelta { content: "The current".into() }
        );
        assert_eq!(decode(r#"{"type":"message.end"}"#), ChatStreamEvent::MessageEnd);
    }

    #[test]
    fn chat_stream_event_decodes_tool_call_events() {
        let success: ChatStreamEvent = serde_json::from_str(
            r#"{"type":"tool_call.success","tool":"model_search","arguments":{"limit":1},"output":"[]","provider_info":{"type":"plugin","plugin_id":"lmstudio/search"}}"#,
        )
        .unwrap();
        assert!(matches!(
            success,
            ChatStreamEvent::ToolCallSuccess { provider_info: ToolProviderInfo::Plugin { plugin_id }, .. }
                if plugin_id == "lmstudio/search"
        ));

        let failure: ChatStreamEvent = serde_json::from_str(
            r#"{"type":"tool_call.failure","reason":"bad arguments","metadata":{"type":"invalid_arguments","tool_name":"model_search","arguments":{"limit":"x"},"provider_info":{"type":"ephemeral_mcp","server_label":"huggingface"}}}"#,
        )
        .unwrap();
        assert!(matches!(
            failure,
            ChatStreamEvent::ToolCallFailure { metadata, .. }
                if metadata.type_ == "invalid_arguments" && metadata.provider_info.is_some()
        ));
    }

    #[test]
    fn chat_stream_event_decodes_error_into_typed_variant() {
        let event: ChatStreamEvent = serde_json::from_str(
            r#"{"type":"error","error":{"type":"invalid_request","message":"\"model\" is required","code":"missing_required_parameter","param":"model"}}"#,
        )
        .unwrap();

        assert_eq!(
            event,
            ChatStreamEvent::Error {
                error: ChatStreamErrorDetail {
                    type_: "invalid_request".into(),
                    message: "\"model\" is required".into(),
                    code: Some("missing_required_parameter".into()),
                    param: Some("model".into()),
                },
            }
        );

        let minimal: ChatStreamEvent =
            serde_json::from_str(r#"{"type":"error","error":{"type":"unknown","message":"boom"}}"#).unwrap();
        assert!(matches!(minimal, ChatStreamEvent::Error { error } if error.code.is_none() && error.param.is_none()));
    }

    #[test]
    fn chat_stream_event_decodes_chat_end_aggregated_result() {
        let event: ChatStreamEvent = serde_json::from_str(
            r#"{"type":"chat.end","result":{"model_instance_id":"openai/gpt-oss-20b","output":[{"type":"message","content":"Hi."}],"stats":{"input_tokens":10,"total_output_tokens":3,"reasoning_output_tokens":0,"tokens_per_second":40.0,"time_to_first_token_seconds":0.2}}}"#,
        )
        .unwrap();

        let ChatStreamEvent::ChatEnd { result } = event else { panic!("expected chat.end, got {event:?}") };
        assert_eq!(result.output, vec![ChatOutputItem::Message { content: "Hi.".into() }]);
        assert_eq!(result.stats.total_output_tokens, 3);
        assert_eq!(result.response_id, None);
    }

    #[test]
    fn chat_stream_event_rejects_unknown_event_type() {
        assert!(serde_json::from_str::<ChatStreamEvent>(r#"{"type":"chat.paused"}"#).is_err());
    }

    #[test]
    fn download_model_response_decodes_job_and_already_downloaded() {
        let job: DownloadModelResponse = serde_json::from_str(
            r#"{"job_id":"job_493c7c9ded","status":"downloading","total_size_bytes":2279145003,"started_at":"2025-10-03T15:33:23.496Z"}"#,
        )
        .unwrap();
        assert_eq!(job.job_id.as_deref(), Some("job_493c7c9ded"));
        assert_eq!(job.total_size_bytes, Some(2_279_145_003));

        let done: DownloadModelResponse = serde_json::from_str(r#"{"status":"already_downloaded"}"#).unwrap();
        assert_eq!(done.job_id, None);
        assert_eq!(done.status, "already_downloaded");
    }

}
