//! Eager fetch orchestration for remote URL dependencies.
//!
//! Provides `RemoteFetchRuntime` — a single-flight, concurrency-capped
//! fetch orchestrator that darkmatter's compose pipeline uses to resolve
//! remote `::file` and `::code` directives and URL-typed expression-function
//! arguments.

#[cfg(test)]
use std::collections::HashSet;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use biscuit_file::file_reference::fetch::{FetchPolicy, HostPattern, PolicyClient};
use dashmap::DashMap;
use url::Url;

use super::ComposeWarning;
use super::cache::FileStore;
use super::cache::remote_cache::{RemoteCacheConfig, RemoteOutcomeEvent, fetch_with_cache};
use super::expression::ExpressionError;
use super::remote::{RemoteReadConfig, RemoteReadError};

/// Default cache config for test constructors: no TTL override, no forced
/// refresh, and the default (`Fallback`) freshness mode.
#[cfg(test)]
fn test_cache_config() -> RemoteCacheConfig {
    RemoteCacheConfig {
        ttl_override: None,
        refresh: false,
        mode: super::remote::RemoteFreshnessMode::default(),
    }
}

/// Per-URL single-flight slot.
enum FetchSlot {
    /// A fetch task is in progress.
    InFlight,
    /// Fetch completed successfully with the response body as UTF-8 text.
    Ready { body: String },
    /// Fetch failed — the error message is stored for all waiters.
    Failed(String),
}

struct SlotGuard {
    state: Mutex<FetchSlot>,
    notify: Condvar,
}

/// Statistics collected during a compose run's remote-fetch activity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RemoteFetchStats {
    /// Number of distinct URLs fetched.
    pub fetched: usize,
    /// Number of times a consumer waited on an already-in-flight fetch.
    pub waits: usize,
    /// Number of URLs denied by policy (no network request attempted).
    pub policy_denials: usize,
    /// Number of URLs that failed due to network or HTTP errors.
    pub failures: usize,
    /// Number of bodies served from the persistent cache without network.
    pub cache_hits: usize,
    /// Number of conditional GETs issued to revalidate cached bodies.
    pub revalidations: usize,
    /// Number of `304 Not Modified` responses (cached body preserved).
    pub not_modified: usize,
    /// Number of stale cached bodies served after a network failure.
    pub stale_served: usize,
    /// Non-fatal persistent-cache I/O failures (a failed write or `no-store`
    /// purge). They never change a fetch's outcome or the counters above.
    pub cache_warnings: Vec<String>,
}

impl RemoteFetchStats {
    /// Stage of the compose warnings projected from [`Self::cache_warnings`].
    pub const CACHE_WARNING_STAGE: &'static str = "remote_cache";
    /// Stable code of the compose warnings projected from
    /// [`Self::cache_warnings`].
    pub const CACHE_WARNING_CODE: &'static str = "dm.remote_cache.io_failure";

    /// Projects each cache warning into the compose warning model.
    pub(crate) fn cache_compose_warnings(&self) -> impl Iterator<Item = ComposeWarning> + '_ {
        self.cache_warnings.iter().map(|message| ComposeWarning {
            code: Some(Self::CACHE_WARNING_CODE.to_string()),
            ..ComposeWarning::new(Self::CACHE_WARNING_STAGE, message.as_str())
        })
    }
}

/// Shared runtime for remote URL fetching across a compose pipeline.
///
/// Holds a single redirect-disabled `PolicyClient` and a deduplicated map of in-flight
/// fetch slots. Each unique URL is fetched at most once per compose run;
/// subsequent consumers block on the same slot and reuse the result.
///
/// ## Concurrency
///
/// All fetch tasks run on a single shared multi-thread Tokio runtime owned by
/// this struct and built lazily on the first registration. Its worker-thread
/// count is bounded by `RemoteReadConfig::remote_concurrency`, so registering
/// many URLs never spawns one OS thread or runtime per URL. A
/// `tokio::sync::Semaphore` sized from the same cap is acquired inside each
/// task before its request, so at most `remote_concurrency` requests are in
/// flight at once regardless of how many URLs are registered.
///
/// ## Lifecycle
///
/// 1. Construct with `RemoteFetchRuntime::with_store(config, store)`.
/// 2. Call `register_and_fetch(url)` for each discovered URL to spawn its
///    fetch task onto the shared runtime.
/// 3. Call `get_content(url)` at point-of-use to block until the fetch
///    completes and retrieve the body text.
/// 4. Call `stats()` after the compose run to collect metrics.
#[derive(Debug, Clone)]
pub struct RemoteFetchRuntime {
    inner: Arc<RemoteFetchInner>,
}

struct RemoteFetchInner {
    slots: DashMap<String, Arc<SlotGuard>>,
    provider_queries: ProviderQueryCache,
    provider_in_flight: Mutex<usize>,
    provider_notify: Condvar,
    policy: FetchPolicy,
    stats: Mutex<RemoteFetchStats>,
    /// Shared redirect-disabled HTTP client reused for every fetch in a compose
    /// run. Built lazily on first fetch so construction happens *inside* the
    /// shared runtime (reqwest binds its IO/timer driver to the ambient runtime
    /// at build time; a client built on a non-runtime thread — e.g. the sync CLI
    /// `main` — fails every send with "error sending request"). `None` (after
    /// init) means the client build failed; every fetch is then reported as a
    /// failure rather than falling back to a redirect-following client.
    client: OnceLock<Option<PolicyClient>>,
    /// Caps concurrent in-flight network requests. Acquired inside each
    /// spawned fetch task, so the slot map may hold more entries than there
    /// are permits without all requests being issued at once.
    semaphore: Arc<tokio::sync::Semaphore>,
    /// Shared executor for all fetch tasks, built lazily on first use so a
    /// compose run with no remote references never spawns worker threads. Its
    /// worker-thread count is bounded by `concurrency`. `None` means the build
    /// failed and every fetch is reported as a failure.
    runtime: OnceLock<Option<tokio::runtime::Runtime>>,
    /// Concurrency cap: sizes both the semaphore and the shared runtime's
    /// worker-thread pool.
    concurrency: usize,
    /// Current number of in-flight fetches (past the semaphore), and the
    /// high-water mark observed. The peak proves the cap bounds spawned work,
    /// not merely request entry.
    in_flight: AtomicUsize,
    peak_in_flight: AtomicUsize,
    /// Optional persistent store for remote artifacts. When absent, every
    /// fetch hits the network with no cross-run caching.
    store: Option<Arc<FileStore>>,
    /// TTL, refresh, and freshness-mode inputs for cache evaluation.
    cache_config: RemoteCacheConfig,
}

/// Provider query slots store the typed expression error so a focused
/// provider classification survives memoization; see
/// [`ResolutionContext::cached_provider_query`].
///
/// [`ResolutionContext::cached_provider_query`]: super::expression::ResolutionContext
type ProviderQueryCache =
    Mutex<HashMap<String, Arc<OnceLock<Result<serde_json::Value, ExpressionError>>>>>;

/// A provider-query cache infrastructure failure (poisoned lock, concurrency
/// gate). Not a provider failure, so it stays the generic catch-all.
fn provider_cache_infrastructure_error(message: &str) -> ExpressionError {
    ExpressionError::Other {
        function: "provider_query".to_string(),
        message: message.to_string(),
    }
}

impl RemoteFetchInner {
    /// Returns the shared fetch runtime, building it on first use.
    ///
    /// `get_or_init` guarantees the multi-thread runtime is built at most once
    /// even under concurrent registration, so no runtime is ever dropped in an
    /// async context. Returns `None` only if the build failed.
    fn runtime(&self) -> Option<&tokio::runtime::Runtime> {
        self.runtime
            .get_or_init(|| {
                count_executor_build();
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(self.concurrency.max(1))
                    .enable_all()
                    .build()
                    .ok()
            })
            .as_ref()
    }

    /// Returns the shared HTTP client, building it on first use.
    ///
    /// Must be called from a thread with an active Tokio runtime (every fetch
    /// task is, since it runs on [`runtime`](Self::runtime)). Building here
    /// rather than in [`build`](RemoteFetchRuntime::build) guarantees reqwest
    /// binds to the runtime that will issue the request. Returns `None` only if
    /// the client build failed.
    fn client(&self) -> Option<&PolicyClient> {
        self.client
            .get_or_init(|| PolicyClient::new().ok())
            .as_ref()
    }
}

impl Drop for RemoteFetchInner {
    /// Shuts the shared runtime down without blocking.
    ///
    /// A `RemoteFetchRuntime` is commonly dropped from within an async context
    /// (e.g. a `#[tokio::test]` body or a caller's own runtime). The blocking
    /// shutdown in `Runtime`'s own `Drop` would panic there, so the runtime is
    /// taken out and shut down in the background instead.
    fn drop(&mut self) {
        if let Some(Some(runtime)) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

thread_local! {
    /// Handle to the compose run's shared executor, installed for the duration
    /// of one provider-query closure.
    ///
    /// The expression evaluator is synchronous and `provider::run` takes no
    /// context argument, so the run's executor reaches the sync bridge through
    /// this scoped slot rather than through a parameter.
    static RUN_PROVIDER_EXECUTOR: RefCell<Option<tokio::runtime::Handle>> =
        const { RefCell::new(None) };
}

/// Process-wide executor for provider queries issued outside any compose run
/// that owns a `RemoteFetchRuntime`.
///
/// Never dropped. `Runtime`'s own `Drop` shuts down blockingly and panics when
/// that happens inside an async context; a `static` is never dropped, so the
/// hazard cannot arise. `None` means the build failed.
static FALLBACK_PROVIDER_EXECUTOR: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();

/// Counts every provider-executor runtime constructed in this process.
///
/// The whole point of the shared executor is that this stays flat across a
/// run's provider queries, which is only observable with a counter.
#[cfg(test)]
static EXECUTOR_BUILDS: AtomicUsize = AtomicUsize::new(0);

fn count_executor_build() {
    #[cfg(test)]
    EXECUTOR_BUILDS.fetch_add(1, Ordering::SeqCst);
}

/// Number of provider-executor runtimes constructed so far in this process.
#[cfg(test)]
pub(crate) fn executor_builds() -> usize {
    EXECUTOR_BUILDS.load(Ordering::SeqCst)
}

/// Restores the previously scoped provider executor when dropped, so nested
/// compose runs do not leak one run's executor into another.
struct ProviderExecutorScope(Option<tokio::runtime::Handle>);

impl ProviderExecutorScope {
    fn enter(handle: Option<tokio::runtime::Handle>) -> Self {
        Self(RUN_PROVIDER_EXECUTOR.with(|cell| cell.replace(handle)))
    }
}

impl Drop for ProviderExecutorScope {
    fn drop(&mut self) {
        let previous = self.0.take();
        RUN_PROVIDER_EXECUTOR.with(|cell| cell.replace(previous));
    }
}

/// Why a provider future produced no value.
pub(crate) enum SharedExecutorError {
    /// No executor could be built, so the future was never polled.
    Unavailable,
    /// The future panicked, or its executor shut down before it completed.
    Panicked,
}

/// Runs `future` to completion on the compose run's shared executor.
///
/// The future is `spawn`ed onto a *foreign* runtime and the calling thread
/// blocks on a channel for its result. This never calls `block_on`, so a caller
/// that is itself inside a Tokio runtime does not hit the "cannot start a
/// runtime from within a runtime" panic — the property the previous
/// thread-per-query bridge existed to provide.
///
/// ## Errors
///
/// Returns [`SharedExecutorError::Panicked`] when the task drops its sender
/// without sending, which covers both an unwinding future and a shut-down
/// executor. A panic therefore surfaces as a value rather than unwinding into
/// the compose run.
pub(crate) fn block_on_shared_executor<T>(
    future: impl Future<Output = T> + Send + 'static,
) -> Result<T, SharedExecutorError>
where
    T: Send + 'static,
{
    let handle = RUN_PROVIDER_EXECUTOR
        .with(|cell| cell.borrow().clone())
        .or_else(fallback_executor_handle)
        .ok_or(SharedExecutorError::Unavailable)?;

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    handle.spawn(async move {
        let _ = sender.send(future.await);
    });
    receiver.recv().map_err(|_| SharedExecutorError::Panicked)
}

fn fallback_executor_handle() -> Option<tokio::runtime::Handle> {
    FALLBACK_PROVIDER_EXECUTOR
        .get_or_init(|| {
            count_executor_build();
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(DEFAULT_PROVIDER_CONCURRENCY)
                .enable_all()
                .build()
                .ok()
        })
        .as_ref()
        .map(|runtime| runtime.handle().clone())
}

/// Worker count for the fallback executor, matching the default remote
/// concurrency cap used by `RemoteFetchRuntime`.
const DEFAULT_PROVIDER_CONCURRENCY: usize = 4;

impl std::fmt::Debug for RemoteFetchInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteFetchInner")
            .field("policy", &self.policy)
            .field("stats", &self.stats)
            .finish()
    }
}

/// Clone-stable weak identity handle for a [`RemoteFetchRuntime`].
///
/// Retains only a `Weak` reference to the shared inner allocation, so identity
/// tracking never extends the runtime's lifetime and never stores a raw pointer
/// address. Cloning a `RemoteFetchRuntime` shares the same allocation, so both
/// handles compare equal; a dropped or independently constructed runtime never
/// compares equal even when its visible configuration matches. This is an
/// in-process instance-identity guard, not a cryptographic identity.
// `same_instance` is exercised by the graph-options identity comparison, which
// the graph does not consume until the opacity cutover; allowance is temporary.
#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct RemoteFetchWeakId(Weak<RemoteFetchInner>);

#[allow(dead_code)]
impl RemoteFetchWeakId {
    /// Returns `true` when both handles still point at the same live
    /// allocation.
    ///
    /// Uses `Weak::upgrade` + `Arc::ptr_eq` so a dropped instance (whose weak
    /// upgrade yields `None`) is never equivalent — this is ABA-safe because
    /// the live `Weak` keeps the allocation's control block from being reused
    /// while either handle exists.
    pub(crate) fn same_instance(&self, other: &Self) -> bool {
        match (self.0.upgrade(), other.0.upgrade()) {
            (Some(a), Some(b)) => Arc::ptr_eq(&a, &b),
            _ => false,
        }
    }

    /// Drop-probe: `true` while the referenced runtime allocation is still
    /// live. Lets a test that has already released its own strong handle prove
    /// the runtime was actually dropped — i.e. that nothing else (notably a
    /// graph) retained a strong reference to it.
    #[cfg(test)]
    pub(crate) fn is_alive(&self) -> bool {
        self.0.upgrade().is_some()
    }
}

impl RemoteFetchRuntime {
    /// Returns the exact host policy shared by all run-local remote reads.
    pub(crate) fn policy(&self) -> FetchPolicy {
        self.inner.policy.clone()
    }

    /// Returns a clone-stable weak identity handle for this runtime.
    ///
    /// The narrow accessor exists so identity capture never reaches through the
    /// private `inner` field; it only downgrades the shared allocation.
    pub(crate) fn weak_id(&self) -> RemoteFetchWeakId {
        RemoteFetchWeakId(Arc::downgrade(&self.inner))
    }

    /// Strong-reference count of the shared runtime allocation.
    ///
    /// Test-only observation handle: identity capture and graph ownership only
    /// ever *downgrade* this `Arc`, so this count lets a graph-level test prove
    /// that building (and dropping) a graph adds no strong reference.
    #[cfg(test)]
    pub(crate) fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// Creates a runtime from the remote-read config with an optional
    /// persistent cache store for cross-run remote artifact caching.
    pub fn with_store(config: &RemoteReadConfig, store: Option<Arc<FileStore>>) -> Self {
        let mut policy = FetchPolicy::deny_all();
        for host in config.http_hosts() {
            policy = policy.allow(HostPattern::Exact(host.clone()));
        }
        let cache_config = RemoteCacheConfig {
            ttl_override: config.remote_ttl,
            refresh: config.refresh,
            mode: config.freshness_mode,
        };
        Self::build(policy, config.remote_concurrency, store, cache_config)
    }

    /// Creates a runtime with a pre-built policy and the default concurrency
    /// cap (for testing).
    #[cfg(test)]
    pub fn with_policy(policy: FetchPolicy) -> Self {
        Self::build(
            policy,
            RemoteReadConfig::default().remote_concurrency,
            None,
            test_cache_config(),
        )
    }

    /// Creates a runtime with a pre-built policy and an explicit concurrency
    /// cap (for testing the cap directly).
    #[cfg(test)]
    pub fn with_policy_and_concurrency(policy: FetchPolicy, concurrency: usize) -> Self {
        Self::build(policy, concurrency, None, test_cache_config())
    }

    /// Creates a runtime with a pre-built policy, a persistent store, and an
    /// explicit cache config (for testing freshness behavior).
    #[cfg(test)]
    pub fn with_policy_store_and_config(
        policy: FetchPolicy,
        store: Option<Arc<FileStore>>,
        cache_config: RemoteCacheConfig,
    ) -> Self {
        Self::build(policy, 4, store, cache_config)
    }

    fn build(
        policy: FetchPolicy,
        concurrency: usize,
        store: Option<Arc<FileStore>>,
        cache_config: RemoteCacheConfig,
    ) -> Self {
        Self {
            inner: Arc::new(RemoteFetchInner {
                slots: DashMap::new(),
                provider_queries: Mutex::new(HashMap::new()),
                provider_in_flight: Mutex::new(0),
                provider_notify: Condvar::new(),
                policy,
                stats: Mutex::new(RemoteFetchStats::default()),
                client: OnceLock::new(),
                // A zero cap would deadlock every fetch; clamp to at least one.
                semaphore: Arc::new(tokio::sync::Semaphore::new(concurrency.max(1))),
                runtime: OnceLock::new(),
                concurrency: concurrency.max(1),
                in_flight: AtomicUsize::new(0),
                peak_in_flight: AtomicUsize::new(0),
                store,
                cache_config,
            }),
        }
    }

    /// Shares one normalized provider result across every surface in this run.
    pub(crate) fn cached_provider_query(
        &self,
        key: String,
        query: impl FnOnce() -> Result<serde_json::Value, ExpressionError>,
    ) -> Result<serde_json::Value, ExpressionError> {
        let slot = {
            let mut queries = self
                .inner
                .provider_queries
                .lock()
                .map_err(|_| provider_cache_infrastructure_error("provider query cache lock was poisoned"))?;
            queries.entry(key).or_default().clone()
        };
        slot.get_or_init(|| {
            let _permit = self.acquire_provider_permit()?;
            // Provider futures run on the same executor as this run's URL
            // fetches, so provider work shares the run's executor lifecycle
            // instead of standing up a runtime of its own.
            let _executor = ProviderExecutorScope::enter(
                self.inner.runtime().map(|runtime| runtime.handle().clone()),
            );
            query()
        })
        .clone()
    }

    fn acquire_provider_permit(&self) -> Result<ProviderPermit<'_>, ExpressionError> {
        let mut in_flight = self
            .inner
            .provider_in_flight
            .lock()
            .map_err(|_| provider_cache_infrastructure_error("provider concurrency lock was poisoned"))?;
        while *in_flight >= self.inner.concurrency {
            in_flight = self
                .inner
                .provider_notify
                .wait(in_flight)
                .map_err(|_| provider_cache_infrastructure_error("provider concurrency lock was poisoned"))?;
        }
        *in_flight += 1;
        Ok(ProviderPermit { runtime: self })
    }

    /// Checks whether a URL is allowed by policy, returning a
    /// `RemoteReadError::DeniedByPolicy` if not.
    pub fn check_allowed(&self, url: &Url) -> Result<(), RemoteReadError> {
        let host = url
            .host_str()
            .ok_or_else(|| RemoteReadError::InvalidUrl {
                url: url.to_string(),
                reason: "missing host".to_string(),
            })?;
        if self.inner.policy.is_allowed(host) {
            Ok(())
        } else {
            {
                let mut stats = self.inner.stats.lock().unwrap();
                stats.policy_denials += 1;
            }
            Err(RemoteReadError::DeniedByPolicy {
                host: host.to_string(),
            })
        }
    }

    /// Registers a URL and starts an eager fetch on the Tokio runtime.
    ///
    /// If the URL is already registered (in-flight or completed), this is a
    /// no-op. If the host is not in the policy allowlist, the slot is
    /// immediately filled with a policy-denial error.
    pub fn register_and_fetch(&self, url: Url) {
        let key = url.to_string();

        if let Err(err) = self.check_allowed(&url) {
            let slot = Arc::new(SlotGuard {
                state: Mutex::new(FetchSlot::Failed(err.to_string())),
                notify: Condvar::new(),
            });
            self.inner.slots.entry(key).or_insert(slot);
            return;
        }

        let slot = Arc::new(SlotGuard {
            state: Mutex::new(FetchSlot::InFlight),
            notify: Condvar::new(),
        });

        let entry = self.inner.slots.entry(key.clone());
        use dashmap::mapref::entry::Entry;
        match entry {
            Entry::Occupied(_) => {}
            Entry::Vacant(vacant) => {
                vacant.insert(slot.clone());
                let inner = Arc::clone(&self.inner);
                let Some(runtime) = inner.runtime() else {
                    // Runtime build failed: fail the slot rather than leave
                    // waiters blocked forever on a fetch that will never run.
                    {
                        let mut stats = inner.stats.lock().unwrap();
                        stats.failures += 1;
                    }
                    *slot.state.lock().unwrap() =
                        FetchSlot::Failed("failed to initialize remote fetch runtime".to_string());
                    slot.notify.notify_all();
                    return;
                };

                let task_inner = Arc::clone(&self.inner);
                let task_slot = Arc::clone(&slot);
                runtime.spawn(async move {
                    let _permit = task_inner.semaphore.clone().acquire_owned().await;
                    // Count in-flight only past the semaphore so the peak
                    // reflects requests actually issued, bounded by the cap.
                    let current = task_inner.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    task_inner.peak_in_flight.fetch_max(current, Ordering::SeqCst);
                    let mut cache_warnings = Vec::new();
                    let result = match task_inner.client() {
                        Some(client) => {
                            fetch_with_cache(
                                task_inner.store.as_deref(),
                                client,
                                &url,
                                &task_inner.policy,
                                &task_inner.cache_config,
                                &mut cache_warnings,
                            )
                            .await
                        }
                        None => Err("failed to initialize remote fetch client".to_string()),
                    };
                    task_inner.in_flight.fetch_sub(1, Ordering::SeqCst);
                    if !cache_warnings.is_empty() {
                        task_inner.stats.lock().unwrap().cache_warnings.extend(cache_warnings);
                    }

                    let mut guard = task_slot.state.lock().unwrap();
                    match result {
                        Ok(outcome) => {
                            {
                                let mut stats = task_inner.stats.lock().unwrap();
                                if outcome.revalidated {
                                    stats.revalidations += 1;
                                }
                                match outcome.event {
                                    RemoteOutcomeEvent::Fetched => stats.fetched += 1,
                                    RemoteOutcomeEvent::CacheHit => stats.cache_hits += 1,
                                    RemoteOutcomeEvent::NotModified => {
                                        stats.not_modified += 1;
                                    }
                                    RemoteOutcomeEvent::StaleServed => {
                                        stats.stale_served += 1;
                                    }
                                }
                            }
                            *guard = FetchSlot::Ready { body: outcome.body };
                        }
                        Err(err) => {
                            {
                                let mut stats = task_inner.stats.lock().unwrap();
                                stats.failures += 1;
                            }
                            *guard = FetchSlot::Failed(err);
                        }
                    }
                    task_slot.notify.notify_all();
                });
            }
        }
    }

    /// Registers a URL that was discovered *after* the initial eager-fetch
    /// phase (e.g., nested remote references found inside fetched Markdown).
    ///
    /// Equivalent to `register_and_fetch` but named to highlight the
    /// post-discovery use case.
    pub fn register_nested(&self, url: Url) {
        self.register_and_fetch(url);
    }

    /// Blocks the calling thread until the URL's fetch completes, then
    /// returns the body text.
    ///
    /// Returns an error string if the fetch failed or was denied by policy.
    /// Returns `None` if the URL was never registered.
    pub fn get_content(&self, url: &Url) -> Result<Option<String>, String> {
        let key = url.to_string();
        let slot = match self.inner.slots.get(&key) {
            Some(s) => s,
            None => return Ok(None),
        };

        let mut guard = slot.state.lock().unwrap();
        loop {
            match &*guard {
                FetchSlot::Ready { body } => return Ok(Some(body.clone())),
                FetchSlot::Failed(err) => return Err(err.clone()),
                FetchSlot::InFlight => {
                    {
                        let mut stats = self.inner.stats.lock().unwrap();
                        stats.waits += 1;
                    }
                    guard = slot.notify.wait(guard).unwrap();
                }
            }
        }
    }

    /// Returns a snapshot of the remote-fetch statistics.
    pub fn stats(&self) -> RemoteFetchStats {
        self.inner.stats.lock().unwrap().clone()
    }

    /// Returns the set of all registered URL keys (for diagnostics in tests).
    #[cfg(test)]
    pub fn registered_urls(&self) -> HashSet<String> {
        self.inner.slots.iter().map(|e| e.key().clone()).collect()
    }

    /// Returns the high-water mark of concurrent in-flight fetches observed.
    ///
    /// A fetch counts as in-flight only after it acquires a semaphore permit,
    /// so this value can never exceed the configured concurrency cap. Tests use
    /// it to prove the cap bounds spawned work, not just request entry.
    #[cfg(test)]
    pub fn peak_in_flight(&self) -> usize {
        self.inner.peak_in_flight.load(Ordering::SeqCst)
    }
}

struct ProviderPermit<'a> {
    runtime: &'a RemoteFetchRuntime,
}

impl Drop for ProviderPermit<'_> {
    fn drop(&mut self) {
        if let Ok(mut in_flight) = self.runtime.inner.provider_in_flight.lock() {
            *in_flight = in_flight.saturating_sub(1);
            self.runtime.inner.provider_notify.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_url() -> Url {
        Url::parse("https://example.com/doc.md").unwrap()
    }

    #[test]
    fn deny_all_policy_rejects_host() {
        let rt = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let url = example_url();
        let err = rt.check_allowed(&url).unwrap_err();
        assert!(matches!(err, RemoteReadError::DeniedByPolicy { .. }));
    }

    #[test]
    fn allowed_host_passes_check() {
        let policy = FetchPolicy::deny_all().allow_host("example.com");
        let rt = RemoteFetchRuntime::with_policy(policy);
        let url = example_url();
        assert!(rt.check_allowed(&url).is_ok());
    }

    #[test]
    fn stats_initially_zero() {
        let rt = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let stats = rt.stats();
        assert_eq!(stats.fetched, 0);
        assert_eq!(stats.waits, 0);
        assert_eq!(stats.policy_denials, 0);
        assert_eq!(stats.failures, 0);
    }

    #[test]
    fn get_content_returns_none_for_unregistered_url() {
        let rt = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let url = example_url();
        assert_eq!(rt.get_content(&url).unwrap(), None);
    }

    #[test]
    fn policy_denial_fills_slot_with_error() {
        let rt = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let url = example_url();
        rt.register_and_fetch(url.clone());

        let result = rt.get_content(&url);
        assert!(result.is_err());
        let stats = rt.stats();
        assert_eq!(stats.policy_denials, 1);
    }

    #[test]
    fn registered_urls_tracks_keys() {
        let rt = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let url = example_url();
        // Use deny-all policy so no tokio spawn occurs.
        rt.register_and_fetch(url);
        let urls = rt.registered_urls();
        assert!(urls.contains("https://example.com/doc.md"));
    }

    /// A provider query must execute on the run's own executor, not on the
    /// process-wide fallback, so provider work follows the run's lifecycle.
    ///
    /// Enumerating the run executor's worker threads and asserting membership
    /// is not sound: tokio work-stealing migrates a task between workers across
    /// an await point. The build counter is the deterministic discriminator —
    /// falling through to the fallback would construct a second executor.
    #[test]
    fn provider_queries_run_on_the_runs_own_executor() {
        let runtime = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let builds_before = executor_builds();

        let observed = runtime
            .cached_provider_query("probe".to_string(), || {
                let id = block_on_shared_executor(async {
                    format!("{:?}", std::thread::current().id())
                })
                .unwrap_or_else(|_| panic!("shared executor is available"));
                Ok(serde_json::json!(id))
            })
            .expect("query succeeds");

        assert_eq!(
            executor_builds() - builds_before,
            1,
            "exactly one executor — the run's own — may be built; a second \
             build means the provider future fell through to the process-wide \
             fallback instead of using the run's executor"
        );
        assert_ne!(
            observed.as_str().expect("thread id string"),
            format!("{:?}", std::thread::current().id()),
            "the future must run on the executor, not inline on the caller"
        );
    }

    #[test]
    fn provider_queries_share_the_remote_concurrency_cap() {
        let runtime = RemoteFetchRuntime::with_policy_and_concurrency(
            FetchPolicy::deny_all(),
            2,
        );
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();
        for index in 0..6 {
            let runtime = runtime.clone();
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            workers.push(std::thread::spawn(move || {
                runtime
                    .cached_provider_query(format!("query:{index}"), || {
                        let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok(serde_json::Value::Null)
                    })
                    .unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn mock_server_fetch_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Hello\n\nWorld"))
            .mount(&server)
            .await;

        let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
        let rt = RemoteFetchRuntime::with_policy(policy);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();

        rt.register_and_fetch(url.clone());

        // Give the async task time to complete.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let content = rt.get_content(&url).unwrap().unwrap();
        assert!(content.contains("# Hello"));
        assert!(content.contains("World"));

        let stats = rt.stats();
        assert_eq!(stats.fetched, 1);
        assert_eq!(stats.failures, 0);
    }

    #[tokio::test]
    async fn mock_server_duplicate_url_fetches_once() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/shared.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("shared content"))
            .mount(&server)
            .await;

        let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
        let rt = RemoteFetchRuntime::with_policy(policy);
        let url = Url::parse(&format!("{}/shared.md", server.uri())).unwrap();

        // Register the same URL twice — should only fetch once.
        rt.register_and_fetch(url.clone());
        rt.register_and_fetch(url.clone());

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let content = rt.get_content(&url).unwrap().unwrap();
        assert_eq!(content, "shared content");

        let stats = rt.stats();
        assert_eq!(stats.fetched, 1);
    }

    #[tokio::test]
    async fn mock_server_http_error_fills_failed_slot() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing.md"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
        let rt = RemoteFetchRuntime::with_policy(policy);
        let url = Url::parse(&format!("{}/missing.md", server.uri())).unwrap();

        rt.register_and_fetch(url.clone());

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let result = rt.get_content(&url);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("404"));

        let stats = rt.stats();
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.fetched, 0);
    }

    #[tokio::test]
    async fn mock_server_policy_denial_no_network_request() {
        let server = MockServer::start().await;
        // Do NOT mount a mock — if the request reaches the server, the test
        // fails because wiremock will return a 500 for unmatched requests.

        let rt = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let url = Url::parse(&format!("{}/blocked.md", server.uri())).unwrap();

        rt.register_and_fetch(url.clone());

        let result = rt.get_content(&url);
        assert!(result.is_err());

        let stats = rt.stats();
        assert_eq!(stats.policy_denials, 1);
        assert_eq!(stats.fetched, 0);
        assert_eq!(stats.failures, 0);
    }

    #[tokio::test]
    async fn concurrency_cap_serializes_but_completes_all_fetches() {
        let server = MockServer::start().await;
        for name in ["a.md", "b.md", "c.md"] {
            Mock::given(method("GET"))
                .and(path(format!("/{name}")))
                .respond_with(ResponseTemplate::new(200).set_body_string(name))
                .mount(&server)
                .await;
        }

        // A cap of 1 forces the fetches through the semaphore one at a time;
        // all three must still complete.
        let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
        let rt = RemoteFetchRuntime::with_policy_and_concurrency(policy, 1);
        let urls: Vec<Url> = ["a.md", "b.md", "c.md"]
            .iter()
            .map(|n| Url::parse(&format!("{}/{n}", server.uri())).unwrap())
            .collect();
        for url in &urls {
            rt.register_and_fetch(url.clone());
        }

        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        for url in &urls {
            assert!(rt.get_content(url).unwrap().is_some());
        }
        assert_eq!(rt.stats().fetched, 3);
    }

    #[tokio::test]
    async fn concurrency_cap_bounds_in_flight_fetches() {
        let server = MockServer::start().await;
        // Each response is delayed so that, absent a cap, all six fetches would
        // be in flight at once. The delay forces real overlap to test against.
        for i in 0..6 {
            Mock::given(method("GET"))
                .and(path(format!("/f{i}.md")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string("body")
                        .set_delay(std::time::Duration::from_millis(200)),
                )
                .mount(&server)
                .await;
        }

        let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
        let rt = RemoteFetchRuntime::with_policy_and_concurrency(policy, 2);
        let urls: Vec<Url> = (0..6)
            .map(|i| Url::parse(&format!("{}/f{i}.md", server.uri())).unwrap())
            .collect();
        for url in &urls {
            rt.register_and_fetch(url.clone());
        }

        // Three batches of two at 200ms each ≈ 600ms; wait generously to drain.
        tokio::time::sleep(std::time::Duration::from_millis(1400)).await;

        for url in &urls {
            assert!(rt.get_content(url).unwrap().is_some());
        }
        assert_eq!(rt.stats().fetched, 6);
        // The semaphore bounds spawned work: at most two fetches run at once,
        // and the delay guarantees two genuinely overlapped.
        assert_eq!(
            rt.peak_in_flight(),
            2,
            "peak in-flight must equal the cap of 2"
        );
    }
}

#[cfg(test)]
mod persistent_cache_tests {
    use super::*;
    use crate::markdown::compose::remote::RemoteFreshnessMode;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use crate::markdown::compose::cache::manifest::CACHE_VERSION;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

    const SETTLE: Duration = Duration::from_millis(250);

    /// Builds an `Arc<FileStore>` rooted at a temp dir's `cache/v1`.
    fn temp_store(dir: &tempfile::TempDir) -> Arc<FileStore> {
        Arc::new(FileStore::at(dir.path().join("cache/v1")))
    }

    fn config(
        ttl: Option<Duration>,
        refresh: bool,
        mode: RemoteFreshnessMode,
    ) -> RemoteCacheConfig {
        RemoteCacheConfig {
            ttl_override: ttl,
            refresh,
            mode,
        }
    }

    fn runtime(store: Arc<FileStore>, cfg: RemoteCacheConfig) -> RemoteFetchRuntime {
        let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
        RemoteFetchRuntime::with_policy_store_and_config(policy, Some(store), cfg)
    }

    /// Responds `304` when the request carries the matching `If-None-Match`,
    /// otherwise serves the body with that ETag.
    struct Conditional304 {
        etag: String,
        body: String,
    }
    impl Respond for Conditional304 {
        fn respond(&self, req: &wiremock::Request) -> ResponseTemplate {
            let matches = req
                .headers
                .get("If-None-Match")
                .and_then(|v| v.to_str().ok())
                .map(|v| v == self.etag)
                .unwrap_or(false);
            if matches {
                ResponseTemplate::new(304).insert_header("ETag", self.etag.as_str())
            } else {
                ResponseTemplate::new(200)
                    .insert_header("ETag", self.etag.as_str())
                    .set_body_string(self.body.clone())
            }
        }
    }

    /// Serves the original body for an unconditional GET and a *new* body when
    /// a conditional `If-None-Match` revalidation arrives.
    struct Conditional200 {
        original_etag: String,
        original_body: String,
        revalidated_body: String,
        revalidated_etag: String,
    }
    impl Respond for Conditional200 {
        fn respond(&self, req: &wiremock::Request) -> ResponseTemplate {
            let conditional = req.headers.contains_key("If-None-Match");
            if conditional {
                ResponseTemplate::new(200)
                    .insert_header("ETag", self.revalidated_etag.as_str())
                    .set_body_string(self.revalidated_body.clone())
            } else {
                ResponseTemplate::new(200)
                    .insert_header("ETag", self.original_etag.as_str())
                    .set_body_string(self.original_body.clone())
            }
        }
    }

    #[tokio::test]
    async fn cache_hit_within_ttl_skips_network() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("v1"))
            // Exactly one request may reach the server across both runs.
            .expect(1)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let cfg = config(Some(Duration::from_secs(3600)), false, RemoteFreshnessMode::Strict);

        // First run: populates the cache from the network.
        let rt1 = runtime(Arc::clone(&store), cfg);
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("v1"));
        assert_eq!(rt1.stats().fetched, 1);

        // Second run: within TTL, served from disk without any network call.
        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("v1"));
        let stats = rt2.stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.fetched, 0);

        // Dropping the server verifies the `.expect(1)` request count.
    }

    /// The store's root does not exist until the first remote body is
    /// written, and that write creates it.
    #[tokio::test]
    async fn first_remote_write_creates_the_missing_store_root() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("v1"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let root = dir.path().join("cache/v1");
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let rt = runtime(store, config(Some(Duration::from_secs(3600)), false, RemoteFreshnessMode::Strict));
        assert!(!root.exists(), "building the runtime created the store root");

        rt.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;

        assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("v1"));
        assert!(root.join("manifests").join("remote").is_dir());
        assert!(root.join("blobs").is_dir());
    }

    /// An unusable store root is not an error: the fetch serves the network
    /// body, nothing is cached, and the offending file is untouched.
    #[tokio::test]
    async fn store_root_under_a_file_stays_network_only() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("v1"))
            // Both runs must reach the network: nothing could be cached.
            .expect(2)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-directory");
        std::fs::write(&file, "plain").unwrap();
        let store = Arc::new(FileStore::at(file.join("cache/v1")));
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let cfg = config(Some(Duration::from_secs(3600)), false, RemoteFreshnessMode::Strict);

        for _ in 0..2 {
            let rt = runtime(Arc::clone(&store), cfg);
            rt.register_and_fetch(url.clone());
            tokio::time::sleep(SETTLE).await;
            assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("v1"));
            let stats = rt.stats();
            assert_eq!((stats.fetched, stats.failures), (1, 0));
            // The failed write is reported, once, without touching a counter.
            assert_eq!(stats.cache_warnings.len(), 1, "{:?}", stats.cache_warnings);
            assert!(
                stats.cache_warnings[0].starts_with("failed to write remote cache entry"),
                "{:?}",
                stats.cache_warnings
            );
        }
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "plain");
    }

    #[tokio::test]
    async fn no_network_cached_run_after_server_gone() {
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);

        let url = {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/doc.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string("cached-body"))
                .mount(&server)
                .await;
            let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
            let rt = runtime(
                Arc::clone(&store),
                config(Some(Duration::from_secs(3600)), false, RemoteFreshnessMode::Optimistic),
            );
            rt.register_and_fetch(url.clone());
            tokio::time::sleep(SETTLE).await;
            assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("cached-body"));
            url
            // Server shuts down here.
        };

        // Optimistic mode must serve the cached body without touching the
        // (now-dead) server.
        let rt = runtime(
            Arc::clone(&store),
            config(None, false, RemoteFreshnessMode::Optimistic),
        );
        rt.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("cached-body"));
        assert_eq!(rt.stats().cache_hits, 1);
        assert_eq!(rt.stats().failures, 0);
    }

    #[tokio::test]
    async fn conditional_304_preserves_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(Conditional304 {
                etag: "etag-1".to_string(),
                body: "v1".to_string(),
            })
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        // TTL zero forces revalidation on the second run.
        let cfg = config(Some(Duration::ZERO), false, RemoteFreshnessMode::Strict);

        let rt1 = runtime(Arc::clone(&store), cfg);
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("v1"));

        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("v1"));
        let stats = rt2.stats();
        assert_eq!(stats.not_modified, 1);
        assert_eq!(stats.revalidations, 1);
        assert_eq!(stats.fetched, 0);
    }

    #[tokio::test]
    async fn conditional_200_replaces_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(Conditional200 {
                original_etag: "e1".to_string(),
                original_body: "v1".to_string(),
                revalidated_body: "v2".to_string(),
                revalidated_etag: "e2".to_string(),
            })
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let cfg = config(Some(Duration::ZERO), false, RemoteFreshnessMode::Strict);

        let rt1 = runtime(Arc::clone(&store), cfg);
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("v1"));

        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("v2"));
        let stats = rt2.stats();
        assert_eq!(stats.fetched, 1);
        assert_eq!(stats.revalidations, 1);
    }

    #[tokio::test]
    async fn refresh_forces_revalidation_when_fresh() {
        let server = MockServer::start().await;
        let hits = Arc::new(AtomicUsize::new(0));
        struct Counting {
            hits: Arc<AtomicUsize>,
        }
        impl Respond for Counting {
            fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
                self.hits.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200)
                    .insert_header("ETag", "x")
                    .set_body_string("v1")
            }
        }
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(Counting {
                hits: Arc::clone(&hits),
            })
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();

        // Populate cache with a long TTL (would otherwise be served from disk).
        let rt1 = runtime(
            Arc::clone(&store),
            config(Some(Duration::from_secs(3600)), false, RemoteFreshnessMode::Strict),
        );
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("v1"));

        // refresh = true must revalidate even though the entry is fresh.
        let rt2 = runtime(
            Arc::clone(&store),
            config(Some(Duration::from_secs(3600)), true, RemoteFreshnessMode::Strict),
        );
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("v1"));
        assert_eq!(rt2.stats().revalidations, 1);
        // Two network hits total: initial fetch + forced revalidation.
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    /// Serves the body with an ETag on the initial (unconditional) GET, then
    /// fails with `500` on any conditional revalidation request.
    struct FailOnRevalidate {
        etag: String,
        body: String,
    }
    impl Respond for FailOnRevalidate {
        fn respond(&self, req: &wiremock::Request) -> ResponseTemplate {
            if req.headers.contains_key("If-None-Match") {
                ResponseTemplate::new(500)
            } else {
                ResponseTemplate::new(200)
                    .insert_header("ETag", self.etag.as_str())
                    .set_body_string(self.body.clone())
            }
        }
    }

    #[tokio::test]
    async fn network_failure_serves_stale_under_fallback() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(FailOnRevalidate {
                etag: "e1".to_string(),
                body: "stale-but-usable".to_string(),
            })
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let cfg = config(Some(Duration::ZERO), false, RemoteFreshnessMode::Fallback);

        // Populate the cache.
        let rt1 = runtime(Arc::clone(&store), cfg);
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("stale-but-usable"));

        // Revalidation returns 500; Fallback serves the stale cached body.
        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("stale-but-usable"));
        assert_eq!(rt2.stats().stale_served, 1);
    }

    #[tokio::test]
    async fn default_mode_serves_stale_on_failure() {
        // The shipped default freshness mode is stale-on-failure, per spec.
        // This test pins that contract through `RemoteFreshnessMode::default()`
        // rather than a hard-coded `Fallback`, so a future default flip fails
        // here loudly instead of silently regressing offline/CI resilience.
        assert_eq!(RemoteFreshnessMode::default(), RemoteFreshnessMode::Fallback);

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(FailOnRevalidate {
                etag: "e1".to_string(),
                body: "stale-but-usable".to_string(),
            })
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        // TTL zero forces revalidation; mode comes from the shipped default.
        let cfg = config(Some(Duration::ZERO), false, RemoteFreshnessMode::default());

        let rt1 = runtime(Arc::clone(&store), cfg);
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("stale-but-usable"));

        // Revalidation returns 500; the default mode serves the stale body.
        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("stale-but-usable"));
        assert_eq!(rt2.stats().stale_served, 1);
    }

    #[tokio::test]
    async fn strict_failure_without_fallback_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(FailOnRevalidate {
                etag: "e1".to_string(),
                body: "v1".to_string(),
            })
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let cfg = config(Some(Duration::ZERO), false, RemoteFreshnessMode::Strict);

        let rt1 = runtime(Arc::clone(&store), cfg);
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("v1"));

        // Revalidation returns 500; Strict must surface the failure.
        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert!(rt2.get_content(&url).is_err());
        assert_eq!(rt2.stats().failures, 1);
        assert_eq!(rt2.stats().stale_served, 0);
    }

    #[tokio::test]
    async fn optimistic_serves_stale_cache_without_revalidation() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("v1"))
            // Only the initial population may reach the server.
            .expect(1)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        // TTL zero => entry is immediately stale, but Optimistic ignores TTL.
        let cfg = config(Some(Duration::ZERO), false, RemoteFreshnessMode::Optimistic);

        let rt1 = runtime(Arc::clone(&store), cfg);
        rt1.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("v1"));

        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("v1"));
        let stats = rt2.stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.revalidations, 0);
    }

    // ── RFC 9111 `no-store` / `no-cache` (acceptance criterion 4) ──────

    const FRESH_TTL: Duration = Duration::from_secs(3600);
    const MODES: [RemoteFreshnessMode; 3] = [
        RemoteFreshnessMode::Strict,
        RemoteFreshnessMode::Fallback,
        RemoteFreshnessMode::Optimistic,
    ];

    /// Every regular file under `root`; empty when `root` does not exist.
    fn files_under(root: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let Ok(entries) = std::fs::read_dir(root) else {
            return files;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(files_under(&path));
            } else {
                files.push(path);
            }
        }
        files
    }

    /// Writes a fresh-by-TTL entry for `url`, as a cache that ignored
    /// `Cache-Control` would have.
    fn seed_entry(store: &FileStore, url: &Url, body: &str, cache_control: &str) {
        seed_manifest(store, url, body, seed_manifest_json(url, body, cache_control));
    }

    /// Like [`seed_entry`], but in the `CACHE_VERSION` 1 shape: the cleartext
    /// request URL under `url` and no `redacted_url`.
    fn seed_v1_entry(store: &FileStore, url: &Url, body: &str, cache_control: &str) {
        let mut manifest = seed_manifest_json(url, body, cache_control);
        let fields = manifest.as_object_mut().unwrap();
        fields.remove("redacted_url");
        fields.insert("url".into(), serde_json::Value::String(url.to_string()));
        fields.insert("cache_version".into(), 1.into());
        seed_manifest(store, url, body, manifest);
    }

    fn seed_manifest_json(url: &Url, body: &str, cache_control: &str) -> serde_json::Value {
        use super::super::cache::manifest::{RemoteUrlManifest, redact_url};
        let content_hash = biscuit_hash::xx_hash(body);
        let now = std::time::SystemTime::now();
        serde_json::to_value(RemoteUrlManifest {
            cache_version: CACHE_VERSION,
            redacted_url: redact_url(url),
            source_id_hash: biscuit_hash::xx_hash(url.as_str()),
            status: 200,
            etag: None,
            last_modified: None,
            cache_control: Some(cache_control.to_string()),
            fetched_at: now,
            expires_at: Some(now + FRESH_TTL),
            content_hash,
            body_blob_hash: content_hash,
            size_bytes: body.len() as u64,
        })
        .unwrap()
    }

    fn seed_manifest(store: &FileStore, url: &Url, body: &str, manifest: serde_json::Value) {
        use super::super::cache::types::ArtifactClass;
        let content_hash = biscuit_hash::xx_hash(body);
        store
            .write_artifact(
                ArtifactClass::RemoteUrl,
                biscuit_hash::xx_hash(url.as_str()),
                &manifest,
                body.as_bytes(),
                content_hash,
                "remote",
            )
            .unwrap();
    }

    /// Fetches `url` once through a fresh runtime and returns it for stats.
    fn run_once(store: &Arc<FileStore>, cfg: RemoteCacheConfig, url: &Url) -> RemoteFetchRuntime {
        let rt = runtime(Arc::clone(store), cfg);
        rt.register_and_fetch(url.clone());
        rt
    }

    /// Serves a numbered body (`body-1`, `body-2`, ...) with `cache_control`.
    async fn numbered_server(cache_control: &'static str, expected: u64) -> MockServer {
        numbered_server_with_lines(&[cache_control], expected).await
    }

    /// Like [`numbered_server`], but sends each entry of `lines` as its own
    /// `Cache-Control` field line.
    async fn numbered_server_with_lines(lines: &[&'static str], expected: u64) -> MockServer {
        let server = MockServer::start().await;
        let hits = AtomicUsize::new(0);
        let lines = lines.to_vec();
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(move |_: &wiremock::Request| {
                let n = hits.fetch_add(1, Ordering::SeqCst) + 1;
                lines
                    .iter()
                    .fold(ResponseTemplate::new(200), |response, line| {
                        response.append_header("Cache-Control", *line)
                    })
                    .set_body_string(format!("body-{n}"))
            })
            .expect(expected)
            .mount(&server)
            .await;
        server
    }

    /// `no-store` writes neither manifest nor blob under any freshness mode or
    /// TTL override, so a second run reaches the network again. The mixed
    /// `max-age=3600, no-store` value is the original defect: first-match
    /// parsing read it as a one-hour lifetime.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_store_response_is_never_stored_under_any_mode_or_ttl() {
        let mut cases: Vec<(&'static str, RemoteFreshnessMode, Option<Duration>)> = Vec::new();
        for mode in MODES {
            for ttl in [None, Some(FRESH_TTL)] {
                cases.push(("max-age=3600, no-store", mode, ttl));
            }
        }
        for header in ["no-store", "Private, No-Store", "no-cache, no-store"] {
            cases.push((header, RemoteFreshnessMode::Optimistic, Some(FRESH_TTL)));
        }

        for (header, mode, ttl) in cases {
            let label = format!("{header:?} / {mode:?} / ttl {ttl:?}");
            let server = numbered_server(header, 2).await;
            let dir = tempfile::tempdir().unwrap();
            let store = temp_store(&dir);
            let root = dir.path().join("cache/v1");
            let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
            let cfg = config(ttl, false, mode);

            let rt1 = run_once(&store, cfg, &url);
            assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("body-1"), "{label}");
            assert!(files_under(&root).is_empty(), "{label}: stored {:?}", files_under(&root));

            let rt2 = run_once(&store, cfg, &url);
            assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("body-2"), "{label}");
            let stats = rt2.stats();
            assert_eq!((stats.fetched, stats.cache_hits), (1, 0), "{label}");
            assert!(stats.cache_warnings.is_empty(), "{label}: {:?}", stats.cache_warnings);
            assert!(files_under(&root).is_empty(), "{label}");
            // Dropping the server verifies `.expect(2)`.
        }
    }

    /// `no-store` on a second `Cache-Control` field line is as binding as on
    /// the first: nothing is written under any freshness mode or TTL override,
    /// and the second run reaches the network again. `biscuit_file`'s
    /// `fetch_integration` pins that `append_header` puts two lines on the wire.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn split_line_no_store_is_never_stored_under_any_mode_or_ttl() {
        for mode in MODES {
            for ttl in [None, Some(FRESH_TTL)] {
                let label = format!("{mode:?} / ttl {ttl:?}");
                let server = numbered_server_with_lines(&["max-age=3600", "no-store"], 2).await;
                let dir = tempfile::tempdir().unwrap();
                let store = temp_store(&dir);
                let root = dir.path().join("cache/v1");
                let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
                let cfg = config(ttl, false, mode);

                let rt1 = run_once(&store, cfg, &url);
                assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("body-1"), "{label}");
                assert!(files_under(&root).is_empty(), "{label}: stored {:?}", files_under(&root));

                let rt2 = run_once(&store, cfg, &url);
                assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("body-2"), "{label}");
                let stats = rt2.stats();
                assert_eq!((stats.fetched, stats.cache_hits), (1, 0), "{label}");
                assert!(stats.cache_warnings.is_empty(), "{label}: {:?}", stats.cache_warnings);
                assert!(files_under(&root).is_empty(), "{label}");
                // Dropping the server verifies `.expect(2)`.
            }
        }
    }

    /// A `no-store` entry already on disk, fresh by TTL, is never served under
    /// any mode: it is purged and the network answers instead.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pre_existing_no_store_entry_is_purged_not_served() {
        for mode in MODES {
            let server = numbered_server("no-store", 1).await;
            let dir = tempfile::tempdir().unwrap();
            let store = temp_store(&dir);
            let root = dir.path().join("cache/v1");
            let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
            seed_entry(&store, &url, "old-secret", "max-age=3600, no-store");
            assert_eq!(files_under(&root).len(), 2);

            let rt = run_once(&store, config(Some(FRESH_TTL), false, mode), &url);
            assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"), "{mode:?}");
            let stats = rt.stats();
            assert_eq!((stats.fetched, stats.cache_hits, stats.failures), (1, 0, 0), "{mode:?}");
            assert!(stats.cache_warnings.is_empty(), "{mode:?}: {:?}", stats.cache_warnings);
            assert!(files_under(&root).is_empty(), "{mode:?}: {:?}", files_under(&root));
        }
    }

    /// Acceptance criterion 5 at the runtime seam: a fresh entry for a denied
    /// host is never read, under every mode, with and without a TTL override.
    /// `fetch_with_cache` reads the store before the policy-enforcing client
    /// runs, so the early `check_allowed` in `register_and_fetch` is the only
    /// thing keeping cached bytes from a denied host.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn denied_host_never_reads_a_fresh_cached_entry() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("network"))
            .expect(0)
            .mount(&server)
            .await;
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let tree = |root: &std::path::Path| {
            let mut files: Vec<_> = files_under(root)
                .into_iter()
                .map(|path| {
                    let bytes = std::fs::read(&path).unwrap();
                    (path, bytes)
                })
                .collect();
            files.sort();
            files
        };

        for mode in MODES {
            for ttl in [None, Some(FRESH_TTL)] {
                let dir = tempfile::tempdir().unwrap();
                let store = temp_store(&dir);
                let root = dir.path().join("cache/v1");
                seed_entry(&store, &url, "seeded", "max-age=3600");
                let before = tree(&root);
                assert_eq!(before.len(), 2);
                let cfg = config(ttl, false, mode);

                let denied = RemoteFetchRuntime::with_policy_store_and_config(
                    FetchPolicy::deny_all(),
                    Some(Arc::clone(&store)),
                    cfg,
                );
                denied.register_and_fetch(url.clone());
                let error = denied.get_content(&url).unwrap_err();
                assert!(error.contains("127.0.0.1"), "{mode:?}/{ttl:?}: {error}");
                let stats = denied.stats();
                assert_eq!(
                    (stats.policy_denials, stats.cache_hits, stats.stale_served, stats.fetched),
                    (1, 0, 0, 0),
                    "{mode:?}/{ttl:?}"
                );
                assert_eq!(tree(&root), before, "{mode:?}/{ttl:?}: the seeded entry changed");

                // Control: the same entry is served to an allowed host, so the
                // denial above is what kept it out.
                let allowed = run_once(&store, cfg, &url);
                assert_eq!(allowed.get_content(&url).unwrap().as_deref(), Some("seeded"));
                assert_eq!(allowed.stats().cache_hits, 1, "{mode:?}/{ttl:?}");
            }
        }
        // Dropping the server verifies that no request reached it.
    }

    /// When the fresh response is storable it replaces the purged entry, and
    /// the old `no-store` body does not survive beside it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pre_existing_no_store_entry_is_replaced_by_a_storable_response() {
        let server = numbered_server("max-age=60", 1).await;
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let root = dir.path().join("cache/v1");
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        seed_entry(&store, &url, "old-secret", "no-store");

        let rt = run_once(&store, config(None, false, RemoteFreshnessMode::Optimistic), &url);
        assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"));

        let files = files_under(&root);
        assert_eq!(files.len(), 2, "{files:?}");
        for file in files {
            let bytes = std::fs::read_to_string(&file).unwrap();
            assert!(!bytes.contains("old-secret"), "{} kept the no-store body", file.display());
        }
    }

    /// A purge that cannot remove the body is non-fatal: the fetch succeeds
    /// and the failure is a cache warning. A directory standing where the blob
    /// file should be makes the removal fail on every platform.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_store_purge_failure_is_a_warning_not_an_error() {
        let server = numbered_server("no-store", 1).await;
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let root = dir.path().join("cache/v1");
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        seed_entry(&store, &url, "old-secret", "no-store");
        let blob = files_under(&root)
            .into_iter()
            .find(|path| path.extension().is_some_and(|ext| ext == "remote"))
            .unwrap();
        std::fs::remove_file(&blob).unwrap();
        std::fs::create_dir_all(blob.join("occupied")).unwrap();

        let rt = run_once(&store, config(None, false, RemoteFreshnessMode::Fallback), &url);
        assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"));
        let stats = rt.stats();
        assert_eq!((stats.fetched, stats.failures), (1, 0));
        assert_eq!(stats.cache_warnings.len(), 1, "{:?}", stats.cache_warnings);
        assert!(
            stats.cache_warnings[0].starts_with("failed to remove no-store remote cache body"),
            "{:?}",
            stats.cache_warnings
        );
        // The manifest was still removed, so the entry can never be served.
        assert!(
            files_under(&root).iter().all(|path| !path.to_string_lossy().ends_with(".json")),
            "{:?}",
            files_under(&root)
        );
    }

    // ── Manifest privacy (acceptance criterion 6) ──────────────────────

    /// Every remote manifest under `root`, parsed.
    fn manifests_under(root: &std::path::Path) -> Vec<serde_json::Value> {
        files_under(root)
            .into_iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .map(|path| serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap())
            .collect()
    }

    /// A URL carrying credentials and a query token is fetched and cached,
    /// then served from cache, and no persisted byte ever holds the secrets.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn manifest_never_persists_url_userinfo_or_query() {
        let server = numbered_server("max-age=3600", 1).await;
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let root = dir.path().join("cache/v1");
        let port = server.address().port();
        let url =
            Url::parse(&format!("http://user:secret@127.0.0.1:{port}/doc.md?token=abc")).unwrap();

        for (run, expected) in [(1, (1, 0)), (2, (0, 1))] {
            let rt = run_once(&store, config(None, false, RemoteFreshnessMode::Strict), &url);
            assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"), "run {run}");
            let stats = rt.stats();
            assert_eq!((stats.fetched, stats.cache_hits), expected, "run {run}");

            let files = files_under(&root);
            assert_eq!(files.len(), 2, "run {run}: {files:?}");
            for file in &files {
                let bytes = std::fs::read(file).unwrap();
                let text = String::from_utf8_lossy(&bytes);
                for secret in ["user", "secret", "token", "abc"] {
                    assert!(!text.contains(secret), "{} holds {secret:?}", file.display());
                }
            }
            let manifests = manifests_under(&root);
            assert_eq!(
                manifests[0]["redacted_url"],
                format!("http://127.0.0.1:{port}/doc.md?<redacted>"),
                "run {run}"
            );
            assert_eq!(manifests[0]["cache_version"], CACHE_VERSION, "run {run}");
            assert_eq!(manifests[0]["source_id_hash"], biscuit_hash::xx_hash(url.as_str()));
        }
    }

    /// Redaction applies to the manifest, never to the key: URLs differing
    /// only in query or userinfo keep separate entries and bodies, even though
    /// their redacted forms are identical.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn urls_differing_only_in_query_or_userinfo_never_share_an_entry() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(|req: &wiremock::Request| {
                let auth = req
                    .headers
                    .get("Authorization")
                    .map(|value| value.to_str().unwrap().to_string())
                    .unwrap_or_default();
                let query = req.url.query().unwrap_or_default().to_string();
                ResponseTemplate::new(200)
                    .insert_header("Cache-Control", "max-age=3600")
                    .set_body_string(format!("query={query} auth={auth}"))
            })
            .expect(4)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let root = dir.path().join("cache/v1");
        let port = server.address().port();
        let urls: Vec<Url> = [
            format!("http://127.0.0.1:{port}/doc.md?v=1"),
            format!("http://127.0.0.1:{port}/doc.md?v=2"),
            format!("http://a:x@127.0.0.1:{port}/doc.md?v=1"),
            format!("http://b:y@127.0.0.1:{port}/doc.md?v=1"),
        ]
        .iter()
        .map(|raw| Url::parse(raw).unwrap())
        .collect();
        let optimistic = config(None, false, RemoteFreshnessMode::Optimistic);

        let mut cold = Vec::new();
        for url in &urls {
            let rt = run_once(&store, optimistic, url);
            cold.push(rt.get_content(url).unwrap().unwrap());
        }
        let distinct: std::collections::BTreeSet<_> = cold.iter().collect();
        assert_eq!(distinct.len(), urls.len(), "{cold:?}");

        let manifests = manifests_under(&root);
        assert_eq!(manifests.len(), urls.len());
        let keys: std::collections::BTreeSet<_> =
            manifests.iter().map(|m| m["source_id_hash"].as_u64().unwrap()).collect();
        assert_eq!(keys.len(), urls.len());
        let redacted: std::collections::BTreeSet<_> =
            manifests.iter().map(|m| m["redacted_url"].as_str().unwrap()).collect();
        assert_eq!(
            redacted.into_iter().collect::<Vec<_>>(),
            [format!("http://127.0.0.1:{port}/doc.md?<redacted>")]
        );

        // Warm: each URL is served its own body from cache (the mock's
        // `expect(4)` fails the test if any warm read reaches the network).
        for (url, body) in urls.iter().zip(&cold) {
            let rt = run_once(&store, optimistic, url);
            assert_eq!(rt.get_content(url).unwrap().as_ref(), Some(body), "{url}");
            assert_eq!(rt.stats().cache_hits, 1, "{url}");
        }
    }

    /// Like [`seed_entry`], but recording a `cache_version` this build does
    /// not write, as a newer darkmatter sharing the cache root would.
    fn seed_foreign_version_entry(store: &FileStore, url: &Url, body: &str, cache_control: &str) {
        let mut manifest = seed_manifest_json(url, body, cache_control);
        manifest["cache_version"] = (CACHE_VERSION + 1).into();
        seed_manifest(store, url, body, manifest);
    }

    /// An entry from another `CACHE_VERSION`, however fresh, is a miss under
    /// every mode — whether its shape no longer parses (v1) or still does (a
    /// newer version). When the new response is `no-store` nothing replaces
    /// it, and it is left on disk byte-for-byte.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn other_version_entry_is_a_miss_and_left_in_place() {
        type Seeder = fn(&FileStore, &Url, &str, &str);
        let seeders: [(&str, Seeder); 2] =
            [("v1", seed_v1_entry), ("newer", seed_foreign_version_entry)];
        for ((version, seed), mode) in seeders
            .into_iter()
            .flat_map(|seeder| MODES.map(|mode| (seeder, mode)))
        {
            let server = numbered_server("no-store", 1).await;
            let dir = tempfile::tempdir().unwrap();
            let store = temp_store(&dir);
            let root = dir.path().join("cache/v1");
            let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
            seed(&store, &url, "legacy-body", "max-age=3600");
            let snapshot = || -> Vec<_> {
                files_under(&root)
                    .into_iter()
                    .map(|file| (file.clone(), std::fs::read(&file).unwrap()))
                    .collect()
            };
            let before = snapshot();
            assert_eq!(before.len(), 2);

            let rt = run_once(&store, config(Some(FRESH_TTL), false, mode), &url);
            let case = format!("{version} {mode:?}");
            assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"), "{case}");
            let stats = rt.stats();
            assert_eq!((stats.fetched, stats.cache_hits), (1, 0), "{case}");
            assert!(stats.cache_warnings.is_empty(), "{case}: {:?}", stats.cache_warnings);
            assert_eq!(snapshot(), before, "{case}");
        }
    }

    /// A storable response for a URL with a v1 entry overwrites that entry in
    /// the v2 shape, so the cleartext URL leaves the disk; the next run reads
    /// the v2 entry back from cache.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn legacy_v1_entry_is_replaced_by_a_redacted_v2_entry() {
        let server = numbered_server("max-age=3600", 1).await;
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let root = dir.path().join("cache/v1");
        let port = server.address().port();
        let url = Url::parse(&format!("http://user:secret@127.0.0.1:{port}/doc.md?token=abc"))
            .unwrap();
        seed_v1_entry(&store, &url, "legacy-body", "max-age=3600");
        assert_eq!(manifests_under(&root)[0]["url"], url.as_str());

        let optimistic = config(None, false, RemoteFreshnessMode::Optimistic);
        let rt = run_once(&store, optimistic, &url);
        assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"));
        assert_eq!(rt.stats().fetched, 1);

        let manifests = manifests_under(&root);
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0]["cache_version"], CACHE_VERSION);
        assert!(manifests[0].get("url").is_none(), "{:?}", manifests[0]);
        assert!(!manifests[0].to_string().contains("secret"));

        let rt = run_once(&store, optimistic, &url);
        assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"));
        assert_eq!((rt.stats().fetched, rt.stats().cache_hits), (0, 1));
    }

    /// A v1 entry recording `no-store` is purged even though its version is a
    /// miss: the version check must not hide it from the R-E purge.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn legacy_v1_no_store_entry_is_purged() {
        for mode in MODES {
            let server = numbered_server("no-store", 1).await;
            let dir = tempfile::tempdir().unwrap();
            let store = temp_store(&dir);
            let root = dir.path().join("cache/v1");
            let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
            seed_v1_entry(&store, &url, "old-secret", "max-age=3600, no-store");
            assert_eq!(files_under(&root).len(), 2);

            let rt = run_once(&store, config(Some(FRESH_TTL), false, mode), &url);
            assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("body-1"), "{mode:?}");
            let stats = rt.stats();
            assert_eq!((stats.fetched, stats.cache_hits), (1, 0), "{mode:?}");
            assert!(stats.cache_warnings.is_empty(), "{mode:?}: {:?}", stats.cache_warnings);
            assert!(files_under(&root).is_empty(), "{mode:?}: {:?}", files_under(&root));
        }
    }

    /// Serves `v1` with ETag `e1` and `cache_control`; a conditional request
    /// is counted and answered by `revalidated`.
    async fn conditional_server(
        cache_control: &'static str,
        revalidated: fn() -> ResponseTemplate,
        conditional_hits: Arc<AtomicUsize>,
    ) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(move |req: &wiremock::Request| {
                if req.headers.contains_key("If-None-Match") {
                    conditional_hits.fetch_add(1, Ordering::SeqCst);
                    revalidated()
                } else {
                    ResponseTemplate::new(200)
                        .insert_header("ETag", "e1")
                        .insert_header("Cache-Control", cache_control)
                        .set_body_string("v1")
                }
            })
            .mount(&server)
            .await;
        server
    }

    /// A stored `no-cache` entry is revalidated under every mode, including
    /// Optimistic, even with a TTL override that would call it fresh.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_cache_entry_is_revalidated_under_every_mode_despite_ttl() {
        for mode in MODES {
            let conditional_hits = Arc::new(AtomicUsize::new(0));
            let server = conditional_server(
                "no-cache",
                || {
                    ResponseTemplate::new(304)
                        .insert_header("ETag", "e1")
                        .insert_header("Cache-Control", "no-cache")
                },
                Arc::clone(&conditional_hits),
            )
            .await;
            let dir = tempfile::tempdir().unwrap();
            let store = temp_store(&dir);
            let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
            let cfg = config(Some(FRESH_TTL), false, mode);

            let rt1 = run_once(&store, cfg, &url);
            assert_eq!(rt1.get_content(&url).unwrap().as_deref(), Some("v1"), "{mode:?}");
            // no-cache is storable: the entry exists, it just is not reusable
            // without validation.
            assert_eq!(files_under(&dir.path().join("cache/v1")).len(), 2, "{mode:?}");

            for run in 1..=2 {
                let rt = run_once(&store, cfg, &url);
                assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("v1"), "{mode:?}");
                let stats = rt.stats();
                assert_eq!(
                    (stats.cache_hits, stats.revalidations, stats.not_modified),
                    (0, 1, 1),
                    "{mode:?} run {run}"
                );
                assert_eq!(conditional_hits.load(Ordering::SeqCst), run, "{mode:?}");
            }
        }
    }

    /// A `no-cache` revalidation that returns new content replaces the entry,
    /// even under Optimistic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_cache_revalidation_serves_changed_content() {
        let conditional_hits = Arc::new(AtomicUsize::new(0));
        let server = conditional_server(
            "no-cache",
            || {
                ResponseTemplate::new(200)
                    .insert_header("ETag", "e2")
                    .insert_header("Cache-Control", "no-cache")
                    .set_body_string("v2")
            },
            Arc::clone(&conditional_hits),
        )
        .await;
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let cfg = config(Some(FRESH_TTL), false, RemoteFreshnessMode::Optimistic);

        assert_eq!(run_once(&store, cfg, &url).get_content(&url).unwrap().as_deref(), Some("v1"));
        let rt = run_once(&store, cfg, &url);
        assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some("v2"));
        assert_eq!((rt.stats().fetched, rt.stats().revalidations), (1, 1));
        assert_eq!(conditional_hits.load(Ordering::SeqCst), 1);
    }

    /// R-G: `no-cache` outranks Fallback. A failed revalidation of a
    /// `no-cache` entry is an error, never a stale serve. The contrast case
    /// (a plain entry is served stale) is `network_failure_serves_stale_under_fallback`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fallback_does_not_serve_a_no_cache_entry_after_failed_revalidation() {
        let conditional_hits = Arc::new(AtomicUsize::new(0));
        let server = conditional_server(
            "max-age=3600, no-cache",
            || ResponseTemplate::new(500),
            Arc::clone(&conditional_hits),
        )
        .await;
        let dir = tempfile::tempdir().unwrap();
        let store = temp_store(&dir);
        let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
        let cfg = config(Some(FRESH_TTL), false, RemoteFreshnessMode::Fallback);

        assert_eq!(run_once(&store, cfg, &url).get_content(&url).unwrap().as_deref(), Some("v1"));
        let rt = run_once(&store, cfg, &url);
        assert!(rt.get_content(&url).is_err());
        let stats = rt.stats();
        assert_eq!((stats.failures, stats.stale_served, stats.cache_hits), (1, 0, 0));
        assert_eq!(conditional_hits.load(Ordering::SeqCst), 1);
    }

    /// A revalidation answered with `no-store` (as a `304` or a new `200`)
    /// stops the old entry from being refreshed or kept: it is purged.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn revalidation_that_turns_no_store_purges_the_entry() {
        type Revalidated = fn() -> ResponseTemplate;
        let cases: [(&str, Revalidated, &str); 2] = [
            (
                "304",
                || {
                    ResponseTemplate::new(304)
                        .insert_header("ETag", "e1")
                        .insert_header("Cache-Control", "no-store")
                },
                "v1",
            ),
            (
                "200",
                || {
                    ResponseTemplate::new(200)
                        .insert_header("Cache-Control", "no-store")
                        .set_body_string("v2")
                },
                "v2",
            ),
        ];
        for (label, revalidated, expected) in cases {
            let conditional_hits = Arc::new(AtomicUsize::new(0));
            let server =
                conditional_server("max-age=0", revalidated, Arc::clone(&conditional_hits)).await;
            let dir = tempfile::tempdir().unwrap();
            let store = temp_store(&dir);
            let root = dir.path().join("cache/v1");
            let url = Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
            let cfg = config(None, false, RemoteFreshnessMode::Strict);

            assert_eq!(run_once(&store, cfg, &url).get_content(&url).unwrap().as_deref(), Some("v1"));
            assert_eq!(files_under(&root).len(), 2, "{label}");

            let rt = run_once(&store, cfg, &url);
            assert_eq!(rt.get_content(&url).unwrap().as_deref(), Some(expected), "{label}");
            assert_eq!(conditional_hits.load(Ordering::SeqCst), 1, "{label}");
            assert!(files_under(&root).is_empty(), "{label}: {:?}", files_under(&root));
        }
    }

    #[test]
    fn cache_warnings_project_to_coded_compose_warnings() {
        let stats = RemoteFetchStats {
            cache_warnings: vec!["failed to write remote cache entry 00: denied".to_string()],
            ..RemoteFetchStats::default()
        };
        let warnings: Vec<ComposeWarning> = stats.cache_compose_warnings().collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].stage, "remote_cache");
        assert_eq!(warnings[0].code.as_deref(), Some("dm.remote_cache.io_failure"));
        assert_eq!(warnings[0].message, stats.cache_warnings[0]);
        assert!(RemoteFetchStats::default().cache_compose_warnings().next().is_none());
    }
}
