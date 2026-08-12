//! Canonical provider identifier and closely-coupled leaf types.
//!
//! This module is a **deliberate leaf**: it sits below [`crate::provider`] in the
//! dependency graph so that [`crate::stream`] (and any other module) can import
//! [`Provider`] without creating a cycle back into the provider catalog.
//!
//! Types that live here:
//! - [`Provider`] enum and its ordering constants.
//! - [`OutputFormatSelector`] — a small metadata enum used by both the provider
//!   catalog and the CLI wrapper.
//!
//! Everything else (behaviour traits, [`ProviderInfo`](crate::provider::ProviderInfo),
//! registry helpers) stays in [`crate::provider`].

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
    /// Kilo Code CLI (Kilo) — an OpenCode-fork agentic coding CLI.
    Kilo = 7,
    /// Pi (earendil-works) — a bespoke multi-provider agentic coding CLI.
    Pi = 8,
    /// Antigravity (Google) — the headless `agy` coding CLI.
    Antigravity = 9,
}

/// Total number of [`Provider`] variants.
///
/// Anchors compile-time array length checks for the registry and any other
/// `[T; PROVIDER_COUNT]` table indexed by `Provider as usize`.
pub const PROVIDER_COUNT: usize = 10;

/// Providers in canonical display order for matrix-style reporting.
pub const PROVIDERS_DISPLAY_ORDER: [Provider; PROVIDER_COUNT] = [
    Provider::Claude,
    Provider::Codex,
    Provider::Gemini,
    Provider::Goose,
    Provider::KimiCode,
    Provider::OpenCode,
    Provider::QwenCode,
    Provider::Kilo,
    Provider::Pi,
    Provider::Antigravity,
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
    let _kilo: usize = Provider::Kilo as usize;
    let _pi: usize = Provider::Pi as usize;
    let _antigravity: usize = Provider::Antigravity as usize;
    assert!(_claude == 0);
    assert!(_codex == 1);
    assert!(_gemini == 2);
    assert!(_goose == 3);
    assert!(_kimi == 4);
    assert!(_open == 5);
    assert!(_qwen == 6);
    assert!(_kilo == 7);
    assert!(_pi == 8);
};

/// Typed selector metadata for provider-native output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OutputFormatSelector {
    /// The provider default; no flag or positional token is needed.
    Default,
    /// A flag-only selector such as Codex `--json`.
    Flag {
        /// Native CLI flag.
        flag: &'static str,
    },
    /// A flag plus value selector such as `--output-format json`.
    ///
    /// Detection accepts both `--flag value` and `--flag=value`.
    FlagValue {
        /// Native CLI flag.
        flag: &'static str,
    },
    /// A positional token selector.
    Positional {
        /// Native positional token.
        token: &'static str,
    },
    /// A transport selector used by Claudine to get structured events, not a
    /// user-requested provider output format. Kimi `--wire` is the current
    /// example: it enables the JSON-RPC transport and must not disable
    /// Claudine's internal structured parsing by itself.
    TransportFlag {
        /// Native CLI flag.
        flag: &'static str,
    },
}
