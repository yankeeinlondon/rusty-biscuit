//! Ollama API types.
//!
//! This module contains request and response types for both the native Ollama API
//! (`/api/*` endpoints) and the OpenAI-compatible API (`/v1/*` endpoints).

use serde::{Deserialize, Serialize};

// =============================================================================
// Native API Types (/api/*)
// =============================================================================

/// Model generation options for native Ollama API.
///
/// Controls model behavior including sampling parameters, context window,
/// and hardware utilization.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelOptions {
    /// Context window size (default: 2048).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,

    /// Number of GPU layers to offload (-1 for all).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_gpu: Option<i32>,

    /// Number of CPU threads to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_thread: Option<u32>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,

    /// Sampling temperature (0.0-2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-K sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Top-P (nucleus) sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Repetition penalty (1.0 = no penalty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f64>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Enable Mirostat sampling (0, 1, or 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat: Option<u8>,

    /// Mirostat learning rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat_eta: Option<f64>,

    /// Mirostat target perplexity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat_tau: Option<f64>,

    /// Penalize newline tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalize_newline: Option<bool>,

    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// Tail-free sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tfs_z: Option<f64>,
}

/// A chat message in the native Ollama format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author (system, user, assistant).
    pub role: String,

    /// The content of the message.
    pub content: String,

    /// Optional images for multimodal models (base64 encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

/// Request body for the `/api/generate` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GenerateBody {
    /// Model name to use.
    pub model: String,

    /// The prompt to generate a response for.
    pub prompt: String,

    /// Optional system prompt to override the model's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Optional template to use for generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,

    /// Raw mode bypasses templating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<bool>,

    /// Enable streaming (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Model options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ModelOptions>,

    /// How long to keep the model loaded (e.g., "5m", "-1" for forever).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,

    /// Images for multimodal models (base64 encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,

    /// Context from a previous response to continue generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i64>>,
}

/// Response from the `/api/generate` endpoint (non-streaming).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateResponse {
    /// Model that generated the response.
    pub model: String,

    /// Timestamp of response creation.
    pub created_at: String,

    /// The generated text.
    pub response: String,

    /// Whether generation is complete.
    pub done: bool,

    /// Context for continuing the conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i64>>,

    /// Total time in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,

    /// Time spent loading the model in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,

    /// Number of tokens in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,

    /// Time spent evaluating the prompt in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,

    /// Number of tokens generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,

    /// Time spent generating tokens in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Request body for the `/api/chat` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatBody {
    /// Model name to use.
    pub model: String,

    /// Chat messages.
    pub messages: Vec<Message>,

    /// Enable streaming (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Model options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ModelOptions>,

    /// How long to keep the model loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,

    /// Format of the response (e.g., "json").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Response from the `/api/chat` endpoint (non-streaming).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Model that generated the response.
    pub model: String,

    /// Timestamp of response creation.
    pub created_at: String,

    /// The assistant's message.
    pub message: Message,

    /// Whether generation is complete.
    pub done: bool,

    /// Total time in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,

    /// Time spent loading the model in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,

    /// Number of tokens in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,

    /// Time spent evaluating the prompt in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,

    /// Number of tokens generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,

    /// Time spent generating tokens in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Request body for the `/api/embeddings` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingsBody {
    /// Model name to use.
    pub model: String,

    /// Text to generate embeddings for.
    pub prompt: String,

    /// Model options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ModelOptions>,

    /// How long to keep the model loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

/// Response from the `/api/embeddings` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    /// The embedding vector.
    pub embedding: Vec<f64>,
}

// -----------------------------------------------------------------------------
// Model Management Types
// -----------------------------------------------------------------------------

/// Model details in list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDetails {
    /// Model families (e.g., ["llama"]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub families: Option<Vec<String>>,

    /// Parameter size (e.g., "8B").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_size: Option<String>,

    /// Quantization level (e.g., "Q4_0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization_level: Option<String>,

    /// Parent model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_model: Option<String>,

    /// Format of the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Model family.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
}

/// A model in the list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name with tag (e.g., "llama3:latest").
    pub name: String,

    /// Model size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Model digest (truncated SHA256).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,

    /// Model modification time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,

    /// Detailed model information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ModelDetails>,
}

/// Response from the `/api/tags` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListModelsResponse {
    /// List of available models.
    pub models: Vec<ModelInfo>,
}

/// Request body for the `/api/show` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowModelBody {
    /// Model name to show.
    pub name: String,

    /// Include verbose details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
}

/// Response from the `/api/show` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowModelResponse {
    /// The Modelfile content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modelfile: Option<String>,

    /// Model parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<String>,

    /// Model template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,

    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Model details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ModelDetails>,

    /// Model information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_info: Option<serde_json::Value>,

    /// License information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// Request body for the `/api/pull` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullModelBody {
    /// Model name to pull.
    pub name: String,

    /// Enable streaming progress updates (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Allow insecure connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insecure: Option<bool>,
}

/// Progress response during model pull (streaming).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullModelProgress {
    /// Current status message.
    pub status: String,

    /// Digest being downloaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,

    /// Total size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,

    /// Completed bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
}

/// Request body for the `/api/push` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushModelBody {
    /// Model name to push.
    pub name: String,

    /// Enable streaming progress updates (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Allow insecure connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insecure: Option<bool>,
}

/// Progress response during model push (streaming).
pub type PushModelProgress = PullModelProgress;

/// Request body for the `/api/copy` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyModelBody {
    /// Source model name.
    pub source: String,

    /// Destination model name.
    pub destination: String,
}

/// Request body for the `/api/delete` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteModelBody {
    /// Model name to delete.
    pub name: String,
}

/// Request body for the `/api/create` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateModelBody {
    /// Name for the new model.
    pub name: String,

    /// Modelfile content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modelfile: Option<String>,

    /// Path to a Modelfile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Enable streaming progress updates (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Progress response during model creation (streaming).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateModelProgress {
    /// Current status message.
    pub status: String,
}

/// A running model in the `/api/ps` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunningModel {
    /// Model name.
    pub name: String,

    /// Model identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Model size in VRAM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Model digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,

    /// Model details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ModelDetails>,

    /// When the model will be unloaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,

    /// Size in VRAM in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_vram: Option<u64>,
}

/// Response from the `/api/ps` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListRunningModelsResponse {
    /// List of currently running models.
    pub models: Vec<RunningModel>,
}

// =============================================================================
// OpenAI-Compatible API Types (/v1/*)
// =============================================================================

/// A chat message in the OpenAI format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIMessage {
    /// The role of the message author.
    pub role: String,

    /// The content of the message.
    pub content: String,
}

/// Request body for the `/v1/chat/completions` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenAIChatCompletionRequest {
    /// Model name to use.
    pub model: String,

    /// Chat messages.
    pub messages: Vec<OpenAIMessage>,

    /// Sampling temperature (0.0-2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-P (nucleus) sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Enable streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
}

/// A choice in the chat completion response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIChatCompletionChoice {
    /// Index of this choice.
    pub index: u32,

    /// The message generated.
    pub message: OpenAIMessage,

    /// Why the model stopped generating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIUsage {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,

    /// Tokens generated.
    pub completion_tokens: u32,

    /// Total tokens used.
    pub total_tokens: u32,
}

/// Response from the `/v1/chat/completions` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIChatCompletionResponse {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type, always "chat.completion".
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// Model used for generation.
    pub model: String,

    /// List of completion choices.
    pub choices: Vec<OpenAIChatCompletionChoice>,

    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIUsage>,
}

/// Request body for the `/v1/completions` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenAICompletionRequest {
    /// Model name to use.
    pub model: String,

    /// The prompt to complete.
    pub prompt: String,

    /// Sampling temperature (0.0-2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-P (nucleus) sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Enable streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Echo back the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>,
}

/// A choice in the completion response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAICompletionChoice {
    /// Index of this choice.
    pub index: u32,

    /// The generated text.
    pub text: String,

    /// Why the model stopped generating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Response from the `/v1/completions` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAICompletionResponse {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type, always "text_completion".
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// Model used for generation.
    pub model: String,

    /// List of completion choices.
    pub choices: Vec<OpenAICompletionChoice>,

    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIUsage>,
}

/// Request body for the `/v1/embeddings` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIEmbeddingRequest {
    /// Model name to use.
    pub model: String,

    /// Text to embed.
    pub input: String,
}

/// An embedding object in the response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIEmbeddingData {
    /// Object type, always "embedding".
    pub object: String,

    /// Index of this embedding.
    pub index: u32,

    /// The embedding vector.
    pub embedding: Vec<f64>,
}

/// Response from the `/v1/embeddings` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIEmbeddingResponse {
    /// Object type, always "list".
    pub object: String,

    /// List of embeddings.
    pub data: Vec<OpenAIEmbeddingData>,

    /// Model used for embeddings.
    pub model: String,

    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIUsage>,
}

/// A model in the OpenAI-compatible list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIModel {
    /// Model identifier.
    pub id: String,

    /// Object type, always "model".
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// Owner of the model (always "library" for Ollama).
    pub owned_by: String,
}

/// Response from the `/v1/models` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIListModelsResponse {
    /// Object type, always "list".
    pub object: String,

    /// List of available models.
    pub data: Vec<OpenAIModel>,
}

// =============================================================================
// Streaming Types
// =============================================================================

/// A streaming chunk from the `/api/chat` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    /// Model that generated the response.
    pub model: String,

    /// Timestamp of chunk creation.
    pub created_at: String,

    /// The message chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,

    /// Whether generation is complete.
    pub done: bool,

    /// Total time in nanoseconds (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,

    /// Number of tokens in the prompt (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,

    /// Number of tokens generated (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
}

/// A streaming chunk from the `/api/generate` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateStreamChunk {
    /// Model that generated the response.
    pub model: String,

    /// Timestamp of chunk creation.
    pub created_at: String,

    /// The generated text chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,

    /// Whether generation is complete.
    pub done: bool,

    /// Context for continuing generation (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i64>>,

    /// Total time in nanoseconds (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,

    /// Number of tokens in the prompt (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,

    /// Number of tokens generated (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
}

/// Delta content in OpenAI streaming format.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIDelta {
    /// The role (only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// The content chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// A choice in the streaming response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIStreamChoice {
    /// Index of this choice.
    pub index: u32,

    /// The delta content.
    pub delta: OpenAIDelta,

    /// Why the model stopped generating (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// A streaming chunk from the `/v1/chat/completions` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIChatStreamChunk {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type, always "chat.completion.chunk".
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// Model used for generation.
    pub model: String,

    /// List of delta choices.
    pub choices: Vec<OpenAIStreamChoice>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_options_default() {
        let options = ModelOptions::default();
        assert!(options.num_ctx.is_none());
        assert!(options.temperature.is_none());
    }

    #[test]
    fn model_options_serialization() {
        let options = ModelOptions {
            num_ctx: Some(4096),
            temperature: Some(0.7),
            ..Default::default()
        };

        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"num_ctx\":4096"));
        assert!(json.contains("\"temperature\":0.7"));
        // Optional fields should be skipped
        assert!(!json.contains("num_gpu"));

        let parsed: ModelOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.num_ctx, Some(4096));
    }

    #[test]
    fn message_serialization() {
        let msg = Message {
            role: "user".to_string(),
            content: "Hello!".to_string(),
            images: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "Hello!");
    }

    #[test]
    fn generate_request_serialization() {
        let request = GenerateBody {
            model: "llama3".to_string(),
            prompt: "Tell me a story".to_string(),
            system: None,
            template: None,
            raw: None,
            stream: Some(false),
            options: Some(ModelOptions {
                temperature: Some(0.8),
                ..Default::default()
            }),
            keep_alive: None,
            images: None,
            context: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: GenerateBody = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model, "llama3");
        assert_eq!(parsed.stream, Some(false));
    }

    #[test]
    fn generate_response_deserialization() {
        let json = r#"{
            "model": "llama3",
            "created_at": "2024-01-01T00:00:00Z",
            "response": "Once upon a time...",
            "done": true,
            "total_duration": 123456789
        }"#;

        let response: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "llama3");
        assert!(response.done);
        assert_eq!(response.total_duration, Some(123456789));
    }

    #[test]
    fn chat_request_serialization() {
        let request = ChatBody {
            model: "llama3".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
                images: None,
            }],
            stream: Some(false),
            options: None,
            keep_alive: None,
            format: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"llama3\""));
        assert!(json.contains("\"messages\":["));
    }

    #[test]
    fn chat_response_deserialization() {
        let json = r#"{
            "model": "llama3",
            "created_at": "2024-01-01T00:00:00Z",
            "message": {"role": "assistant", "content": "Hello!"},
            "done": true
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "llama3");
        assert_eq!(response.message.role, "assistant");
    }

    #[test]
    fn embeddings_request_serialization() {
        let request = EmbeddingsBody {
            model: "llama3".to_string(),
            prompt: "Hello world".to_string(),
            options: None,
            keep_alive: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: EmbeddingsBody = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.prompt, "Hello world");
    }

    #[test]
    fn list_models_response_deserialization() {
        let json = r#"{
            "models": [
                {
                    "name": "llama3:latest",
                    "size": 4150000000,
                    "digest": "365c0bd3c000",
                    "details": {
                        "families": ["llama"],
                        "parameter_size": "8B",
                        "quantization_level": "Q4_0"
                    }
                }
            ]
        }"#;

        let response: ListModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].name, "llama3:latest");
    }

    #[test]
    fn pull_model_progress_deserialization() {
        let json = r#"{"status": "downloading sha256:abc123", "total": 5000, "completed": 1000}"#;

        let progress: PullModelProgress = serde_json::from_str(json).unwrap();
        assert!(progress.status.contains("downloading"));
        assert_eq!(progress.total, Some(5000));
    }

    #[test]
    fn openai_chat_completion_request_serialization() {
        let request = OpenAIChatCompletionRequest {
            model: "llama3".to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            top_p: None,
            max_tokens: Some(100),
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"llama3\""));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn openai_chat_completion_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "llama3",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }]
        }"#;

        let response: OpenAIChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, "Hello!");
    }

    #[test]
    fn openai_list_models_response_deserialization() {
        let json = r#"{
            "object": "list",
            "data": [
                {
                    "id": "llama3",
                    "object": "model",
                    "created": 1686935002,
                    "owned_by": "library"
                }
            ]
        }"#;

        let response: OpenAIListModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.object, "list");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "llama3");
    }

    #[test]
    fn openai_embedding_response_deserialization() {
        let json = r#"{
            "object": "list",
            "data": [{
                "object": "embedding",
                "index": 0,
                "embedding": [0.1, 0.2, 0.3]
            }],
            "model": "llama3"
        }"#;

        let response: OpenAIEmbeddingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].embedding.len(), 3);
    }

    #[test]
    fn chat_stream_chunk_deserialization() {
        let json = r#"{
            "model": "llama3",
            "created_at": "2024-01-01T00:00:00Z",
            "message": {"role": "assistant", "content": "Hi"},
            "done": false
        }"#;

        let chunk: ChatStreamChunk = serde_json::from_str(json).unwrap();
        assert!(!chunk.done);
        assert!(chunk.message.is_some());
    }

    #[test]
    fn openai_stream_chunk_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1677652288,
            "model": "llama3",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        }"#;

        let chunk: OpenAIChatStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn running_model_deserialization() {
        let json = r#"{
            "name": "llama3:latest",
            "model": "llama3",
            "size": 4000000000,
            "expires_at": "2024-01-01T00:05:00Z"
        }"#;

        let model: RunningModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.name, "llama3:latest");
        assert!(model.expires_at.is_some());
    }
}
