use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Model details in list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListModelsResponse {
    /// List of available models.
    pub models: Vec<ModelInfo>,
}

/// Request body for the `/api/show` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ShowModelBody {
    /// Model name to show.
    pub name: String,

    /// Include verbose details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
}

/// Response from the `/api/show` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

    /// Model capabilities (e.g., "completion", "vision", "tools", "thinking").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
}

/// Request body for the `/api/pull` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CopyModelBody {
    /// Source model name.
    pub source: String,

    /// Destination model name.
    pub destination: String,
}

/// Request body for the `/api/delete` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeleteModelBody {
    /// Model name to delete.
    pub name: String,
}

/// Request body for the `/api/create` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListRunningModelsResponse {
    /// List of currently running models.
    pub models: Vec<RunningModel>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
