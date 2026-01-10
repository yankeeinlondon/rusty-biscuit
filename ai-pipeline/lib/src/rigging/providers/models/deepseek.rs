//! Auto-generated provider model enum
//!
//! Generated: 2026-01-10T01:34:00.551908+00:00
//! Generator: gen-models v0.1.0
//! Provider: Deepseek
//!
//! Do not edit manually.

use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelDeepseek {
    /// Model: `deepseek-chat`
    Deepseek__Chat,
    /// Model: `deepseek-reasoner`
    Deepseek__Reasoner,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
