//! Central registry that resolves a [`Provider`] to its [`ProviderInfo`].
//!
//! This is the only place in the lib crate where a `match Provider` is
//! permitted post-Phase-4. All other dispatch flows through the registry's
//! field accessors and behavior trait objects.

use std::sync::OnceLock;

use crate::provider_id::{PROVIDER_COUNT, PROVIDERS_DISPLAY_ORDER, Provider};

use super::ProviderInfo;
use super::antigravity::ANTIGRAVITY_INFO;
use super::claude::CLAUDE_INFO;
use super::codex::CODEX_INFO;
use super::gemini::GEMINI_INFO;
use super::goose::GOOSE_INFO;
use super::kilo::KILO_INFO;
use super::kimi::KIMI_INFO;
use super::opencode::OPENCODE_INFO;
use super::pi::PI_INFO;
use super::qwen::QWEN_INFO;

/// The canonical provider registry, initialized on first access.
pub(crate) static REGISTRY: OnceLock<[&'static ProviderInfo; PROVIDER_COUNT]> = OnceLock::new();

/// Returns the [`ProviderInfo`] for the given [`Provider`].
///
/// The mapping is total (every variant has a registered entry) and the
/// returned reference is `'static` since each `ProviderInfo` lives in the
/// binary's read-only data segment.
pub fn provider_info(provider: Provider) -> &'static ProviderInfo {
    let registry = REGISTRY.get_or_init(|| {
        [
            &CLAUDE_INFO,
            &CODEX_INFO,
            &GEMINI_INFO,
            &GOOSE_INFO,
            &KIMI_INFO,
            &OPENCODE_INFO,
            &QWEN_INFO,
            &KILO_INFO,
            &PI_INFO,
            &ANTIGRAVITY_INFO,
        ]
    });
    registry[provider as usize]
}

/// Returns every [`ProviderInfo`] in canonical display order.
pub fn all_providers() -> impl Iterator<Item = &'static ProviderInfo> {
    PROVIDERS_DISPLAY_ORDER.into_iter().map(provider_info)
}
