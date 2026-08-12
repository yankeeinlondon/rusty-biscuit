//! Auto-generated provider model enum
//!
//! Generated: 2026-07-02T21:18:37.062005+00:00
//! Generator: gen-models v0.1.0
//! Provider: MoonshotAi
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [Moonshot AI (Kimi)](<https://moonshot.ai>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelMoonshotAi {
    /// Model: `kimi-k2.5`
    Kimi__K2_5,
    /// Model: `kimi-k2.6`
    Kimi__K2_6,
    /// Model: `kimi-k2.7-code`
    Kimi__K2_7__Code,
    /// Model: `kimi-k2.7-code-highspeed`
    Kimi__K2_7__Code__Highspeed,
    /// Model: `moonshot-v1-128k`
    Moonshot__V1__128k,
    /// Model: `moonshot-v1-128k-vision-preview`
    Moonshot__V1__128k__Vision__Preview,
    /// Model: `moonshot-v1-32k`
    Moonshot__V1__32k,
    /// Model: `moonshot-v1-32k-vision-preview`
    Moonshot__V1__32k__Vision__Preview,
    /// Model: `moonshot-v1-8k`
    Moonshot__V1__8k,
    /// Model: `moonshot-v1-8k-vision-preview`
    Moonshot__V1__8k__Vision__Preview,
    /// Model: `moonshot-v1-auto`
    Moonshot__V1__Auto,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
