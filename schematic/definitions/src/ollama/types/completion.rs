use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Model generation options for native Ollama API.
///
/// Controls model behavior including sampling parameters, context window,
/// and hardware utilization.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddingsResponse {
    /// The embedding vector.
    pub embedding: Vec<f64>,
}

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
}
