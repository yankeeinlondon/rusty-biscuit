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
mod tests {
    use super::*;
    use crate::config::claudine_config::{
        DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
    };

    /// OpenCode's `ShellCommand` identity, destructured from the
    /// generated catalog so tests stay truthful to the committed data.
    fn opencode_command() -> (&'static str, &'static [&'static str]) {
        match provider_info(Provider::OpenCode).model_catalog_source {
            ModelCatalogSource::ShellCommand { program, args } => (program, args),
            other => panic!("opencode must be ShellCommand-sourced, got {other:?}"),
        }
    }

    #[test]
    fn service_validates_baseline_model() {
        let service = ModelCatalogService::new();
        assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
        assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
    }

    /// Aliases from the expected-offering records (`opus`, `flash`) are
    /// part of the validation baseline: users author them in frontmatter
    /// `model:` hints.
    #[test]
    fn service_validates_offering_alias() {
        let service = ModelCatalogService::new();
        assert!(service.is_valid(Provider::Claude, "opus"));
        assert!(service.is_valid(Provider::Gemini, "flash"));
    }

    /// Goose had no compiled model list before the baseline flip; its
    /// expected offerings made it validatable.
    #[test]
    fn service_validates_goose_from_expected_offerings() {
        let service = ModelCatalogService::new();
        assert!(service.is_valid(Provider::Goose, "gpt-5"));
    }

    /// Ids under a declared offering-source namespace (local runners)
    /// pass validation even though their model population cannot be
    /// enumerated statically. The `/` separator is mandatory.
    #[test]
    fn service_accepts_offering_source_prefix() {
        let service = ModelCatalogService::new();
        assert!(service.is_valid(Provider::OpenCode, "ollama/llama3.3"));
        assert!(service.is_valid(Provider::OpenCode, "OLLAMA/llama3.3"));
        assert!(!service.is_valid(Provider::OpenCode, "ollamallama3.3"));
        assert!(!service.is_valid(Provider::OpenCode, "no-such-runner/model"));
    }

    /// The on-disk listing cache never feeds validation — for any
    /// provider, no-source or shell-sourced alike.
    #[test]
    fn validation_ignores_listing_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        for provider in [Provider::Codex, Provider::OpenCode] {
            let stale = ModelCacheEntry {
                provider,
                models: vec!["cached-only-model".into()],
                fetched_at: chrono::Utc::now(),
            };
            service.cache.write(&stale).unwrap();
            assert!(!service.is_valid(provider, "cached-only-model"));
        }
        assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
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
        assert!(service.is_valid(Provider::Codex, "GPT-5.5"));
        assert!(service.is_valid(Provider::Claude, "CLAUDE-OPUS-4-8"));
    }

    #[test]
    fn service_with_additive_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Codex,
            ProviderModelOverride::AddList(vec!["custom-codex-model".into()]),
        );
        let service = ModelCatalogService::with_overrides(overrides);

        assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
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

        assert!(!service.is_valid(Provider::Codex, "gpt-5.5"));
        assert!(service.is_valid(Provider::Codex, "only-this-model"));
    }

    #[test]
    fn override_catalog_id_hit_miss_and_case_insensitive() {
        use crate::config::claudine_config::{ModelOverrideEntry, ModelOverrideValue};

        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Codex,
            ProviderModelOverride::Detailed(DetailedModelOverride {
                mode: ModelOverrideMode::Add,
                values: vec![
                    "plain-model".into(),
                    ModelOverrideValue::Entry(ModelOverrideEntry {
                        id: "joined-model".into(),
                        catalog_id: Some("openai/joined-model@1.0".into()),
                    }),
                ],
            }),
        );
        overrides.insert(
            Provider::Gemini,
            ProviderModelOverride::AddList(vec!["list-model".into()]),
        );
        let service = ModelCatalogService::with_overrides(overrides);

        // Hit, including case-insensitive id match.
        assert_eq!(
            service.override_catalog_id(Provider::Codex, "joined-model"),
            Some("openai/joined-model@1.0".to_string())
        );
        assert_eq!(
            service.override_catalog_id(Provider::Codex, "JOINED-MODEL"),
            Some("openai/joined-model@1.0".to_string())
        );

        // Miss: plain value, unknown id, bare-list shorthand, no override.
        assert_eq!(
            service.override_catalog_id(Provider::Codex, "plain-model"),
            None
        );
        assert_eq!(
            service.override_catalog_id(Provider::Codex, "not-configured"),
            None
        );
        assert_eq!(
            service.override_catalog_id(Provider::Gemini, "list-model"),
            None
        );
        assert_eq!(
            service.override_catalog_id(Provider::Claude, "anything"),
            None
        );
    }

    #[test]
    fn service_first_valid_finds_match() {
        let service = ModelCatalogService::new();
        let candidates = vec!["not-real".into(), "gpt-5.5".into(), "gpt-5.4".into()];
        assert_eq!(
            service.first_valid(Provider::Codex, &candidates),
            Some("gpt-5.5".into())
        );
    }

    #[test]
    fn service_first_valid_returns_none_when_no_match() {
        let service = ModelCatalogService::new();
        let candidates = vec!["not-real".into(), "also-fake".into()];
        assert_eq!(service.first_valid(Provider::Codex, &candidates), None);
    }

    #[test]
    fn gemini_catalog_is_research_fed() {
        // Expected offerings are research-fed for all providers
        // (previously Gemini was empty/None-sourced).
        let service = ModelCatalogService::new();
        assert!(!service.catalog_for(Provider::Gemini).is_empty());
        assert!(service.is_valid(Provider::Gemini, "gemini-2.5-pro"));
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
    fn refresh_provider_blocking_no_shell_source_no_subprocess() {
        // Providers without a ShellCommand source must never spawn a
        // subprocess.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.refresh_provider_blocking(Provider::Claude);
        service.refresh_provider_blocking(Provider::Codex);
        service.refresh_provider_blocking(Provider::Gemini);
        service.refresh_provider_blocking(Provider::Goose);
        service.refresh_provider_blocking(Provider::KimiCode);
        service.refresh_provider_blocking(Provider::QwenCode);
        assert_eq!(service.shell_command_fetch_attempts(), 0);
    }

    /// A `None`-sourced refresh writes an empty listing to the
    /// drift-channel cache — there is no dynamic listing to record, and
    /// validation is baseline-fed regardless.
    #[test]
    fn refresh_provider_blocking_none_source_writes_empty_listing() {
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        service.refresh_provider_blocking(Provider::QwenCode);
        assert_eq!(service.shell_command_fetch_attempts(), 0);

        let listing = service.cached_listing(Provider::QwenCode).unwrap();
        assert!(listing.is_empty(), "expected empty listing, got {listing:?}");
        assert!(service.is_valid(Provider::QwenCode, "qwen3-coder-plus"));
    }

    #[test]
    fn refresh_provider_blocking_opencode_twice_dedupes() {
        // A repeat OpenCode refresh must not re-attempt the subprocess.
        // We seed the dedup cache up front to avoid relying on
        // `opencode` being on PATH.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let (program, args) = opencode_command();
        service.prime_shell_command_dedup(
            program,
            args,
            Ok(vec!["qwen-coder".into(), "gpt-5.2".into()]),
        );
        service.refresh_provider_blocking(Provider::OpenCode);
        service.refresh_provider_blocking(Provider::OpenCode);
        assert_eq!(
            service.shell_command_fetch_attempts(),
            0,
            "primed dedup must short-circuit subprocess attempts"
        );
    }

    #[test]
    fn refresh_provider_blocking_failure_does_not_affect_validation() {
        // Validation is baseline-fed, so a failed listing fetch must
        // leave it fully intact — for the failing provider itself and
        // for every other provider.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let (program, args) = opencode_command();
        service.prime_shell_command_dedup(
            program,
            args,
            Err(CatalogFetchError::CliNotFound("opencode".into())),
        );
        service.refresh_provider_blocking(Provider::OpenCode);

        assert!(service.is_valid(Provider::OpenCode, "opencode/claude-opus-4-8"));
        assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
        // The failed fetch wrote nothing to the listing cache.
        assert!(service.cached_listing(Provider::OpenCode).is_none());
    }

    #[test]
    fn refresh_all_uses_primed_shell_result() {
        // refresh_all() must not spawn any subprocess when OpenCode's
        // command result is primed; no-source providers refresh for
        // free.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let (program, args) = opencode_command();
        service.prime_shell_command_dedup(
            program,
            args,
            Ok(vec![
                "qwen-2.5-coder".into(),
                "gpt-5".into(),
                "claude-sonnet-4".into(),
            ]),
        );

        service.refresh_blocking();

        assert_eq!(
            service.shell_command_fetch_attempts(),
            0,
            "primed dedup must short-circuit subprocess attempts in refresh_all"
        );

        // The primed shell result lands in the drift-channel listing
        // cache only; validation stays on the expected baseline, so the
        // listing-only ids are NOT valid.
        let listing = service.cached_listing(Provider::OpenCode).unwrap();
        assert!(listing.contains(&"qwen-2.5-coder".into()));
        assert!(listing.contains(&"gpt-5".into()));
        let opencode = service.catalog_for(Provider::OpenCode);
        assert!(!opencode.contains(&"qwen-2.5-coder".into()));
        assert!(opencode.contains(&"opencode/claude-opus-4-8".into()));

        // Qwen's baseline is unaffected by the OpenCode output.
        let qwen = service.catalog_for(Provider::QwenCode);
        assert!(qwen.contains(&"qwen3-coder-plus".into()));
        assert!(!qwen.contains(&"gpt-5".into()));
    }

    #[tokio::test]
    async fn concurrent_opencode_refreshes_run_fetcher_once() {
        // Drive two OpenCode refreshes concurrently against an
        // injectable fake source that blocks until released. The dedup
        // contract requires the fetcher to run exactly once even when
        // both callers observe the command's OnceCell as uninitialized
        // at the start of their await.
        use std::time::Duration;
        use tokio::sync::Notify;

        let tmp = tempfile::tempdir().unwrap();
        let mut service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        let fetch_count = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());

        let fetch_count_for_fetcher = fetch_count.clone();
        let started_for_fetcher = started.clone();
        let release_for_fetcher = release.clone();
        service.set_shell_command_fetcher(Arc::new(move |_program, _args| {
            let fetch_count = fetch_count_for_fetcher.clone();
            let started = started_for_fetcher.clone();
            let release = release_for_fetcher.clone();
            Box::pin(async move {
                fetch_count.fetch_add(1, Ordering::SeqCst);
                started.notify_waiters();
                release.notified().await;
                Ok(vec![
                    "qwen-2.5-coder".to_string(),
                    "gpt-5".to_string(),
                    "claude-sonnet-4".to_string(),
                ])
            })
        }));

        let s1 = service.clone();
        let s2 = service.clone();
        let first_handle =
            tokio::spawn(async move { s1.refresh_provider(Provider::OpenCode).await });
        let second_handle =
            tokio::spawn(async move { s2.refresh_provider(Provider::OpenCode).await });

        // Wait until the first (and only) fetcher invocation has begun
        // and is parked on `release.notified()`. Then give the second
        // task room to schedule and observe the in-flight OnceCell.
        started.notified().await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Release the in-flight fetch so both callers can complete.
        release.notify_waiters();

        let first_result = first_handle.await.unwrap().unwrap();
        let second_result = second_handle.await.unwrap().unwrap();

        assert_eq!(
            fetch_count.load(Ordering::SeqCst),
            1,
            "shell-command fetcher must run exactly once across concurrent refreshes"
        );
        assert_eq!(
            service.shell_command_fetch_attempts(),
            1,
            "OnceCell init closure must run exactly once"
        );

        assert!(first_result.contains(&"gpt-5".to_string()));
        assert_eq!(first_result, second_result);
    }

    /// W3: when a cache file already exists for a dynamic-source
    /// provider, `refresh_provider_async` must return promptly without
    /// blocking on the fetcher closure. We prove this by installing a
    /// fetcher that parks indefinitely; if the call were blocking, the
    /// test would exceed its time budget.
    ///
    /// `serial` because the env-var sibling test mutates
    /// `CLAUDINE_BACKGROUND_REFRESH` for the whole process.
    #[test]
    #[serial_test::serial]
    fn refresh_provider_async_returns_immediately_when_cache_exists() {
        use std::sync::mpsc;
        use std::time::Duration;
        use tokio::sync::Notify;

        let tmp = tempfile::tempdir().unwrap();
        let mut service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        // Seed an on-disk cache so the provider has a non-empty entry.
        let entry = ModelCacheEntry {
            provider: Provider::OpenCode,
            models: vec!["seeded-model".into()],
            fetched_at: chrono::Utc::now(),
        };
        service.cache.write(&entry).unwrap();

        // Install a fetcher that would block forever if called. The async
        // refresh should never await on it from the caller's thread.
        let parked = Arc::new(Notify::new());
        let parked_for_fetcher = parked.clone();
        service.set_shell_command_fetcher(Arc::new(move |_program, _args| {
            let parked = parked_for_fetcher.clone();
            Box::pin(async move {
                parked.notified().await;
                Ok(Vec::new())
            })
        }));

        // Run the call on a worker thread and require it to return well
        // under the wall-clock budget the parked fetcher would impose.
        let svc = service.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            svc.refresh_provider_async(Provider::OpenCode);
            let _ = tx.send(());
        });
        rx.recv_timeout(Duration::from_secs(5))
            .expect("refresh_provider_async must not block when cache exists");

        // The seeded cache must still be readable immediately after the
        // call returns — the current invocation always sees the existing
        // entry, never the in-flight refresh.
        let read = service.cache.read(Provider::OpenCode).unwrap();
        assert_eq!(read.models, vec!["seeded-model"]);

        // Release the parked fetcher so the detached background thread
        // can wind down without leaking the runtime.
        parked.notify_waiters();
    }

    /// Cold start (no cache) no longer blocks: validation is
    /// baseline-fed, so the subprocess result is never needed by the
    /// current run and the refresh always detaches to the background.
    /// Proven with a fetcher that parks indefinitely — a blocking
    /// fallback would exceed the time budget.
    #[test]
    #[serial_test::serial]
    fn refresh_provider_async_backgrounds_even_without_cache() {
        use std::sync::mpsc;
        use std::time::Duration;
        use tokio::sync::Notify;

        let tmp = tempfile::tempdir().unwrap();
        let mut service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        // Pre-condition: no on-disk cache (the old contract blocked here).
        assert!(service.cache.read(Provider::OpenCode).is_none());

        let parked = Arc::new(Notify::new());
        let parked_for_fetcher = parked.clone();
        service.set_shell_command_fetcher(Arc::new(move |_program, _args| {
            let parked = parked_for_fetcher.clone();
            Box::pin(async move {
                parked.notified().await;
                Ok(Vec::new())
            })
        }));

        let svc = service.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            svc.refresh_provider_async(Provider::OpenCode);
            let _ = tx.send(());
        });
        rx.recv_timeout(Duration::from_secs(5))
            .expect("refresh_provider_async must not block on a cold cache");

        // Release the parked fetcher so the detached background thread
        // can wind down without leaking the runtime.
        parked.notify_waiters();
    }

    /// W3: no-source providers always run inline because writing the
    /// empty listing is in-process and free.
    #[test]
    #[serial_test::serial]
    fn refresh_provider_async_none_source_runs_inline() {
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        service.refresh_provider_async(Provider::Claude);
        // The inline refresh wrote a (necessarily empty) listing cache
        // entry before returning.
        let read = service.cache.read(Provider::Claude).unwrap();
        assert!(read.models.is_empty());
        assert_eq!(service.shell_command_fetch_attempts(), 0);
    }

    /// W3 escape hatch: `CLAUDINE_BACKGROUND_REFRESH=0` forces the
    /// caller-blocking path even when a cache exists.
    #[test]
    #[serial_test::serial]
    fn refresh_provider_async_env_var_disables_background() {
        use crate::model_catalog::provider_sources::CatalogFetchError;

        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        let entry = ModelCacheEntry {
            provider: Provider::OpenCode,
            models: vec!["seeded".into()],
            fetched_at: chrono::Utc::now(),
        };
        service.cache.write(&entry).unwrap();
        let (program, args) = opencode_command();
        service.prime_shell_command_dedup(
            program,
            args,
            Err(CatalogFetchError::CliNotFound("opencode".into())),
        );

        let prior = std::env::var("CLAUDINE_BACKGROUND_REFRESH").ok();
        unsafe {
            std::env::set_var("CLAUDINE_BACKGROUND_REFRESH", "0");
        }
        // With the env var set we go through `refresh_provider_blocking`,
        // which uses the primed dedup cell and returns synchronously.
        service.refresh_provider_async(Provider::OpenCode);
        unsafe {
            match prior {
                Some(v) => std::env::set_var("CLAUDINE_BACKGROUND_REFRESH", v),
                None => std::env::remove_var("CLAUDINE_BACKGROUND_REFRESH"),
            }
        }

        // The original cache survives because the primed fetch returned an error.
        let read = service.cache.read(Provider::OpenCode).unwrap();
        assert_eq!(read.models, vec!["seeded"]);
    }

    #[test]
    fn refresh_all_none_source_providers_no_subprocess() {
        // refresh_all() must not spawn any subprocess for no-source
        // providers (Claude, Codex) and should still write their (empty)
        // listing cache entries.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        // Prime the dedup cache so the OpenCode refresh does not reach
        // the subprocess.
        let (program, args) = opencode_command();
        service.prime_shell_command_dedup(
            program,
            args,
            Ok(vec!["qwen-2.5-coder".into(), "gpt-5".into()]),
        );
        service.refresh_blocking();

        assert_eq!(
            service.shell_command_fetch_attempts(),
            0,
            "no-source providers must not trigger a shell-command subprocess"
        );

        // Listing cache entries were written, and baseline validation is
        // unaffected either way.
        assert!(service.cached_listing(Provider::Claude).is_some());
        assert!(service.cached_listing(Provider::Codex).is_some());
        assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
        assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
    }
}
