//! Auto-generated provider model enum
//!
//! Generated: 2026-07-06T22:44:33.473171+00:00
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
    /// Model: `claude-fable-5`
    Claude__Fable__5,
    /// Model: `claude-haiku-4-5-20251001`
    Claude__Haiku__4__5__20251001,
    /// Model: `claude-opus-4-1-20250805`
    Claude__Opus__4__1__20250805,
    /// Model: `claude-opus-4-5-20251101`
    Claude__Opus__4__5__20251101,
    /// Model: `claude-opus-4-6`
    Claude__Opus__4__6,
    /// Model: `claude-opus-4-7`
    Claude__Opus__4__7,
    /// Model: `claude-opus-4-8`
    Claude__Opus__4__8,
    /// Model: `claude-sonnet-4-5-20250929`
    Claude__Sonnet__4__5__20250929,
    /// Model: `claude-sonnet-4-6`
    Claude__Sonnet__4__6,
    /// Model: `claude-sonnet-5`
    Claude__Sonnet__5,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
