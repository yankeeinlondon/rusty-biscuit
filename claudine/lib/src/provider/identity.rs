//! Canonical [`Provider`] enum and ordering constants.
//!
//! `Provider` was previously defined in [`crate::events::provider`] (now a
//! deprecated re-export). The canonical home is now `claudine::provider`.

use serde::{Deserialize, Serialize};

/// Supported agentic CLI providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Provider {
    /// Claude Code (Anthropic).
    Claude,
    /// Codex CLI (OpenAI).
    Codex,
    /// Gemini CLI (Google).
    Gemini,
    /// Goose (Block).
    Goose,
    /// Kimi Code CLI (Moonshot AI).
    KimiCode,
    /// OpenCode.
    OpenCode,
    /// Qwen Code CLI (Alibaba).
    QwenCode,
    /// Roo Code.
    RooCode,
}

/// Providers in canonical display order for matrix-style reporting.
pub const PROVIDERS_DISPLAY_ORDER: [Provider; 8] = [
    Provider::Claude,
    Provider::Codex,
    Provider::Gemini,
    Provider::Goose,
    Provider::KimiCode,
    Provider::OpenCode,
    Provider::QwenCode,
    Provider::RooCode,
];
