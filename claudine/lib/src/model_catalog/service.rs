//! Model catalog service.
//!
//! The service answers: "Is this model valid for this provider right now?"
//! It combines cached catalogs, user overrides, and dynamic sources.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::OnceCell;

use super::cache::{ModelCache, ModelCacheEntry};
use super::config::merge_overrides;
use super::provider_sources::{
    CatalogFetchError, fetch_provider_catalog, fetch_shell_command_models,
    static_catalog_for_provider,
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
/// Call [`refresh`](Self::refresh) to populate/update cached catalogs
/// before validation.
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
    /// Performs at most one subprocess per `ShellCommand` source per
    /// service instance: a repeat refresh of the same command reuses the
    /// memoized result. Static-source providers write the static list to
    /// cache without spawning any subprocess; providers without a source
    /// are no-ops.
    ///
    /// Never panics; failures are silently ignored so stale cache or
    /// static fallback remains available.
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
    /// Returns immediately, leaving the existing on-disk cache (if any)
    /// in place for the current invocation. The refresh runs in a
    /// detached worker thread and updates the on-disk cache for the
    /// *next* invocation.
    ///
    /// ## Cold start
    ///
    /// When no cache exists yet for a dynamic-source provider (true
    /// first-run), this falls back to [`refresh_provider_blocking`](Self::refresh_provider_blocking) so
    /// frontmatter `model:` validation has data to work with rather than
    /// silently flagging a brand-new model as unknown.
    ///
    /// ## Static and no-source providers
    ///
    /// Static-source providers (e.g. Claude, Codex, Qwen) and no-source
    /// providers are essentially free to refresh, so
    /// this delegates to [`refresh_provider_blocking`](Self::refresh_provider_blocking) which writes the
    /// static list to cache without spawning any subprocess.
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
            ModelCatalogSource::None | ModelCatalogSource::Static => {
                // Cheap: no subprocess spawn. Run inline so callers
                // observe the static catalog immediately.
                self.refresh_provider_blocking(provider);
                return;
            }
            ModelCatalogSource::ShellCommand { .. } => {}
        }

        // Cold-start fallback: no cache exists yet. We must block so the
        // current run has data; otherwise frontmatter `model:` validation
        // would silently flag a brand-new model as unknown.
        if self.cache.read(provider).is_none() {
            self.refresh_provider_blocking(provider);
            return;
        }

        // Detached background refresh. The next invocation reads the
        // refreshed cache; the current one keeps using the stale entry.
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

    /// Refresh the catalog for a single provider.
    ///
    /// Attempts to fetch the latest catalog. On failure, the existing cache
    /// is left untouched (stale-cache fallback).
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

    /// Refresh the catalog for a single provider with in-process dedup.
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
            ModelCatalogSource::Static => static_catalog_for_provider(provider),
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
    /// Static-source providers (Claude, Codex, Qwen)
    /// write their static lists to cache without spawning any subprocess.
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
    /// Merges cached data (or static source) with user overrides.
    pub fn catalog_for(&self, provider: Provider) -> Vec<String> {
        let base = match provider_info(provider).model_catalog_source {
            ModelCatalogSource::Static => static_catalog_for_provider(provider),
            ModelCatalogSource::None | ModelCatalogSource::ShellCommand { .. } => {
                match self.cache.read(provider) {
                    Some(entry) => entry.models,
                    None => static_catalog_for_provider(provider),
                }
            }
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

    /// OpenCode's `ShellCommand` identity, destructured from the
    /// generated catalog so tests stay truthful to the committed data.
    fn opencode_command() -> (&'static str, &'static [&'static str]) {
        match provider_info(Provider::OpenCode).model_catalog_source {
            ModelCatalogSource::ShellCommand { program, args } => (program, args),
            other => panic!("opencode must be ShellCommand-sourced, got {other:?}"),
        }
    }

    #[test]
    fn service_validates_static_model() {
        let service = ModelCatalogService::new();
        assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
        assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
    }

    #[test]
    fn static_source_ignores_stale_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let stale = ModelCacheEntry {
            provider: Provider::Codex,
            models: vec!["old-codex-model".into()],
            fetched_at: chrono::Utc::now(),
        };
        service.cache.write(&stale).unwrap();

        assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
        assert!(!service.is_valid(Provider::Codex, "old-codex-model"));
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
    fn gemini_static_catalog_is_research_fed() {
        // Research-fed as of generator v1 (previously empty/None-sourced).
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
    fn refresh_provider_blocking_static_no_subprocess() {
        // Static-source providers (Claude, Codex) must never spawn a
        // shell-command subprocess.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.refresh_provider_blocking(Provider::Claude);
        service.refresh_provider_blocking(Provider::Codex);
        assert_eq!(service.shell_command_fetch_attempts(), 0);
    }

    #[test]
    fn refresh_provider_blocking_no_shell_source_no_subprocess() {
        // Providers without a ShellCommand source must never spawn a
        // subprocess.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        service.refresh_provider_blocking(Provider::Gemini);
        service.refresh_provider_blocking(Provider::Goose);
        service.refresh_provider_blocking(Provider::KimiCode);
        assert_eq!(service.shell_command_fetch_attempts(), 0);
    }

    /// Qwen left shell sourcing (2026-07-05 ruling): it is Static now,
    /// so a refresh writes its research-fed list without any subprocess.
    #[test]
    fn refresh_provider_blocking_qwen_is_static_no_subprocess() {
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        service.refresh_provider_blocking(Provider::QwenCode);
        assert_eq!(service.shell_command_fetch_attempts(), 0);

        let qwen = service.catalog_for(Provider::QwenCode);
        assert!(
            qwen.contains(&"qwen3-coder-plus".into()),
            "expected research-fed static model in {qwen:?}"
        );
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
    fn refresh_provider_blocking_failure_falls_back_to_static() {
        // Even if the dynamic source has been primed with a failure,
        // catalog_for() must still return the static catalog when one
        // exists. Refresh failures must not corrupt later validation.
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let (program, args) = opencode_command();
        service.prime_shell_command_dedup(
            program,
            args,
            Err(CatalogFetchError::CliNotFound("opencode".into())),
        );
        service.refresh_provider_blocking(Provider::OpenCode);

        // OpenCode has no static catalog, but is_valid for Claude (a
        // static-source provider) must still work because refreshing
        // OpenCode never touches Claude state.
        assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
    }

    #[test]
    fn refresh_all_uses_primed_shell_result_and_static_qwen() {
        // refresh_all() must not spawn any subprocess when OpenCode's
        // command result is primed, and Qwen (Static since the
        // 2026-07-05 ruling) refreshes from its compiled list.
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

        // OpenCode reflects the primed shell result; Qwen reflects its
        // research-fed static list, not the OpenCode output.
        let opencode = service.catalog_for(Provider::OpenCode);
        assert!(opencode.contains(&"qwen-2.5-coder".into()));
        assert!(opencode.contains(&"gpt-5".into()));

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

    /// W3: a true cold start (no cache) must fall back to blocking
    /// behaviour so frontmatter `model:` validation has data.
    #[test]
    #[serial_test::serial]
    fn refresh_provider_async_blocks_when_no_cache() {
        use crate::model_catalog::provider_sources::CatalogFetchError;

        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        // Seed only the in-memory dedup cell so OpenCode "fetch" returns
        // immediately without spawning the real subprocess. The disk
        // cache remains empty, exercising the cold-start fallback.
        let (program, args) = opencode_command();
        service.prime_shell_command_dedup(
            program,
            args,
            Err(CatalogFetchError::CliNotFound("opencode".into())),
        );

        // Pre-condition: no on-disk cache.
        assert!(service.cache.read(Provider::OpenCode).is_none());

        // Cold-start path is synchronous: the call must block on
        // `refresh_provider_blocking` even though we asked for async.
        service.refresh_provider_async(Provider::OpenCode);
        // The fetcher returned an error so no entry was written, but the
        // call returned. (If the fallback hadn't been blocking, the test
        // would still race, but we'd lose the cold-cache contract.)
    }

    /// W3: static-source providers always run inline because the static
    /// catalog list is in-process and free.
    #[test]
    #[serial_test::serial]
    fn refresh_provider_async_static_runs_inline() {
        let tmp = tempfile::tempdir().unwrap();
        let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

        service.refresh_provider_async(Provider::Claude);
        // Static catalog must have been written to cache by the inline
        // refresh, so a follow-up read returns the static list.
        let read = service.cache.read(Provider::Claude).unwrap();
        assert!(!read.models.is_empty());
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
    fn refresh_all_static_providers_no_subprocess() {
        // refresh_all() must not spawn any subprocess for static-source
        // providers (Claude, Codex) and should still write their catalogs
        // to cache correctly.
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
            "static providers must not trigger a shell-command subprocess"
        );

        // Static catalogs should still be available via cache
        assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
        assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
    }
}
