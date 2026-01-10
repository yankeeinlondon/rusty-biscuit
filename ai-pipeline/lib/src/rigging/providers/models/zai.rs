//! Auto-generated provider model enum
//!
//! Generated: 2026-01-10T01:34:02.976193+00:00
//! Generator: gen-models v0.1.0
//! Provider: Zai
//!
//! Do not edit manually.

use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelZai {
    /// Model: `glm-4.5`
    Glm__4_5,
    /// Model: `glm-4.5-air`
    Glm__4_5__Air,
    /// Model: `glm-4.6`
    Glm__4_6,
    /// Model: `glm-4.7`
    Glm__4_7,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
