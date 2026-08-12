//! Model catalog validation, overrides, and listing cache.
//!
//! This module provides a unified way to answer the question:
//! "Is this model valid for this provider right now?"
//!
//! Validation is two-tier ([`ModelCatalogService::is_valid`]):
//! - membership in the generated expected-offering baseline
//!   ([`expected_baseline`]: ids plus rolling aliases like `opus`),
//!   merged with user-defined overrides from config (additive or
//!   replace);
//! - offering-source namespace prefixes (local runners like `ollama/*`)
//!   whose model population cannot be enumerated statically.
//!
//! Dynamic provider listings (e.g. `opencode models`) are still fetched
//! and cached under `~/.claudine/cache/models/` — no longer as a
//! validation source but as the drift-channel input that gets diffed
//! against [`expected_ids`].
//!
//! Alongside the runtime service, [`families`] resolves `family_latest`
//! questions (rolling aliases, family keys) against the generated
//! [`families_generated`] slice of the unchained-ai models-catalog
//! artifact.

mod cache;
mod config;
mod drift;
mod families;
mod families_generated;
mod provider_sources;
mod service;

pub use cache::{ModelCache, ModelCacheEntry};
pub use config::merge_overrides;
pub use drift::{listing_drift, resolved_model_drift};
pub use families::{
    FAMILY_LATEST_MAX_AGE_DAYS, FamilyLatest, FamilyLatestStamp, FamilyRow, Staleness,
    family_latest, family_latest_at, family_latest_stamp, resolve_alias, resolve_alias_at,
};
pub use provider_sources::{expected_baseline, expected_ids, fetch_provider_catalog};
pub use service::ModelCatalogService;
