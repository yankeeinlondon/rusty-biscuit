//! Typed [`ModelCatalogSource`] descriptor for the provider catalog.
//!
//! Centralizes how each provider sources its model catalog so the
//! `model_catalog::provider_sources` module can dispatch on a typed flag
//! instead of `match Provider { ... }`.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Source of a provider's model catalog.
///
/// The strum introspection derives back the generator's schema↔catalog
/// compatibility gate: research sidecars that feed this enum must declare
/// `enum(...)` members that are a subset of [`ModelCatalogSource::VARIANTS`]
/// (snake_case, matching the serde wire form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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
