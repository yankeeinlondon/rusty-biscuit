//! Central registry that resolves a [`Provider`] to its [`ProviderInfo`].
//!
//! This is the only place in the lib crate where a `match Provider` is
//! permitted post-Phase-4. All other dispatch flows through the registry's
//! field accessors and behavior trait objects.

use super::claude::CLAUDE_INFO;
use super::codex::CODEX_INFO;
use super::gemini::GEMINI_INFO;
use super::goose::GOOSE_INFO;
use super::identity::{Provider, PROVIDERS_DISPLAY_ORDER};
use super::kimi::KIMI_INFO;
use super::opencode::OPENCODE_INFO;
use super::qwen::QWEN_INFO;
use super::roo::ROO_INFO;
use super::ProviderInfo;

/// Returns the [`ProviderInfo`] for the given [`Provider`].
///
/// The mapping is total (every variant has a registered entry) and the
/// returned reference is `'static` since each `ProviderInfo` lives in the
/// binary's read-only data segment.
pub fn provider_info(provider: Provider) -> &'static ProviderInfo {
    match provider {
        Provider::Claude => &CLAUDE_INFO,
        Provider::Codex => &CODEX_INFO,
        Provider::Gemini => &GEMINI_INFO,
        Provider::Goose => &GOOSE_INFO,
        Provider::KimiCode => &KIMI_INFO,
        Provider::OpenCode => &OPENCODE_INFO,
        Provider::QwenCode => &QWEN_INFO,
        Provider::RooCode => &ROO_INFO,
    }
}

/// Returns every [`ProviderInfo`] in canonical display order.
pub fn all_providers() -> impl Iterator<Item = &'static ProviderInfo> {
    PROVIDERS_DISPLAY_ORDER.into_iter().map(provider_info)
}
