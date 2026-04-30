//! Model catalog service.
//!
//! The service answers: "Is this model valid for this provider right now?"
//! It combines cached catalogs, user overrides, and dynamic sources.

use std::collections::{HashMap, HashSet};

use super::cache::{ModelCache, ModelCacheEntry};
use super::config::merge_overrides;
use super::provider_sources::{fetch_provider_catalog, static_catalog_for_provider};
use crate::config::claudine_config::{ClaudineConfig, ProviderModelOverride};
use crate::provider::Provider;

/// Unified model catalog service.
///
/// Created from user config overrides and an optional cache directory.
/// Call [`refresh`](Self::refresh) to populate/update cached catalogs
/// before validation.
#[derive(Debug, Clone)]
pub struct ModelCatalogService {
    cache: ModelCache,
    overrides: HashMap<Provider, ProviderModelOverride>,
}

impl Default for ModelCatalogService {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelCatalogService {
    /// Create a service with the default cache directory and no overrides.
    pub fn new() -> Self {
        Self {
            cache: ModelCache::new(),
            overrides: HashMap::new(),
        }
    }

    /// Create a service from config overrides.
    pub fn with_overrides(overrides: HashMap<Provider, ProviderModelOverride>) -> Self {
        Self {
            cache: ModelCache::new(),
            overrides,
        }
    }

    /// Create a service with a custom cache directory (useful for tests).
    pub fn with_cache_dir(cache_dir: std::path::PathBuf) -> Self {
        Self {
            cache: ModelCache::with_dir(cache_dir),
            overrides: HashMap::new(),
        }
    }

    /// Create a service with both custom cache and overrides.
    pub fn with_cache_and_overrides(
        cache_dir: std::path::PathBuf,
        overrides: HashMap<Provider, ProviderModelOverride>,
    ) -> Self {
        Self {
            cache: ModelCache::with_dir(cache_dir),
            overrides,
        }
    }

    /// Create a service loading overrides from the given config.
    pub fn from_config(config: &ClaudineConfig) -> Self {
        Self::with_overrides(config.models.clone())
    }

    /// Best-effort blocking refresh of all supported providers.
    ///
    /// Never panics; failures are silently ignored so that stale cache or
    /// static fallback remains available. Runs in a dedicated thread so this
    /// works even when called from within an existing Tokio runtime.
    pub fn refresh_blocking(&self) {
        let self_clone = self.clone();
        let _ = std::thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Runtime::new() else {
                return;
            };
            rt.block_on(async {
                let _ = self_clone.refresh_all().await;
            });
        })
        .join();
    }

    /// Refresh the catalog for a single provider.
    ///
    /// Attempts to fetch the latest catalog. On failure, the existing cache
    /// is left untouched (stale-cache fallback).
    pub async fn refresh(
        &self,
        provider: Provider,
    ) -> Result<Vec<String>, super::provider_sources::CatalogFetchError> {
        let fetched = fetch_provider_catalog(provider).await?;
        let entry = ModelCacheEntry {
            provider,
            models: fetched.clone(),
            fetched_at: chrono::Utc::now(),
        };
        // Best-effort write; ignore errors so cache remains optional
        let _ = self.cache.write(&entry);
        Ok(fetched)
    }

    /// Refresh all supported providers.
    pub async fn refresh_all(
        &self,
    ) -> Vec<(
        Provider,
        Result<Vec<String>, super::provider_sources::CatalogFetchError>,
    )> {
        let providers = [
            Provider::Claude,
            Provider::Codex,
            Provider::OpenCode,
            Provider::QwenCode,
        ];
        let mut results = Vec::new();
        for provider in providers {
            results.push((provider, self.refresh(provider).await));
        }
        results
    }

    /// Return the effective catalog for a provider.
    ///
    /// Merges cached data (or static source) with user overrides.
    pub fn catalog_for(&self, provider: Provider) -> Vec<String> {
        let base = match self.cache.read(provider) {
            Some(entry) => entry.models,
            None => static_catalog_for_provider(provider),
        };
        let override_entry = self.overrides.get(&provider);
        merge_overrides(provider, &base, override_entry)
    }

    /// Check whether a model ID is present in the effective catalog.
    pub fn is_valid(&self, provider: Provider, model_id: &str) -> bool {
        let catalog = self.catalog_for(provider);
        catalog.iter().any(|m| m.eq_ignore_ascii_case(model_id))
    }

    /// Return the set of model IDs for a provider.
    pub fn model_set(&self, provider: Provider) -> HashSet<String> {
        self.catalog_for(provider).into_iter().collect()
    }

    /// Return the first valid model from a list, if any.
    pub fn first_valid(&self, provider: Provider, candidates: &[String]) -> Option<String> {
        let set = self.model_set(provider);
        candidates.iter().find(|c| set.contains(*c)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::claudine_config::{
        DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
    };

    #[test]
    fn service_validates_static_model() {
        let service = ModelCatalogService::new();
        assert!(service.is_valid(Provider::Codex, "o3-mini"));
        assert!(service.is_valid(Provider::Claude, "claude-3-7-sonnet-20250219"));
    }

    #[test]
    fn service_rejects_unknown_model() {
        let service = ModelCatalogService::new();
        assert!(!service.is_valid(Provider::Codex, "not-a-real-model-xyz"));
        assert!(!service.is_valid(Provider::Claude, "not-a-real-model-xyz"));
    }

    #[test]
    fn service_case_insensitive_validation() {
        let service = ModelCatalogService::new();
        assert!(service.is_valid(Provider::Codex, "O3-MINI"));
        assert!(service.is_valid(Provider::Claude, "CLAUDE-3-7-SONNET-20250219"));
    }

    #[test]
    fn service_with_additive_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Codex,
            ProviderModelOverride::AddList(vec!["custom-codex-model".into()]),
        );
        let service = ModelCatalogService::with_overrides(overrides);

        assert!(service.is_valid(Provider::Codex, "o3-mini"));
        assert!(service.is_valid(Provider::Codex, "custom-codex-model"));
    }

    #[test]
    fn service_with_replace_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Codex,
            ProviderModelOverride::Detailed(DetailedModelOverride {
                mode: ModelOverrideMode::Replace,
                values: vec!["only-this-model".into()],
            }),
        );
        let service = ModelCatalogService::with_overrides(overrides);

        assert!(!service.is_valid(Provider::Codex, "o3-mini"));
        assert!(service.is_valid(Provider::Codex, "only-this-model"));
    }

    #[test]
    fn service_first_valid_finds_match() {
        let service = ModelCatalogService::new();
        let candidates = vec!["not-real".into(), "o3-mini".into(), "gpt-5.2".into()];
        assert_eq!(
            service.first_valid(Provider::Codex, &candidates),
            Some("o3-mini".into())
        );
    }

    #[test]
    fn service_first_valid_returns_none_when_no_match() {
        let service = ModelCatalogService::new();
        let candidates = vec!["not-real".into(), "also-fake".into()];
        assert_eq!(service.first_valid(Provider::Codex, &candidates), None);
    }

    #[test]
    fn gemini_has_no_static_catalog() {
        let service = ModelCatalogService::new();
        assert!(service.catalog_for(Provider::Gemini).is_empty());
        assert!(!service.is_valid(Provider::Gemini, "gemini-2.5-pro"));
    }

    #[test]
    fn gemini_can_have_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Gemini,
            ProviderModelOverride::AddList(vec!["gemini-2.5-pro".into()]),
        );
        let service = ModelCatalogService::with_overrides(overrides);
        assert!(service.is_valid(Provider::Gemini, "gemini-2.5-pro"));
    }

    #[test]
    fn refresh_blocking_does_not_panic() {
        let service = ModelCatalogService::new();
        service.refresh_blocking(); // should not panic even if network is down
    }
}
