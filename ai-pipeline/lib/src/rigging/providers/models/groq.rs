//! Auto-generated provider model enum
//!
//! Generated: 2026-01-10T02:15:39.571622+00:00
//! Generator: gen-models v0.1.0
//! Provider: Groq
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [Groq](<https://groq.com>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelGroq {
    /// Model: `allam-2-7b`
    Allam__2__7b,
    /// Model: `canopylabs/orpheus-arabic-saudi`
    Canopylabs___Orpheus__Arabic__Saudi,
    /// Model: `canopylabs/orpheus-v1-english`
    Canopylabs___Orpheus__V1__English,
    /// Model: `groq/compound`
    Groq___Compound,
    /// Model: `groq/compound-mini`
    Groq___Compound__Mini,
    /// Model: `llama-3.1-8b-instant`
    Llama__3_1__8b__Instant,
    /// Model: `llama-3.3-70b-versatile`
    Llama__3_3__70b__Versatile,
    /// Model: `meta-llama/llama-4-maverick-17b-128e-instruct`
    Meta__Llama___Llama__4__Maverick__17b__128e__Instruct,
    /// Model: `meta-llama/llama-4-scout-17b-16e-instruct`
    Meta__Llama___Llama__4__Scout__17b__16e__Instruct,
    /// Model: `meta-llama/llama-guard-4-12b`
    Meta__Llama___Llama__Guard__4__12b,
    /// Model: `meta-llama/llama-prompt-guard-2-22m`
    Meta__Llama___Llama__Prompt__Guard__2__22m,
    /// Model: `meta-llama/llama-prompt-guard-2-86m`
    Meta__Llama___Llama__Prompt__Guard__2__86m,
    /// Model: `moonshotai/kimi-k2-instruct`
    Moonshotai___Kimi__K2__Instruct,
    /// Model: `moonshotai/kimi-k2-instruct-0905`
    Moonshotai___Kimi__K2__Instruct__0905,
    /// Model: `openai/gpt-oss-120b`
    Openai___Gpt__Oss__120b,
    /// Model: `openai/gpt-oss-20b`
    Openai___Gpt__Oss__20b,
    /// Model: `openai/gpt-oss-safeguard-20b`
    Openai___Gpt__Oss__Safeguard__20b,
    /// Model: `qwen/qwen3-32b`
    Qwen___Qwen3__32b,
    /// Model: `whisper-large-v3`
    Whisper__Large__V3,
    /// Model: `whisper-large-v3-turbo`
    Whisper__Large__V3__Turbo,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
