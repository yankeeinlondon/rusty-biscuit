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

mod cache;
mod config;
mod provider_sources;
mod service;

pub use cache::{ModelCache, ModelCacheEntry};
pub use config::merge_overrides;
pub use provider_sources::{fetch_provider_catalog, static_catalog_for_provider};
pub use service::ModelCatalogService;
