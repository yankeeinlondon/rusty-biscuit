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

use super::cache::FileStore;
use super::cache::remote_cache::{RemoteCacheConfig, RemoteOutcomeEvent, fetch_with_cache};
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
    /// Fetch completed successfully with the response body as UTF-8 text and
    /// the xxHash of that body (used for closure-hash dependency tracking).
    Ready { body: String, content_hash: u64 },
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

type ProviderQueryCache =
    Mutex<HashMap<String, Arc<OnceLock<Result<serde_json::Value, String>>>>>;

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
        for host in &config.allowed_hosts {
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
        query: impl FnOnce() -> Result<serde_json::Value, String>,
    ) -> Result<serde_json::Value, String> {
        let slot = {
            let mut queries = self
                .inner
                .provider_queries
                .lock()
                .map_err(|_| "provider query cache lock was poisoned".to_string())?;
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

    fn acquire_provider_permit(&self) -> Result<ProviderPermit<'_>, String> {
        let mut in_flight = self
            .inner
            .provider_in_flight
            .lock()
            .map_err(|_| "provider concurrency lock was poisoned".to_string())?;
        while *in_flight >= self.inner.concurrency {
            in_flight = self
                .inner
                .provider_notify
                .wait(in_flight)
                .map_err(|_| "provider concurrency lock was poisoned".to_string())?;
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
                    let result = match task_inner.client() {
                        Some(client) => {
                            fetch_with_cache(
                                task_inner.store.as_deref(),
                                client,
                                &url,
                                &task_inner.policy,
                                &task_inner.cache_config,
                            )
                            .await
                        }
                        None => Err("failed to initialize remote fetch client".to_string()),
                    };
                    task_inner.in_flight.fetch_sub(1, Ordering::SeqCst);

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
                            *guard = FetchSlot::Ready {
                                body: outcome.body,
                                content_hash: outcome.content_hash,
                            };
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
                FetchSlot::Ready { body, .. } => return Ok(Some(body.clone())),
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

    /// Returns the xxHash of a URL's fetched body, blocking until the fetch
    /// completes.
    ///
    /// Used to feed remote dependency changes into a parent document's
    /// closure hash. Returns `None` if the URL was never registered or its
    /// fetch failed.
    pub fn content_hash(&self, url: &Url) -> Option<u64> {
        let key = url.to_string();
        let slot = self.inner.slots.get(&key)?;

        let mut guard = slot.state.lock().unwrap();
        loop {
            match &*guard {
                FetchSlot::Ready { content_hash, .. } => return Some(*content_hash),
                FetchSlot::Failed(_) => return None,
                FetchSlot::InFlight => {
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
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

    const SETTLE: Duration = Duration::from_millis(250);

    /// Builds an `Arc<FileStore>` rooted at a temp dir's `cache/v1`.
    fn temp_store(dir: &tempfile::TempDir) -> Arc<FileStore> {
        Arc::new(FileStore::new(dir.path().join("cache/v1")).unwrap())
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
        let hash1 = rt1.content_hash(&url).unwrap();

        let rt2 = runtime(Arc::clone(&store), cfg);
        rt2.register_and_fetch(url.clone());
        tokio::time::sleep(SETTLE).await;
        assert_eq!(rt2.get_content(&url).unwrap().as_deref(), Some("v2"));
        let stats = rt2.stats();
        assert_eq!(stats.fetched, 1);
        assert_eq!(stats.revalidations, 1);
        // Closure-hash invalidation: changed remote body → changed content hash.
        let hash2 = rt2.content_hash(&url).unwrap();
        assert_ne!(hash1, hash2);
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
}
