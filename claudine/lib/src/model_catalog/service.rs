//! Model catalog service.
//!
//! The service answers: "Is this model valid for this provider right now?"
//! It combines cached catalogs, user overrides, and dynamic sources.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::cache::{ModelCache, ModelCacheEntry};
use super::config::merge_overrides;
use super::provider_sources::{
    CatalogFetchError, fetch_opencode_models, fetch_provider_catalog, static_catalog_for_provider,
};
use crate::config::claudine_config::{ClaudineConfig, ProviderModelOverride};
use crate::provider::{ModelCatalogSource, Provider, provider_info};

/// Memoized outcome of an `opencode models` subprocess attempt.
///
/// `None` means the dedup slot is empty; `Some(Ok(_))` means a previous
/// attempt succeeded; `Some(Err(_))` means a previous attempt failed and
/// must not be retried within this scope.
type OpencodeDedupSlot = Arc<Mutex<Option<Result<Vec<String>, CatalogFetchError>>>>;

/// Unified model catalog service.
///
/// Created from user config overrides and an optional cache directory.
/// Call [`refresh`](Self::refresh) to populate/update cached catalogs
/// before validation.
#[derive(Debug, Clone)]
pub struct ModelCatalogService {
    cache: ModelCache,
    overrides: HashMap<Provider, ProviderModelOverride>,
    /// In-memory dedup cache for the OpenCode dynamic source.
    ///
    /// Populated the first time `fetch_opencode_models()` runs for this
    /// service instance and reused for any later `OpenCode` or `QwenCode`
    /// refresh in the same scope. Cloning the service shares this cache
    /// because [`Arc`] is reference-counted.
    opencode_dedup: OpencodeDedupSlot,
    /// Number of actual `opencode models` subprocess attempts performed
    /// by this service instance. Used by tests to verify dedup behavior.
    opencode_fetch_attempts: Arc<AtomicUsize>,
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
            opencode_dedup: Arc::new(Mutex::new(None)),
            opencode_fetch_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a service from config overrides.
    pub fn with_overrides(overrides: HashMap<Provider, ProviderModelOverride>) -> Self {
        Self {
            cache: ModelCache::new(),
            overrides,
            opencode_dedup: Arc::new(Mutex::new(None)),
            opencode_fetch_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a service with a custom cache directory (useful for tests).
    pub fn with_cache_dir(cache_dir: std::path::PathBuf) -> Self {
        Self {
            cache: ModelCache::with_dir(cache_dir),
            overrides: HashMap::new(),
            opencode_dedup: Arc::new(Mutex::new(None)),
            opencode_fetch_attempts: Arc::new(AtomicUsize::new(0)),
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
            opencode_dedup: Arc::new(Mutex::new(None)),
            opencode_fetch_attempts: Arc::new(AtomicUsize::new(0)),
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

    /// Best-effort blocking refresh for a single provider.
    ///
    /// Performs at most one `opencode models` subprocess per service
    /// instance: a `QwenCode` refresh that follows an `OpenCode` refresh
    /// reuses the cached OpenCode result. Static-source providers
    /// (Claude, Codex) write the static list to cache without spawning
    /// any subprocess. Providers without a source (Gemini, Goose, Kimi,
    /// Roo) are no-ops.
    ///
    /// Never panics; failures are silently ignored so stale cache or
    /// static fallback remains available.
    pub fn refresh_provider_blocking(&self, provider: Provider) {
        let self_clone = self.clone();
        let _ = std::thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Runtime::new() else {
                return;
            };
            rt.block_on(async {
                let _ = self_clone.refresh_provider(provider).await;
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
    ) -> Result<Vec<String>, CatalogFetchError> {
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

    /// Refresh the catalog for a single provider with in-process dedup.
    ///
    /// This is the async counterpart of [`refresh_provider_blocking`]. It
    /// shares the OpenCode subprocess result between `OpenCode` and
    /// `QwenCode` refreshes so a `--qwen` run never repeats the
    /// `opencode models` call.
    pub async fn refresh_provider(
        &self,
        provider: Provider,
    ) -> Result<Vec<String>, CatalogFetchError> {
        let fetched = match provider_info(provider).dynamic_source {
            ModelCatalogSource::None => Vec::new(),
            ModelCatalogSource::Static => static_catalog_for_provider(provider),
            ModelCatalogSource::OpencodeCli => self.fetch_opencode_with_dedup().await?,
            ModelCatalogSource::OpencodeCliQwenFiltered => self
                .fetch_opencode_with_dedup()
                .await?
                .into_iter()
                .filter(|m| m.to_ascii_lowercase().contains("qwen"))
                .collect(),
        };
        let entry = ModelCacheEntry {
            provider,
            models: fetched.clone(),
            fetched_at: chrono::Utc::now(),
        };
        let _ = self.cache.write(&entry);
        Ok(fetched)
    }

    /// Fetch the OpenCode model catalog, reusing an in-memory result
    /// captured earlier in the same service-instance scope.
    ///
    /// Both successful and failed fetch outcomes are memoized so that a
    /// transient error does not get retried mid-prep.
    async fn fetch_opencode_with_dedup(&self) -> Result<Vec<String>, CatalogFetchError> {
        if let Some(ref cached) = *self.opencode_dedup.lock().expect("opencode_dedup poisoned") {
            return cached.clone();
        }
        self.opencode_fetch_attempts.fetch_add(1, Ordering::SeqCst);
        let result = fetch_opencode_models().await;
        let mut slot = self.opencode_dedup.lock().expect("opencode_dedup poisoned");
        if slot.is_none() {
            *slot = Some(result.clone());
        }
        result
    }

    /// Number of actual `opencode models` subprocess attempts performed
    /// by this service instance.
    ///
    /// Exposed for tests that need to verify the dedup contract.
    #[doc(hidden)]
    pub fn opencode_fetch_attempts(&self) -> usize {
        self.opencode_fetch_attempts.load(Ordering::SeqCst)
    }

    /// Pre-populate the in-memory OpenCode dedup cache with a known
    /// result. Used by tests to exercise [`refresh_provider`] without
    /// shelling out.
    #[doc(hidden)]
    pub fn prime_opencode_dedup(&self, result: Result<Vec<String>, CatalogFetchError>) {
        *self.opencode_dedup.lock().expect("opencode_dedup poisoned") = Some(result);
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

    #[test]
    fn refresh_provider_blocking_static_no_subprocess() {
        // Static-source providers (Claude, Codex) must never spawn the
        // opencode subprocess.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.refresh_provider_blocking(Provider::Claude);
        service.refresh_provider_blocking(Provider::Codex);
        assert_eq!(service.opencode_fetch_attempts(), 0);
    }

    #[test]
    fn refresh_provider_blocking_no_source_no_subprocess() {
        // Providers without a dynamic source (Gemini, Goose, Kimi, Roo)
        // must never spawn the opencode subprocess.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.refresh_provider_blocking(Provider::Gemini);
        service.refresh_provider_blocking(Provider::Goose);
        service.refresh_provider_blocking(Provider::KimiCode);
        service.refresh_provider_blocking(Provider::RooCode);
        assert_eq!(service.opencode_fetch_attempts(), 0);
    }

    #[test]
    fn refresh_provider_blocking_qwen_dedupes_opencode_via_primed_cache() {
        // Pre-populate the in-memory dedup cache so the QwenCode refresh
        // never reaches the subprocess. Verifies that the fallback path
        // honors the cache.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.prime_opencode_dedup(Ok(vec![
            "qwen-2.5-coder".into(),
            "gpt-5".into(),
            "claude-sonnet-4".into(),
        ]));

        service.refresh_provider_blocking(Provider::QwenCode);
        assert_eq!(service.opencode_fetch_attempts(), 0);

        let qwen = service.catalog_for(Provider::QwenCode);
        assert!(qwen.contains(&"qwen-2.5-coder".into()));
        assert!(!qwen.contains(&"gpt-5".into()));
    }

    #[test]
    fn refresh_provider_blocking_opencode_then_qwen_dedupes() {
        // OpenCode refresh primes the dedup cache; QwenCode refresh that
        // follows must not re-attempt the subprocess. We seed the cache
        // up front to avoid relying on `opencode` being on PATH.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.prime_opencode_dedup(Ok(vec![
            "qwen-coder".into(),
            "gpt-5.2".into(),
        ]));
        service.refresh_provider_blocking(Provider::OpenCode);
        service.refresh_provider_blocking(Provider::QwenCode);
        assert_eq!(
            service.opencode_fetch_attempts(),
            0,
            "primed dedup must short-circuit subprocess attempts"
        );
    }

    #[test]
    fn refresh_provider_blocking_failure_falls_back_to_static() {
        // Even if the dynamic source has been primed with a failure,
        // catalog_for() must still return the static catalog when one
        // exists. Refresh failures must not corrupt later validation.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.prime_opencode_dedup(Err(CatalogFetchError::CliNotFound("opencode".into())));
        service.refresh_provider_blocking(Provider::OpenCode);

        // OpenCode has no static catalog, but is_valid for Claude (a
        // static-source provider) must still work because refreshing
        // OpenCode never touches Claude state.
        assert!(service.is_valid(Provider::Claude, "claude-3-7-sonnet-20250219"));
    }
}
