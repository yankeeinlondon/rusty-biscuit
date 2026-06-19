//! Ollama local model enum
//!
//! Ollama models are not fetched from a provider API; they are hosted
//! locally and selected by the user. This enum therefore only provides a
//! `Bespoke` variant plus a few common defaults.

use model_id::ModelId;

/// Models provided by a local [Ollama](<https://ollama.com>) server.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelOllama {
    /// Model: `llama3.1`
    Llama__3_1,
    /// Model: `llama3.2`
    Llama__3_2,
    /// Model: `qwen2.5:14b`
    Qwen__2_5__14b,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
