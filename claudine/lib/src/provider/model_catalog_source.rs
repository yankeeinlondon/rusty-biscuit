//! Typed [`ModelCatalogSource`] descriptor for [`ProviderInfo`].
//!
//! Centralizes how each provider sources its model catalog so the
//! `model_catalog::provider_sources` module can dispatch on a typed flag
//! instead of `match Provider { ... }`.

use serde::Serialize;

/// Source of a provider's model catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCatalogSource {
    /// No catalog source available; rely entirely on user-supplied overrides.
    None,
    /// Static, compiled-in list (e.g. derived from generated enums).
    Static,
    /// Dynamic: shells out to `opencode models` for the full list.
    OpencodeCli,
    /// Dynamic: shells out to `opencode models` and filters to entries
    /// containing `qwen`.
    OpencodeCliQwenFiltered,
}
