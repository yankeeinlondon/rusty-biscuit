//! LM Studio API types.
//!
//! This module contains request and response types for the LM Studio v1 native API
//! (`/api/v1/*` endpoints). LM Studio runs locally and supports optional bearer token
//! authentication.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// =============================================================================
// Chat Types
// =============================================================================

/// A chat message in the LM Studio format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Message {
    /// The role of the message author (system, user, assistant).
    pub role: String,

    /// The content of the message.
    pub content: String,
}

/// Request body for the `/api/v1/chat` endpoint.
///
/// The reply is a [`ChatResult`] unless `stream` is `true`, in which case it
/// is a stream of [`ChatStreamEvent`]s.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChatBody {
    /// Model identifier to use for generation.
    pub model: String,

    /// Chat messages.
    pub messages: Vec<Message>,

    /// Context length override (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,

    /// Whether to stream the response as SSE (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Sampling temperature (0.0-2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

impl schematic_define::Registration for ChatBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ChatBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

// =============================================================================
// Model Management Types
// =============================================================================

/// Quantization information for a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Quantization {
    /// The quantization format (e.g., "Q4_0", "Q8_0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl schematic_define::Registration for Quantization {
    const RUST_IDENTIFIER: Option<&'static str> = Some("Quantization");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Model capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Capabilities {
    /// Whether the model supports vision/multimodal input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision: Option<bool>,

    /// Whether the model supports function calling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling: Option<bool>,
}

impl schematic_define::Registration for Capabilities {
    const RUST_IDENTIFIER: Option<&'static str> = Some("Capabilities");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// A loaded model instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LoadedInstance {
    /// The instance identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Current context length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,
}

impl schematic_define::Registration for LoadedInstance {
    const RUST_IDENTIFIER: Option<&'static str> = Some("LoadedInstance");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Statistics about a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Stats {
    /// Model size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Number of parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<String>,
}

impl schematic_define::Registration for Stats {
    const RUST_IDENTIFIER: Option<&'static str> = Some("Stats");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Information about a model in the list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelInfo {
    /// Model identifier (path).
    pub id: String,

    /// Object type (always "model").
    pub object: String,

    /// Model type (e.g., "llm").
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Publisher/author of the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    /// Architecture of the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,

    /// Compatibility type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_type: Option<String>,

    /// Quantization details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<Quantization>,

    /// Model statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<Stats>,

    /// Model capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,

    /// Currently loaded instances.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loaded_instances: Option<Vec<LoadedInstance>>,
}

impl schematic_define::Registration for ModelInfo {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ModelInfo");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Response from the `/api/v1/models` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListModelsResponse {
    /// Object type (always "list").
    pub object: String,

    /// List of available models.
    pub data: Vec<ModelInfo>,
}

impl schematic_define::Registration for ListModelsResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ListModelsResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

// =============================================================================
// Load Model Types
// =============================================================================

/// Configuration for an MCP integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Integration {
    /// The MCP server name or identifier.
    pub name: String,
}

/// Load configuration options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LoadConfig {
    /// Context length to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,

    /// GPU offload layers (-1 for all).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_offload: Option<i32>,
}

/// Request body for the `/api/v1/models/load` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LoadModelBody {
    /// Model identifier to load.
    pub model: String,

    /// Optional load configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<LoadConfig>,

    /// Optional MCP integrations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrations: Option<Vec<Integration>>,
}

impl schematic_define::Registration for LoadModelBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("LoadModelBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Response from the `/api/v1/models/load` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LoadModelResponse {
    /// The loaded model's identifier.
    pub model: String,

    /// Load status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl schematic_define::Registration for LoadModelResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("LoadModelResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

// =============================================================================
// Unload Model Types
// =============================================================================

/// Request body for the `/api/v1/models/unload` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UnloadModelBody {
    /// Model identifier to unload.
    pub model: String,
}

impl schematic_define::Registration for UnloadModelBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("UnloadModelBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Response from the `/api/v1/models/unload` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UnloadModelResponse {
    /// Unload status message.
    pub status: String,
}

impl schematic_define::Registration for UnloadModelResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("UnloadModelResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

// =============================================================================
// Download Model Types
// =============================================================================

/// Request body for the `/api/v1/models/download` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DownloadModelBody {
    /// Model identifier to download (e.g., "lmstudio-community/gemma-2-2b-it-GGUF").
    pub model: String,
}

impl schematic_define::Registration for DownloadModelBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("DownloadModelBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Response from the `/api/v1/models/download` endpoint: the download job,
/// returned as one JSON object (the endpoint does not stream).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DownloadModelResponse {
    /// Download job identifier; absent when `status` is `already_downloaded`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,

    /// Job status: `downloading`, `paused`, `completed`, `failed`, or
    /// `already_downloaded`.
    pub status: String,

    /// Total download size in bytes; absent when `status` is
    /// `already_downloaded`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_size_bytes: Option<u64>,

    /// ISO 8601 start time; absent when `status` is `already_downloaded`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,

    /// ISO 8601 completion time; present when `status` is `completed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

impl schematic_define::Registration for DownloadModelResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("DownloadModelResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

// =============================================================================
// Download Status Types
// =============================================================================

/// An output item in the download status response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OutputItem {
    /// Type of output (e.g., "file").
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Path where the file is being downloaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Download progress (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
}

impl schematic_define::Registration for OutputItem {
    const RUST_IDENTIFIER: Option<&'static str> = Some("OutputItem");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Request body for the `/api/v1/models/download/status` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DownloadStatusBody {
    /// The job ID to check status for.
    pub job_id: String,
}

impl schematic_define::Registration for DownloadStatusBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("DownloadStatusBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Response from the `/api/v1/models/download/status` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DownloadStatusResponse {
    /// Current status (e.g., "downloading", "completed", "failed").
    pub status: String,

    /// Overall progress (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,

    /// Output items being downloaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<OutputItem>>,

    /// Error message if download failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl schematic_define::Registration for DownloadStatusResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("DownloadStatusResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

// =============================================================================
// Chat Result Types
// =============================================================================

/// The provider of a tool the model called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolProviderInfo {
    /// A tool provided by an LM Studio plugin.
    Plugin {
        /// The plugin identifier.
        plugin_id: String,
    },
    /// A tool provided by an ephemeral MCP server.
    EphemeralMcp {
        /// The MCP server label.
        server_label: String,
    },
}

impl schematic_define::Registration for ToolProviderInfo {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ToolProviderInfo");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Why a tool call the model attempted was rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InvalidToolCallMetadata {
    /// `invalid_name` or `invalid_arguments`.
    #[serde(rename = "type")]
    pub type_: String,

    /// The tool name the model used.
    pub tool_name: String,

    /// The arguments the model passed; only for `invalid_arguments`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,

    /// The tool provider; only for `invalid_arguments`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_info: Option<ToolProviderInfo>,
}

impl schematic_define::Registration for InvalidToolCallMetadata {
    const RUST_IDENTIFIER: Option<&'static str> = Some("InvalidToolCallMetadata");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// One item of a chat result's `output`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatOutputItem {
    /// Text of the model's reply.
    Message {
        /// The reply text.
        content: String,
    },
    /// A tool call the model made and its result.
    ToolCall {
        /// The tool name.
        tool: String,
        /// The arguments passed to the tool.
        arguments: serde_json::Value,
        /// The raw tool output.
        output: String,
        /// The tool provider.
        provider_info: ToolProviderInfo,
    },
    /// The model's reasoning text.
    Reasoning {
        /// The reasoning text.
        content: String,
    },
    /// A tool call that was rejected.
    InvalidToolCall {
        /// Why the call was invalid.
        reason: String,
        /// Details of the rejected call.
        metadata: InvalidToolCallMetadata,
    },
}

impl schematic_define::Registration for ChatOutputItem {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ChatOutputItem");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// Token usage and performance of one chat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChatStats {
    /// Input tokens, including formatting, tool definitions, and prior messages.
    pub input_tokens: u64,

    /// Total output tokens generated.
    pub total_output_tokens: u64,

    /// Output tokens spent on reasoning.
    pub reasoning_output_tokens: u64,

    /// Generation speed.
    pub tokens_per_second: f64,

    /// Seconds until the first token.
    pub time_to_first_token_seconds: f64,

    /// Seconds spent loading the model; only when the request loaded it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_load_time_seconds: Option<f64>,
}

impl schematic_define::Registration for ChatStats {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ChatStats");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// The reply of `/api/v1/chat` when `stream` is false or omitted, and the
/// `result` of the stream's final `chat.end` event.
///
/// Not named `ChatResponse`: that name belongs to the generated success enum
/// of the `Chat` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChatResult {
    /// The loaded model instance that generated the reply.
    pub model_instance_id: String,

    /// Output items in generation order.
    pub output: Vec<ChatOutputItem>,

    /// Token usage and performance.
    pub stats: ChatStats,

    /// Response identifier starting with `resp_`; present when `store` is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
}

impl schematic_define::Registration for ChatResult {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ChatResult");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// The `error` object of a chat stream `error` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChatStreamErrorDetail {
    /// The error category, such as `invalid_request` or `model_not_found`.
    #[serde(rename = "type")]
    pub type_: String,

    /// Human-readable description.
    pub message: String,

    /// A more specific error code, such as `missing_required_parameter`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// The request parameter the error relates to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

impl schematic_define::Registration for ChatStreamErrorDetail {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ChatStreamErrorDetail");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// One server-sent event of `/api/v1/chat` when `stream` is true, decoded
/// from the event's `data`, whose `type` repeats the SSE event name.
///
/// The stream ends with [`ChatStreamEvent::ChatEnd`]. An `error` event does
/// not end it: `chat.end` still follows with whatever was generated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ChatStreamEvent {
    /// The chat began.
    #[serde(rename = "chat.start")]
    ChatStart {
        /// The model instance serving the chat.
        model_instance_id: String,
    },
    /// The model began loading.
    #[serde(rename = "model_load.start")]
    ModelLoadStart {
        /// The model instance being loaded.
        model_instance_id: String,
    },
    /// Model loading progressed.
    #[serde(rename = "model_load.progress")]
    ModelLoadProgress {
        /// The model instance being loaded.
        model_instance_id: String,
        /// Progress from 0 to 1.
        progress: f64,
    },
    /// The model finished loading.
    #[serde(rename = "model_load.end")]
    ModelLoadEnd {
        /// The loaded model instance.
        model_instance_id: String,
        /// Seconds the load took.
        load_time_seconds: f64,
    },
    /// Prompt processing began.
    #[serde(rename = "prompt_processing.start")]
    PromptProcessingStart,
    /// Prompt processing progressed.
    #[serde(rename = "prompt_processing.progress")]
    PromptProcessingProgress {
        /// Progress from 0 to 1.
        progress: f64,
    },
    /// Prompt processing finished.
    #[serde(rename = "prompt_processing.end")]
    PromptProcessingEnd,
    /// Reasoning output began.
    #[serde(rename = "reasoning.start")]
    ReasoningStart,
    /// A fragment of reasoning text.
    #[serde(rename = "reasoning.delta")]
    ReasoningDelta {
        /// The text fragment.
        content: String,
    },
    /// Reasoning output finished.
    #[serde(rename = "reasoning.end")]
    ReasoningEnd,
    /// The model began a tool call.
    #[serde(rename = "tool_call.start")]
    ToolCallStart {
        /// The tool name.
        tool: String,
        /// The tool provider.
        provider_info: ToolProviderInfo,
    },
    /// The arguments of the current tool call are complete.
    #[serde(rename = "tool_call.arguments")]
    ToolCallArguments {
        /// The tool name.
        tool: String,
        /// The arguments, shaped by the tool.
        arguments: serde_json::Value,
        /// The tool provider.
        provider_info: ToolProviderInfo,
    },
    /// The tool call returned.
    #[serde(rename = "tool_call.success")]
    ToolCallSuccess {
        /// The tool name.
        tool: String,
        /// The arguments passed to the tool.
        arguments: serde_json::Value,
        /// The raw tool output.
        output: String,
        /// The tool provider.
        provider_info: ToolProviderInfo,
    },
    /// The tool call was rejected.
    #[serde(rename = "tool_call.failure")]
    ToolCallFailure {
        /// Why the call failed.
        reason: String,
        /// Details of the rejected call.
        metadata: InvalidToolCallMetadata,
    },
    /// Reply text began.
    #[serde(rename = "message.start")]
    MessageStart,
    /// A fragment of reply text.
    #[serde(rename = "message.delta")]
    MessageDelta {
        /// The text fragment.
        content: String,
    },
    /// Reply text finished.
    #[serde(rename = "message.end")]
    MessageEnd,
    /// An error occurred; the stream continues to `chat.end`.
    #[serde(rename = "error")]
    Error {
        /// The error details.
        error: ChatStreamErrorDetail,
    },
    /// The chat finished; the stream's terminator.
    #[serde(rename = "chat.end")]
    ChatEnd {
        /// The aggregated result, the same shape as the non-streaming reply.
        result: ChatResult,
    },
}

impl schematic_define::Registration for ChatStreamEvent {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ChatStreamEvent");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_serialization() {
        let msg = Message {
            role: "user".to_string(),
            content: "Hello!".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "Hello!");
    }

    #[test]
    fn chat_body_serialization() {
        let body = ChatBody {
            model: "gemma-2-2b-it".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: Some(true),
            temperature: Some(0.7),
            ..Default::default()
        };

        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"model\":\"gemma-2-2b-it\""));
        assert!(json.contains("\"temperature\":0.7"));

        let parsed: ChatBody = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model, "gemma-2-2b-it");
    }

    #[test]
    fn list_models_response_deserialization() {
        let json = r#"{
            "object": "list",
            "data": [
                {
                    "id": "lmstudio-community/gemma-2-2b-it-GGUF",
                    "object": "model",
                    "type": "llm",
                    "publisher": "lmstudio-community",
                    "architecture": "gemma2"
                }
            ]
        }"#;

        let response: ListModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.object, "list");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "lmstudio-community/gemma-2-2b-it-GGUF");
    }

    #[test]
    fn model_info_type_field_rename() {
        let json = r#"{
            "id": "test-model",
            "object": "model",
            "type": "llm"
        }"#;

        let model: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(model.type_, Some("llm".to_string()));

        // Serialization should use "type" not "type_"
        let serialized = serde_json::to_string(&model).unwrap();
        assert!(serialized.contains("\"type\":\"llm\""));
        assert!(!serialized.contains("type_"));
    }

    #[test]
    fn load_model_body_serialization() {
        let body = LoadModelBody {
            model: "gemma-2-2b-it".to_string(),
            config: Some(LoadConfig {
                context_length: Some(4096),
                gpu_offload: Some(-1),
            }),
            integrations: None,
        };

        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"context_length\":4096"));
        assert!(json.contains("\"gpu_offload\":-1"));
    }

    #[test]
    fn download_status_response_deserialization() {
        let json = r#"{
            "status": "downloading",
            "progress": 0.5,
            "output": [
                {
                    "type": "file",
                    "path": "/models/test.gguf",
                    "progress": 0.5
                }
            ]
        }"#;

        let response: DownloadStatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "downloading");
        assert_eq!(response.progress, Some(0.5));
        assert!(response.output.is_some());
    }

    #[test]
    fn output_item_type_field_rename() {
        let json = r#"{
            "type": "file",
            "path": "/test.gguf",
            "progress": 0.75
        }"#;

        let item: OutputItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.type_, Some("file".to_string()));

        let serialized = serde_json::to_string(&item).unwrap();
        assert!(serialized.contains("\"type\":\"file\""));
    }

    #[test]
    fn unload_model_body_serialization() {
        let body = UnloadModelBody {
            model: "gemma-2-2b-it".to_string(),
        };

        let json = serde_json::to_string(&body).unwrap();
        let parsed: UnloadModelBody = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model, "gemma-2-2b-it");
    }

    #[test]
    fn download_model_body_serialization() {
        let body = DownloadModelBody {
            model: "lmstudio-community/gemma-2-2b-it-GGUF".to_string(),
        };

        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("lmstudio-community/gemma-2-2b-it-GGUF"));
    }
}
