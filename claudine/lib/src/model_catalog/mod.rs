//! Model catalog cache, validation, and override service.
//!
//! This module provides a unified way to answer the question:
//! "Is this model valid for this provider right now?"
//!
//! It combines:
//! - Cached provider model lists (under `~/.claudine/cache/models/`)
//! - User-defined model overrides from config (additive or replace)
//! - Dynamic sourcing for providers that support it (OpenCode, Qwen)
//! - Static sourcing for providers with known model enums (Claude, Codex)
//!
//! When a catalog is unavailable for a provider, frontmatter `model` hints
//! are gracefully skipped rather than treated as errors.
//!
//! Alongside the runtime service, [`families`] resolves `family_latest`
//! questions (rolling aliases, family keys) against the generated
//! [`families_generated`] slice of the unchained-ai models-catalog
//! artifact.

mod cache;
mod config;
mod families;
mod families_generated;
mod provider_sources;
mod service;

pub use cache::{ModelCache, ModelCacheEntry};
pub use config::merge_overrides;
pub use families::{
    FAMILY_LATEST_MAX_AGE_DAYS, FamilyLatest, FamilyRow, Staleness, family_latest,
    family_latest_at, resolve_alias, resolve_alias_at,
};
pub use provider_sources::{fetch_provider_catalog, static_catalog_for_provider};
pub use service::ModelCatalogService;
