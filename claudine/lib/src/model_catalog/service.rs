//! Model catalog service.
//!
//! The service answers: "Is this model valid for this provider right now?"
//! Validation is baseline-fed: the generated expected-offering records
//! (ids plus aliases) merged with user overrides, plus offering-source
//! namespace prefixes for local runners. Dynamic listings are still
//! fetched and cached, but only as drift-channel input — they never feed
//! validation.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::OnceCell;

use super::cache::{ModelCache, ModelCacheEntry};
use super::config::merge_overrides;
use super::provider_sources::{
    CatalogFetchError, expected_baseline, fetch_provider_catalog, fetch_shell_command_models,
};
use crate::config::claudine_config::{ClaudineConfig, ProviderModelOverride};
use crate::provider::{ModelCatalogSource, Provider, provider_info};

/// A `ShellCommand` catalog source's identity: `(program, args)`.
type ShellCommandKey = (&'static str, &'static [&'static str]);

/// Memoized outcome of one shell-command subprocess attempt.
///
/// Using [`tokio::sync::OnceCell`] guarantees the initialization closure
/// runs **at most once** even with multiple concurrent callers; later
/// callers wait for the in-flight initialization to complete and then
/// observe the cached result. Both successful and failed outcomes are
/// memoized so transient errors are not retried within this scope.
type ShellDedupSlot = Arc<OnceCell<Result<Vec<String>, CatalogFetchError>>>;

/// Per-command dedup slots, keyed so distinct `ShellCommand` sources never
/// share a memoized result.
type ShellDedupMap = Arc<std::sync::Mutex<HashMap<ShellCommandKey, ShellDedupSlot>>>;

/// Pluggable async fetcher used by [`ModelCatalogService`] for
/// `ShellCommand` dynamic sources.
///
/// Production code wires this to [`fetch_shell_command_models`]; tests can
/// inject a fake to avoid spawning real subprocesses while still
/// exercising the dedup contract under concurrency.
type ShellCommandFetcher = Arc<
    dyn Fn(
            &'static str,
            &'static [&'static str],
        ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, CatalogFetchError>> + Send>>
        + Send
        + Sync,
>;

fn default_shell_command_fetcher() -> ShellCommandFetcher {
    Arc::new(|program, args| Box::pin(fetch_shell_command_models(program, args)))
}

/// Unified model catalog service.
///
/// Created from user config overrides and an optional cache directory.
/// Validation ([`is_valid`](Self::is_valid)) works immediately from the
/// compiled expected-offering baseline; refreshing only maintains the
/// on-disk listing cache that feeds the drift channel.
#[derive(Clone)]
pub struct ModelCatalogService {
    cache: ModelCache,
    overrides: HashMap<Provider, ProviderModelOverride>,
    /// In-memory dedup cache for `ShellCommand` dynamic sources, keyed
    /// by `(program, args)`.
    ///
    /// A slot is populated the first time its command's fetcher runs for
    /// this service instance and reused by any later refresh of the same
    /// command in the same scope. Cloning the service shares this cache
    /// because [`Arc`] is reference-counted, and the underlying
    /// [`tokio::sync::OnceCell`] coordinates concurrent initialization
    /// so the fetcher runs exactly once per command even when refreshes
    /// race.
    shell_dedup: ShellDedupMap,
    /// Function used to fetch a shell-command model list. Defaults to
    /// [`fetch_shell_command_models`]; tests can substitute a fake via
    /// [`Self::set_shell_command_fetcher`].
    shell_command_fetcher: ShellCommandFetcher,
    /// Number of times a shell-command fetcher initialization closure has
    /// actually executed for this service instance. Increments inside
    /// the [`OnceCell`] init closure so it accurately reflects real
    /// dedup behavior. Used by tests.
    shell_command_fetch_attempts: Arc<AtomicUsize>,
}

impl std::fmt::Debug for ModelCatalogService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelCatalogService")
            .field("cache", &self.cache)
            .field("overrides", &self.overrides)
            .field(
                "shell_dedup_commands",
                &self.shell_dedup.lock().map(|map| map.len()).unwrap_or(0),
            )
            .field(
                "shell_command_fetch_attempts",
                &self.shell_command_fetch_attempts.load(Ordering::SeqCst),
            )
            .finish()
    }
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
            shell_dedup: ShellDedupMap::default(),
            shell_command_fetcher: default_shell_command_fetcher(),
            shell_command_fetch_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a service from config overrides.
    pub fn with_overrides(overrides: HashMap<Provider, ProviderModelOverride>) -> Self {
        Self {
            cache: ModelCache::new(),
            overrides,
            shell_dedup: ShellDedupMap::default(),
            shell_command_fetcher: default_shell_command_fetcher(),
            shell_command_fetch_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a service with a custom cache directory (useful for tests).
    pub fn with_cache_dir(cache_dir: std::path::PathBuf) -> Self {
        Self {
            cache: ModelCache::with_dir(cache_dir),
            overrides: HashMap::new(),
            shell_dedup: ShellDedupMap::default(),
            shell_command_fetcher: default_shell_command_fetcher(),
            shell_command_fetch_attempts: Arc::new(AtomicUsize::new(0)),
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
            shell_dedup: ShellDedupMap::default(),
            shell_command_fetcher: default_shell_command_fetcher(),
            shell_command_fetch_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a service loading overrides from the given config.
    pub fn from_config(config: &ClaudineConfig) -> Self {
        Self::with_overrides(config.models.clone())
    }

    /// Best-effort blocking refresh of all supported providers.
    ///
    /// Never panics; failures are silently ignored — validation is
    /// baseline-fed, so a failed listing refresh only means a staler
    /// drift-channel cache. Runs in a dedicated thread so this works
    /// even when called from within an existing Tokio runtime.
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
    /// Performs at most one subprocess per `ShellCommand` source per
    /// service instance: a repeat refresh of the same command reuses the
    /// memoized result. No-source providers write an empty listing to
    /// cache without spawning any subprocess.
    ///
    /// Never panics; failures are silently ignored — validation is
    /// baseline-fed, so a failed refresh only leaves the drift-channel
    /// listing cache stale.
    ///
    /// Fast-paths a no-op when the process-scoped user-interrupt flag has
    /// already been raised, so SIGINT during the prep window does not
    /// trigger any further dynamic-source subprocess spawns.
    pub fn refresh_provider_blocking(&self, provider: Provider) {
        if crate::interrupt::interrupted() {
            return;
        }
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

    /// Background refresh for a single provider (W3).
    ///
    /// Returns immediately. The refresh runs in a detached worker thread
    /// and updates the on-disk listing cache for later invocations.
    /// Validation never reads that cache ([`catalog_for`](Self::catalog_for)
    /// is baseline-fed), so there is no cold-start reason to block on the
    /// subprocess: a missing first-run cache just means the drift
    /// comparison waits one invocation.
    ///
    /// ## No-source providers
    ///
    /// No-source providers are essentially free to refresh, so this
    /// delegates to [`refresh_provider_blocking`](Self::refresh_provider_blocking),
    /// which writes the empty listing to cache without spawning any
    /// subprocess.
    ///
    /// ## Escape hatch
    ///
    /// Setting `CLAUDINE_BACKGROUND_REFRESH=0` forces the caller-blocking
    /// path for users who explicitly want the legacy behaviour.
    pub fn refresh_provider_async(&self, provider: Provider) {
        if crate::interrupt::interrupted() {
            return;
        }
        if std::env::var("CLAUDINE_BACKGROUND_REFRESH").as_deref() == Ok("0") {
            self.refresh_provider_blocking(provider);
            return;
        }

        let info = provider_info(provider);
        match info.model_catalog_source {
            ModelCatalogSource::None => {
                // Cheap: no subprocess spawn. Run inline so the listing
                // cache is written before we return.
                self.refresh_provider_blocking(provider);
                return;
            }
            ModelCatalogSource::ShellCommand { .. } => {}
        }

        // Detached background refresh. A later invocation reads the
        // refreshed cache; the current one never needs it.
        // Cache write uses an atomic temp+rename so a process exiting
        // mid-refresh leaves the previous cache intact.
        let self_clone = self.clone();
        std::thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Runtime::new() else {
                return;
            };
            rt.block_on(async {
                let _ = self_clone.refresh_provider(provider).await;
            });
        });
    }

    /// Refresh the listing cache for a single provider.
    ///
    /// Attempts to fetch the latest listing. On failure, the existing
    /// cache entry is left untouched.
    pub async fn refresh(&self, provider: Provider) -> Result<Vec<String>, CatalogFetchError> {
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

    /// Refresh the listing cache for a single provider with in-process
    /// dedup.
    ///
    /// This is the async counterpart of [`refresh_provider_blocking`](Self::refresh_provider_blocking). A
    /// `ShellCommand` source's subprocess result is memoized per command,
    /// so repeat refreshes within one service scope never repeat the
    /// spawn.
    pub async fn refresh_provider(
        &self,
        provider: Provider,
    ) -> Result<Vec<String>, CatalogFetchError> {
        let fetched = match provider_info(provider).model_catalog_source {
            ModelCatalogSource::None => Vec::new(),
            ModelCatalogSource::ShellCommand { program, args } => {
                self.fetch_shell_command_with_dedup(program, args).await?
            }
        };
        let entry = ModelCacheEntry {
            provider,
            models: fetched.clone(),
            fetched_at: chrono::Utc::now(),
        };
        let _ = self.cache.write(&entry);
        Ok(fetched)
    }

    /// Fetch one shell-command catalog, reusing an in-memory result
    /// captured earlier in the same service-instance scope for the same
    /// `(program, args)` command.
    ///
    /// Concurrency-safe: backed by [`tokio::sync::OnceCell`] so the
    /// fetcher closure runs at most once per command even when refreshes
    /// race. Both successful and failed outcomes are memoized so
    /// transient errors are not retried mid-prep.
    async fn fetch_shell_command_with_dedup(
        &self,
        program: &'static str,
        args: &'static [&'static str],
    ) -> Result<Vec<String>, CatalogFetchError> {
        let slot = {
            let mut map = self.shell_dedup.lock().expect("shell dedup mutex poisoned");
            map.entry((program, args)).or_default().clone()
        };
        let fetcher = self.shell_command_fetcher.clone();
        let attempts = self.shell_command_fetch_attempts.clone();
        slot.get_or_init(|| async move {
            attempts.fetch_add(1, Ordering::SeqCst);
            fetcher(program, args).await
        })
        .await
        .clone()
    }

    /// Number of times a shell-command fetcher initialization closure has
    /// actually run for this service instance.
    ///
    /// Exposed for tests that need to verify the dedup contract.
    #[doc(hidden)]
    pub fn shell_command_fetch_attempts(&self) -> usize {
        self.shell_command_fetch_attempts.load(Ordering::SeqCst)
    }

    /// Pre-populate the in-memory dedup cache for one shell command with
    /// a known result. Used by tests to exercise [`refresh_provider`]
    /// without shelling out.
    ///
    /// Idempotent: if the command's dedup cell is already initialized,
    /// the new value is silently ignored.
    #[doc(hidden)]
    pub fn prime_shell_command_dedup(
        &self,
        program: &'static str,
        args: &'static [&'static str],
        result: Result<Vec<String>, CatalogFetchError>,
    ) {
        let slot = {
            let mut map = self.shell_dedup.lock().expect("shell dedup mutex poisoned");
            map.entry((program, args)).or_default().clone()
        };
        let _ = slot.set(result);
    }

    /// Replace the shell-command fetcher with a custom async closure.
    ///
    /// Test-only helper that lets the dedup contract be exercised
    /// against an injectable fake source rather than a real subprocess.
    #[doc(hidden)]
    pub fn set_shell_command_fetcher(&mut self, fetcher: ShellCommandFetcher) {
        self.shell_command_fetcher = fetcher;
    }

    /// Refresh all supported providers.
    ///
    /// Uses [`refresh_provider`](Self::refresh_provider) internally so
    /// `ShellCommand` subprocess results are memoized per command.
    /// No-source providers write empty listings to cache without
    /// spawning any subprocess.
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
            results.push((provider, self.refresh_provider(provider).await));
        }
        results
    }

    /// Return the effective catalog for a provider.
    ///
    /// The base is the generated expected-offering baseline
    /// ([`expected_baseline`]: ids plus rolling aliases); user overrides
    /// merge on top (additive or replace). The on-disk listing cache
    /// never feeds this — the dynamic listing is drift-channel input
    /// only.
    pub fn catalog_for(&self, provider: Provider) -> Vec<String> {
        let base = expected_baseline(provider);
        let override_entry = self.overrides.get(&provider);
        merge_overrides(provider, &base, override_entry)
    }

    /// Check whether a model ID is acceptable for a provider.
    ///
    /// Two-tier acceptance:
    ///
    /// 1. membership in [`catalog_for`](Self::catalog_for)
    ///    (case-insensitive) — expected-offering ids, their aliases, and
    ///    user overrides;
    /// 2. an offering-source namespace match: ids like `ollama/llama3.3`
    ///    ride a local-runner namespace whose model population cannot be
    ///    enumerated statically, so any `prefix/…` id under a declared
    ///    `offering_sources` prefix is accepted.
    pub fn is_valid(&self, provider: Provider, model_id: &str) -> bool {
        let catalog = self.catalog_for(provider);
        catalog.iter().any(|m| m.eq_ignore_ascii_case(model_id))
            || matches_offering_source(provider, model_id)
    }

    /// Catalog identity of a configured user override for `model_id`.
    ///
    /// Returns the `catalog_id` join key of an override value whose id
    /// matches `model_id` case-insensitively, if the user configured one.
    /// This is the identity-join hook for user-added models
    /// (model-catalog-boundary design, runtime migration step 3): overrides
    /// win the local merge, and an optional `catalog_id` lets such a model
    /// still join models-catalog identity. No other consumer exists yet.
    ///
    /// Only the object form of an override value carries a `catalog_id`;
    /// bare-string values (and the bare-list shorthand) yield `None`.
    pub fn override_catalog_id(&self, provider: Provider, model_id: &str) -> Option<String> {
        match self.overrides.get(&provider)? {
            ProviderModelOverride::AddList(_) => None,
            ProviderModelOverride::Detailed(detailed) => detailed
                .values
                .iter()
                .find_map(|value| {
                    if value.id().eq_ignore_ascii_case(model_id) {
                        value.catalog_id()
                    } else {
                        None
                    }
                })
                .map(str::to_string),
        }
    }

    /// Models from the on-disk listing cache for a provider, if any.
    ///
    /// The cached dynamic listing does not feed validation; it is the
    /// input the drift channel diffs against
    /// [`expected_ids`](super::expected_ids). `None` when no cache entry
    /// exists yet.
    pub fn cached_listing(&self, provider: Provider) -> Option<Vec<String>> {
        self.cache.read(provider).map(|entry| entry.models)
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

/// Whether `model_id` rides one of the provider's offering-source
/// namespaces (`<prefix>/<model>`, e.g. `ollama/llama3.3`).
///
/// The `/` separator is required: `offering_sources` prefixes are bare
/// runner names (`ollama`), and matching without the separator would
/// accept unrelated ids like `ollamafoo`. Prefix comparison is
/// case-insensitive, matching catalog membership.
fn matches_offering_source(provider: Provider, model_id: &str) -> bool {
    provider_info(provider).offering_sources.iter().any(|source| {
        model_id
            .get(..source.prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(source.prefix))
            && model_id[source.prefix.len()..].starts_with('/')
    })
}

#[cfg(test)]
mod tests;
