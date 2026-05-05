//! Auto-generated provider model enum
//!
//! Generated: 2026-05-04T04:02:11.353446+00:00
//! Generator: gen-models v0.1.0
//! Provider: Anthropic
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [Anthropic](<https://anthropic.com>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelAnthropic {
    /// Model: `claude-haiku-4-5-20251001`
    Claude__Haiku__4__5__20251001,
    /// Model: `claude-opus-4-1-20250805`
    Claude__Opus__4__1__20250805,
    /// Model: `claude-opus-4-20250514`
    Claude__Opus__4__20250514,
    /// Model: `claude-opus-4-5-20251101`
    Claude__Opus__4__5__20251101,
    /// Model: `claude-opus-4-6`
    Claude__Opus__4__6,
    /// Model: `claude-opus-4-7`
    Claude__Opus__4__7,
    /// Model: `claude-sonnet-4-20250514`
    Claude__Sonnet__4__20250514,
    /// Model: `claude-sonnet-4-5-20250929`
    Claude__Sonnet__4__5__20250929,
    /// Model: `claude-sonnet-4-6`
    Claude__Sonnet__4__6,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
