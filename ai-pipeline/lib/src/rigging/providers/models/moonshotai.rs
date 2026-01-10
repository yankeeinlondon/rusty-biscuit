//! Auto-generated provider model enum
//!
//! Generated: 2026-01-10T01:34:01.196184+00:00
//! Generator: gen-models v0.1.0
//! Provider: MoonshotAi
//!
//! Do not edit manually.

use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelMoonshotAi {
    /// Model: `kimi-k2-0711-preview`
    Kimi__K2__0711__Preview,
    /// Model: `kimi-k2-0905-preview`
    Kimi__K2__0905__Preview,
    /// Model: `kimi-k2-thinking`
    Kimi__K2__Thinking,
    /// Model: `kimi-k2-thinking-turbo`
    Kimi__K2__Thinking__Turbo,
    /// Model: `kimi-k2-turbo-preview`
    Kimi__K2__Turbo__Preview,
    /// Model: `kimi-latest`
    Kimi__Latest,
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
