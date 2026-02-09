//! LM Studio API types.
//!
//! This module contains request and response types for the LM Studio v1 native API
//! (`/api/v1/*` endpoints). LM Studio runs locally and supports optional bearer token
//! authentication.

use serde::{Deserialize, Serialize};

// =============================================================================
// Chat Types
// =============================================================================

/// A chat message in the LM Studio format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author (system, user, assistant).
    pub role: String,

    /// The content of the message.
    pub content: String,
}

/// Request body for the `/api/v1/chat` endpoint.
///
/// The chat endpoint supports SSE streaming by default.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatBody {
    /// Model identifier to use for generation.
    pub model: String,

    /// Chat messages.
    pub messages: Vec<Message>,

    /// Context length override (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,

    /// Whether to stream the response (default: true).
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

// =============================================================================
// Model Management Types
// =============================================================================

/// Quantization information for a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantization {
    /// The quantization format (e.g., "Q4_0", "Q8_0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Model capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Whether the model supports vision/multimodal input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision: Option<bool>,

    /// Whether the model supports function calling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling: Option<bool>,
}

/// A loaded model instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadedInstance {
    /// The instance identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Current context length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,
}

/// Statistics about a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    /// Model size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Number of parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<String>,
}

/// Information about a model in the list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Response from the `/api/v1/models` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListModelsResponse {
    /// Object type (always "list").
    pub object: String,

    /// List of available models.
    pub data: Vec<ModelInfo>,
}

// =============================================================================
// Load Model Types
// =============================================================================

/// Configuration for an MCP integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integration {
    /// The MCP server name or identifier.
    pub name: String,
}

/// Load configuration options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LoadConfig {
    /// Context length to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,

    /// GPU offload layers (-1 for all).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_offload: Option<i32>,
}

/// Request body for the `/api/v1/models/load` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

/// Response from the `/api/v1/models/load` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadModelResponse {
    /// The loaded model's identifier.
    pub model: String,

    /// Load status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

// =============================================================================
// Unload Model Types
// =============================================================================

/// Request body for the `/api/v1/models/unload` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnloadModelBody {
    /// Model identifier to unload.
    pub model: String,
}

/// Response from the `/api/v1/models/unload` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnloadModelResponse {
    /// Unload status message.
    pub status: String,
}

// =============================================================================
// Download Model Types
// =============================================================================

/// Request body for the `/api/v1/models/download` endpoint.
///
/// The download endpoint supports SSE streaming for progress updates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadModelBody {
    /// Model identifier to download (e.g., "lmstudio-community/gemma-2-2b-it-GGUF").
    pub model: String,
}

/// Response from the `/api/v1/models/download` endpoint.
///
/// Initial response before streaming begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadModelResponse {
    /// Download job identifier.
    pub job_id: String,

    /// Download status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

// =============================================================================
// Download Status Types
// =============================================================================

/// An output item in the download status response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request body for the `/api/v1/models/download/status` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadStatusBody {
    /// The job ID to check status for.
    pub job_id: String,
}

/// Response from the `/api/v1/models/download/status` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

// =============================================================================
// Streaming Types
// =============================================================================

/// A streaming chunk from the `/api/v1/chat` endpoint (SSE format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    /// The chunk content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Whether this is the final chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,

    /// Usage statistics (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ChatUsage>,
}

/// Token usage statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatUsage {
    /// Tokens in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,

    /// Tokens generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,

    /// Total tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
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
        assert_eq!(
            response.data[0].id,
            "lmstudio-community/gemma-2-2b-it-GGUF"
        );
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

    #[test]
    fn chat_stream_chunk_deserialization() {
        let json = r#"{
            "content": "Hello",
            "done": false
        }"#;

        let chunk: ChatStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.content, Some("Hello".to_string()));
        assert_eq!(chunk.done, Some(false));
    }
}
