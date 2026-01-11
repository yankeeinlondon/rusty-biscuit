//! Auto-generated provider model enum
//!
//! Generated: 2026-01-11T20:35:19.183608+00:00
//! Generator: gen-models v0.1.0
//! Provider: OpenAi
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [OpenAI](<https://openai.com>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelOpenAi {
    /// Model: `gpt-3.5-turbo`
    Gpt__3_5__Turbo,
    /// Model: `gpt-5-search-api`
    Gpt__5__Search__Api,
    /// Model: `gpt-5.1-codex`
    Gpt__5_1__Codex,
    /// Model: `gpt-5.2`
    Gpt__5_2,
    /// Model: `gpt-5.2-chat-latest`
    Gpt__5_2__Chat__Latest,
    /// Model: `o3`
    O3,
    /// Model: `o3-mini`
    O3__Mini,
    /// Model: `o3-mini-2025-01-31`
    O3__Mini__2025__01__31,
    /// Model: `o4-mini`
    O4__Mini,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
