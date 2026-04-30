//! Canonical [`Provider`] enum and ordering constants.
//!
//! `Provider` was previously defined in [`crate::events::provider`] (now a
//! deprecated re-export). The canonical home is now `claudine::provider`.
//!
//! This module also owns the array-backed ordering used by
//! [`crate::provider::registry`], and the [`PROVIDER_COUNT`] constant that
//! pins the variant count at compile time.

use serde::{Deserialize, Serialize};

/// Supported agentic CLI providers.
///
/// `#[repr(usize)]` lets the registry index a `[&'static ProviderInfo; N]`
/// array by casting `Provider as usize`, eliminating the central
/// `match provider { ... }` dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
#[repr(usize)]
pub enum Provider {
    /// Claude Code (Anthropic).
    Claude = 0,
    /// Codex CLI (OpenAI).
    Codex = 1,
    /// Gemini CLI (Google).
    Gemini = 2,
    /// Goose (Block).
    Goose = 3,
    /// Kimi Code CLI (Moonshot AI).
    KimiCode = 4,
    /// OpenCode.
    OpenCode = 5,
    /// Qwen Code CLI (Alibaba).
    QwenCode = 6,
    /// Roo Code.
    RooCode = 7,
}

/// Total number of [`Provider`] variants.
///
/// Anchors compile-time array length checks for the registry and any other
/// `[T; PROVIDER_COUNT]` table indexed by `Provider as usize`.
pub const PROVIDER_COUNT: usize = 8;

/// Providers in canonical display order for matrix-style reporting.
pub const PROVIDERS_DISPLAY_ORDER: [Provider; PROVIDER_COUNT] = [
    Provider::Claude,
    Provider::Codex,
    Provider::Gemini,
    Provider::Goose,
    Provider::KimiCode,
    Provider::OpenCode,
    Provider::QwenCode,
    Provider::RooCode,
];

// Compile-time assertion: every `Provider` discriminant fits in
// `0..PROVIDER_COUNT` and `PROVIDERS_DISPLAY_ORDER` is exhaustive.
const _: () = {
    // If `PROVIDER_COUNT` ever drifts from `PROVIDERS_DISPLAY_ORDER.len()`,
    // this array sizing fails at compile time.
    let _check: [(); PROVIDER_COUNT] = [(); PROVIDERS_DISPLAY_ORDER.len()];

    // Verify each variant maps to its expected discriminant index.
    let _claude: usize = Provider::Claude as usize;
    let _codex: usize = Provider::Codex as usize;
    let _gemini: usize = Provider::Gemini as usize;
    let _goose: usize = Provider::Goose as usize;
    let _kimi: usize = Provider::KimiCode as usize;
    let _open: usize = Provider::OpenCode as usize;
    let _qwen: usize = Provider::QwenCode as usize;
    let _roo: usize = Provider::RooCode as usize;
    assert!(_claude == 0);
    assert!(_codex == 1);
    assert!(_gemini == 2);
    assert!(_goose == 3);
    assert!(_kimi == 4);
    assert!(_open == 5);
    assert!(_qwen == 6);
    assert!(_roo == 7);
};
