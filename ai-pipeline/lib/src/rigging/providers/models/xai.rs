//! Auto-generated provider model enum
//!
//! Generated: 2026-01-10T02:01:42.489978+00:00
//! Generator: gen-models v0.1.0
//! Provider: Xai
//!
//! Do not edit manually.

use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelXai {
    /// Model: `grok-2-image-1212`
    Grok__2__Image__1212,
    /// Model: `grok-2-vision-1212`
    Grok__2__Vision__1212,
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
    /// Model: `grok-code-fast-1`
    Grok__Code__Fast__1,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
