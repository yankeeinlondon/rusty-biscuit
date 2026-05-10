//! Auto-generated provider model enum
//!
//! Generated: 2026-05-07T02:07:05.024388+00:00
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
    /// Model: `grok-3`
    Grok__3,
    /// Model: `grok-3-mini`
    Grok__3__Mini,
    /// Model: `grok-4-0709`
    Grok__4__0709,
    /// Model: `grok-4-1-fast-non-reasoning`
    Grok__4__1__Fast__Non__Reasoning,
    /// Model: `grok-4-1-fast-reasoning`
    Grok__4__1__Fast__Reasoning,
    /// Model: `grok-4-fast-non-reasoning`
    Grok__4__Fast__Non__Reasoning,
    /// Model: `grok-4-fast-reasoning`
    Grok__4__Fast__Reasoning,
    /// Model: `grok-4.20-0309-non-reasoning`
    Grok__4_20__0309__Non__Reasoning,
    /// Model: `grok-4.20-0309-reasoning`
    Grok__4_20__0309__Reasoning,
    /// Model: `grok-4.20-multi-agent-0309`
    Grok__4_20__Multi__Agent__0309,
    /// Model: `grok-4.3`
    Grok__4_3,
    /// Model: `grok-code-fast-1`
    Grok__Code__Fast__1,
    /// Model: `grok-imagine-image`
    Grok__Imagine__Image,
    /// Model: `grok-imagine-image-pro`
    Grok__Imagine__Image__Pro,
    /// Model: `grok-imagine-image-quality`
    Grok__Imagine__Image__Quality,
    /// Model: `grok-imagine-video`
    Grok__Imagine__Video,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
