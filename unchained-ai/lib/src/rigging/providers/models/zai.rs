//! Auto-generated provider model enum
//!
//! Generated: 2026-05-07T02:07:05.704419+00:00
//! Generator: gen-models v0.1.0
//! Provider: Zai
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [Zhipu AI (Z.ai)](<https://zhipuai.cn>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelZai {
    /// Model: `glm-4.5`
    Glm__4_5,
    /// Model: `glm-4.5-air`
    Glm__4_5__Air,
    /// Model: `glm-4.6`
    Glm__4_6,
    /// Model: `glm-4.7`
    Glm__4_7,
    /// Model: `glm-5`
    Glm__5,
    /// Model: `glm-5-turbo`
    Glm__5__Turbo,
    /// Model: `glm-5.1`
    Glm__5_1,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
