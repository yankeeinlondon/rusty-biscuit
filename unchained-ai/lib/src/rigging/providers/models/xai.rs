//! Auto-generated provider model enum
//!
//! Generated: 2026-07-06T22:44:36.162013+00:00
//! Generator: gen-models v0.1.0
//! Provider: Xai
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [xAI](<https://x.ai>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelXai {
    /// Model: `grok-4.20-0309-non-reasoning`
    Grok__4_20__0309__Non__Reasoning,
    /// Model: `grok-4.20-0309-reasoning`
    Grok__4_20__0309__Reasoning,
    /// Model: `grok-4.20-multi-agent-0309`
    Grok__4_20__Multi__Agent__0309,
    /// Model: `grok-4.3`
    Grok__4_3,
    /// Model: `grok-build-0.1`
    Grok__Build__0_1,
    /// Model: `grok-imagine-image`
    Grok__Imagine__Image,
    /// Model: `grok-imagine-image-quality`
    Grok__Imagine__Image__Quality,
    /// Model: `grok-imagine-video`
    Grok__Imagine__Video,
    /// Model: `grok-imagine-video-1.5`
    Grok__Imagine__Video__1_5,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
