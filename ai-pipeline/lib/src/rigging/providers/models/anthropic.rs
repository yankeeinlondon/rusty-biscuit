//! Auto-generated provider model enum
//!
//! Generated: 2026-01-10T02:01:40.574369+00:00
//! Generator: gen-models v0.1.0
//! Provider: Anthropic
//!
//! Do not edit manually.

use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelAnthropic {
    /// Model: `claude-3-5-haiku-20241022`
    Claude__3__5__Haiku__20241022,
    /// Model: `claude-3-7-sonnet-20250219`
    Claude__3__7__Sonnet__20250219,
    /// Model: `claude-3-haiku-20240307`
    Claude__3__Haiku__20240307,
    /// Model: `claude-haiku-4-5-20251001`
    Claude__Haiku__4__5__20251001,
    /// Model: `claude-opus-4-1-20250805`
    Claude__Opus__4__1__20250805,
    /// Model: `claude-opus-4-20250514`
    Claude__Opus__4__20250514,
    /// Model: `claude-opus-4-5-20251101`
    Claude__Opus__4__5__20251101,
    /// Model: `claude-sonnet-4-20250514`
    Claude__Sonnet__4__20250514,
    /// Model: `claude-sonnet-4-5-20250929`
    Claude__Sonnet__4__5__20250929,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
