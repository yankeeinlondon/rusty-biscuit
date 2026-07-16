// Provider-specific policy backends.
//
// Each module implements `ProviderPolicyBackend` for one provider.
// Backends are registered with `PolicyEngine` during construction.

pub(crate) mod common;

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod goose;
pub mod kimi;
pub mod opencode;
pub mod qwen;

pub(crate) use claude::ClaudePolicyBackend;
pub(crate) use codex::CodexPolicyBackend;
pub(crate) use gemini::GeminiPolicyBackend;
pub(crate) use goose::GoosePolicyBackend;
pub(crate) use kimi::KimiPolicyBackend;
pub(crate) use opencode::OpenCodePolicyBackend;
pub(crate) use qwen::QwenPolicyBackend;
