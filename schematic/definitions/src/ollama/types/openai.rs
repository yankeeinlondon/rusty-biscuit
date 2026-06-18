use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A chat message in the OpenAI format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OpenAIMessage {
    /// The role of the message author.
    pub role: String,

    /// The content of the message.
    pub content: String,
}

/// Request body for the `/v1/chat/completions` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OpenAIEmbeddingRequest {
    /// Model name to use.
    pub model: String,

    /// Text to embed.
    pub input: String,
}

/// An embedding object in the response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OpenAIEmbeddingData {
    /// Object type, always "embedding".
    pub object: String,

    /// Index of this embedding.
    pub index: u32,

    /// The embedding vector.
    pub embedding: Vec<f64>,
}

/// Response from the `/v1/embeddings` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OpenAIListModelsResponse {
    /// Object type, always "list".
    pub object: String,

    /// List of available models.
    pub data: Vec<OpenAIModel>,
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
}
