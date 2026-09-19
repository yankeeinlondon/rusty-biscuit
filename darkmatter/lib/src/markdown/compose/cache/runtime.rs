//! Run-local in-memory cache with single-flight deduplication.
//!
//! `RunLocalCache` replaces the old `PipelineCache` and adds single-flight
//! behavior for child compose operations and individual operations (code
//! transclusion, TOC linking): when multiple rayon threads request the same
//! result simultaneously, only one computes while others wait.

use super::hashing::compose_cache_key;
use super::types::{CacheAccessMode, CacheStats};
use crate::markdown::Markdown;
use crate::markdown::compose::ComposeReport;
use crate::markdown::toc::MarkdownTocNode;
use crate::markdown::types::MarkdownResult;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// State of a single-flight slot for a cached result.
enum SlotState<T> {
    /// Computation is in progress — waiters should block on the condvar.
    InFlight,
    /// Computation completed successfully.
    Ready(Arc<T>),
    /// Computation failed — waiters should recompute or propagate.
    Failed(String),
}

/// A single-flight slot protected by a mutex and condvar.
struct SingleFlightSlot<T> {
    state: Mutex<SlotState<T>>,
    ready: Condvar,
}

/// The cached result of a child compose operation.
pub(crate) struct ComposeResult {
    /// The composed document content (post-compose, pre-parent-transforms).
    pub content: String,
    /// The compose report from the child pipeline.
    pub report: ComposeReport,
    /// Every runtime-context group read by a source in this subtree.
    pub context_groups: crate::markdown::compose::ContextRequirements,
}

/// The cached result of an individual operation (code transclusion, TOC linking).
pub(crate) struct OperationResult {
    /// The core operation output (before parent-specific wrappers).
    pub content: String,
}

/// Timeout for waiting on an in-flight computation before falling back
/// to duplicate computation (mitigates rayon deadlock risk).
const INFLIGHT_TIMEOUT: Duration = Duration::from_secs(30);

/// Run-local cache shared across all threads in a single compose invocation.
///
/// All fields are `Arc`-wrapped so `clone()` produces a shared view
/// (same backing data), matching the old `PipelineCache` clone semantics.
///
/// Memory-only by design: until a `ContentPolicy` defines freshness for
/// cached content, composed children, operation results, and document
/// snapshots are never persisted (more-context R18,
/// `fixes/2026-09-16-content-policy-no-cache`). The persistent semantic-result
/// path was removed rather than kept dormant, and
/// `lib/tests/semantic_results_never_persist.rs` fails if a store is
/// reattached here.
#[derive(Clone)]
pub(crate) struct RunLocalCache {
    markdown_documents: Arc<DashMap<String, Markdown>>,
    toc_headings: Arc<DashMap<String, Vec<MarkdownTocNode>>>,
    compose_results: Arc<DashMap<String, Arc<SingleFlightSlot<ComposeResult>>>>,
    operation_results: Arc<DashMap<String, Arc<SingleFlightSlot<OperationResult>>>>,
    stats: Arc<Mutex<CacheStats>>,
    access_mode: CacheAccessMode,
}

impl std::fmt::Debug for RunLocalCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunLocalCache")
            .field("access_mode", &self.access_mode)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl Default for RunLocalCache {
    fn default() -> Self {
        Self::new(CacheAccessMode::ReadWrite)
    }
}

impl RunLocalCache {
    /// Creates a new run-local cache with the given access mode.
    pub fn new(access_mode: CacheAccessMode) -> Self {
        Self {
            markdown_documents: Arc::new(DashMap::new()),
            toc_headings: Arc::new(DashMap::new()),
            compose_results: Arc::new(DashMap::new()),
            operation_results: Arc::new(DashMap::new()),
            stats: Arc::new(Mutex::new(CacheStats::default())),
            access_mode,
        }
    }

    /// Loads a markdown document, returning a cached copy if available.
    pub fn load_markdown(&self, path: &Path) -> MarkdownResult<Markdown> {
        let key = compose_cache_key(path);

        if self.access_mode != CacheAccessMode::Off
            && let Some(markdown) = self.markdown_documents.get(&key)
        {
            return Ok(markdown.clone());
        }

        let markdown = Markdown::try_from(path)?;

        if self.access_mode != CacheAccessMode::Off && self.access_mode != CacheAccessMode::ReadOnly
        {
            Ok(self
                .markdown_documents
                .entry(key)
                .or_insert_with(|| markdown.clone())
                .clone())
        } else {
            Ok(markdown)
        }
    }

    /// Loads TOC headings for a document, returning a cached copy if available.
    pub fn load_toc_headings(&self, path: &Path) -> std::io::Result<Vec<MarkdownTocNode>> {
        let key = compose_cache_key(path);

        if self.access_mode != CacheAccessMode::Off
            && let Some(headings) = self.toc_headings.get(&key)
        {
            return Ok(headings.clone());
        }

        let content = std::fs::read_to_string(path)?;
        let markdown: Markdown = content.into();
        let headings = markdown
            .toc()
            .all_headings()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        if self.access_mode != CacheAccessMode::Off && self.access_mode != CacheAccessMode::ReadOnly
        {
            Ok(self
                .toc_headings
                .entry(key)
                .or_insert_with(|| headings.clone())
                .clone())
        } else {
            Ok(headings)
        }
    }

    /// Single-flight get-or-compute for child compose results.
    ///
    /// If the key is already cached (Ready), returns the cached result.
    /// If another thread is computing (InFlight), waits up to 30s then
    /// falls back to duplicate computation to avoid rayon deadlock.
    /// If absent, marks InFlight, runs the closure, caches the result.
    pub fn get_or_compute_compose<F>(
        &self,
        key: &str,
        compute: F,
    ) -> MarkdownResult<Arc<ComposeResult>>
    where
        F: FnOnce() -> MarkdownResult<ComposeResult>,
    {
        // When caching is off, always compute directly
        if self.access_mode == CacheAccessMode::Off {
            let result = compute()?;
            return Ok(Arc::new(result));
        }

        // Check for existing slot
        let slot = if self.access_mode == CacheAccessMode::Refresh {
            let slot = Arc::new(SingleFlightSlot {
                state: Mutex::new(SlotState::InFlight),
                ready: Condvar::new(),
            });
            self.compose_results
                .insert(key.to_string(), Arc::clone(&slot));
            slot
        } else {
            match self.compose_results.entry(key.to_string()) {
                Entry::Occupied(mut entry) => {
                    let existing = Arc::clone(entry.get());
                    let state = existing.state.lock().unwrap();
                    match &*state {
                        SlotState::Ready(result) => {
                            self.record_hit();
                            return Ok(Arc::clone(result));
                        }
                        SlotState::InFlight => {
                            let slot = Arc::clone(&existing);
                            drop(state);
                            return self.wait_for_compose_slot(key, &slot, compute);
                        }
                        SlotState::Failed(_) => {
                            drop(state);
                            let slot = Arc::new(SingleFlightSlot {
                                state: Mutex::new(SlotState::InFlight),
                                ready: Condvar::new(),
                            });
                            entry.insert(Arc::clone(&slot));
                            slot
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    let slot = Arc::new(SingleFlightSlot {
                        state: Mutex::new(SlotState::InFlight),
                        ready: Condvar::new(),
                    });
                    entry.insert(Arc::clone(&slot));
                    slot
                }
            }
        };

        // We own the InFlight slot.
        self.record_miss();

        match compute() {
            Ok(result) => {
                let arc_result = Arc::new(result);
                {
                    let mut state = slot.state.lock().unwrap();
                    *state = SlotState::Ready(Arc::clone(&arc_result));
                }
                slot.ready.notify_all();
                self.record_write();
                Ok(arc_result)
            }
            Err(err) => {
                {
                    let mut state = slot.state.lock().unwrap();
                    *state = SlotState::Failed(err.to_string());
                }
                slot.ready.notify_all();
                Err(err)
            }
        }
    }

    /// Single-flight get-or-compute for individual operation results.
    ///
    /// Same deduplication pattern as compose results, but for cheaper
    /// operations like code transclusion and TOC linking.
    pub fn get_or_compute_operation<F>(
        &self,
        key: &str,
        compute: F,
    ) -> MarkdownResult<Arc<OperationResult>>
    where
        F: FnOnce() -> MarkdownResult<OperationResult>,
    {
        if self.access_mode == CacheAccessMode::Off {
            let result = compute()?;
            return Ok(Arc::new(result));
        }

        let slot = if self.access_mode == CacheAccessMode::Refresh {
            let slot = Arc::new(SingleFlightSlot {
                state: Mutex::new(SlotState::InFlight),
                ready: Condvar::new(),
            });
            self.operation_results
                .insert(key.to_string(), Arc::clone(&slot));
            slot
        } else {
            match self.operation_results.entry(key.to_string()) {
                Entry::Occupied(mut entry) => {
                    let existing = Arc::clone(entry.get());
                    let state = existing.state.lock().unwrap();
                    match &*state {
                        SlotState::Ready(result) => {
                            self.record_hit();
                            return Ok(Arc::clone(result));
                        }
                        SlotState::InFlight => {
                            let slot = Arc::clone(&existing);
                            drop(state);
                            return self.wait_for_operation_slot(key, &slot, compute);
                        }
                        SlotState::Failed(_) => {
                            drop(state);
                            let slot = Arc::new(SingleFlightSlot {
                                state: Mutex::new(SlotState::InFlight),
                                ready: Condvar::new(),
                            });
                            entry.insert(Arc::clone(&slot));
                            slot
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    let slot = Arc::new(SingleFlightSlot {
                        state: Mutex::new(SlotState::InFlight),
                        ready: Condvar::new(),
                    });
                    entry.insert(Arc::clone(&slot));
                    slot
                }
            }
        };

        self.record_miss();

        match compute() {
            Ok(result) => {
                let arc_result = Arc::new(result);
                {
                    let mut state = slot.state.lock().unwrap();
                    *state = SlotState::Ready(Arc::clone(&arc_result));
                }
                slot.ready.notify_all();
                self.record_write();
                Ok(arc_result)
            }
            Err(err) => {
                {
                    let mut state = slot.state.lock().unwrap();
                    *state = SlotState::Failed(err.to_string());
                }
                slot.ready.notify_all();
                Err(err)
            }
        }
    }

    /// Takes a snapshot of the current cache stats.
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    fn wait_for_compose_slot<F>(
        &self,
        key: &str,
        slot: &Arc<SingleFlightSlot<ComposeResult>>,
        fallback_compute: F,
    ) -> MarkdownResult<Arc<ComposeResult>>
    where
        F: FnOnce() -> MarkdownResult<ComposeResult>,
    {
        let state = slot.state.lock().unwrap();
        let (state, timeout_result) = slot
            .ready
            .wait_timeout_while(state, INFLIGHT_TIMEOUT, |s| {
                matches!(s, SlotState::InFlight)
            })
            .unwrap();

        if timeout_result.timed_out() {
            self.record_error();
            drop(state);
            let result = fallback_compute()?;
            return Ok(Arc::new(result));
        }

        match &*state {
            SlotState::Ready(result) => {
                self.record_hit();
                self.record_inflight_wait();
                Ok(Arc::clone(result))
            }
            SlotState::Failed(msg) => {
                let msg = msg.clone();
                drop(state);
                self.record_miss();
                let result = fallback_compute()?;
                let arc_result = Arc::new(result);
                {
                    if let Some(existing) = self.compose_results.get(key) {
                        let slot = Arc::clone(existing.value());
                        drop(existing);
                        let mut s = slot.state.lock().unwrap();
                        if matches!(&*s, SlotState::Failed(m) if *m == msg) {
                            *s = SlotState::Ready(Arc::clone(&arc_result));
                        }
                    }
                }
                self.record_write();
                Ok(arc_result)
            }
            SlotState::InFlight => {
                self.record_error();
                drop(state);
                let result = fallback_compute()?;
                Ok(Arc::new(result))
            }
        }
    }

    fn wait_for_operation_slot<F>(
        &self,
        key: &str,
        slot: &Arc<SingleFlightSlot<OperationResult>>,
        fallback_compute: F,
    ) -> MarkdownResult<Arc<OperationResult>>
    where
        F: FnOnce() -> MarkdownResult<OperationResult>,
    {
        let state = slot.state.lock().unwrap();
        let (state, timeout_result) = slot
            .ready
            .wait_timeout_while(state, INFLIGHT_TIMEOUT, |s| {
                matches!(s, SlotState::InFlight)
            })
            .unwrap();

        if timeout_result.timed_out() {
            self.record_error();
            drop(state);
            let result = fallback_compute()?;
            return Ok(Arc::new(result));
        }

        match &*state {
            SlotState::Ready(result) => {
                self.record_hit();
                self.record_inflight_wait();
                Ok(Arc::clone(result))
            }
            SlotState::Failed(msg) => {
                let msg = msg.clone();
                drop(state);
                self.record_miss();
                let result = fallback_compute()?;
                let arc_result = Arc::new(result);
                {
                    if let Some(existing) = self.operation_results.get(key) {
                        let slot = Arc::clone(existing.value());
                        drop(existing);
                        let mut s = slot.state.lock().unwrap();
                        if matches!(&*s, SlotState::Failed(m) if *m == msg) {
                            *s = SlotState::Ready(Arc::clone(&arc_result));
                        }
                    }
                }
                self.record_write();
                Ok(arc_result)
            }
            SlotState::InFlight => {
                self.record_error();
                drop(state);
                let result = fallback_compute()?;
                Ok(Arc::new(result))
            }
        }
    }
    fn record_hit(&self) {
        self.stats.lock().unwrap().hits += 1;
    }

    fn record_miss(&self) {
        self.stats.lock().unwrap().misses += 1;
    }

    fn record_write(&self) {
        self.stats.lock().unwrap().writes += 1;
    }

    fn record_inflight_wait(&self) {
        self.stats.lock().unwrap().inflight_waits += 1;
    }

    fn record_error(&self) {
        self.stats.lock().unwrap().errors += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_off_always_computes() {
        let cache = RunLocalCache::new(CacheAccessMode::Off);
        let mut call_count = 0;

        for _ in 0..3 {
            let result = cache
                .get_or_compute_compose("key1", || {
                    call_count += 1;
                    Ok(ComposeResult {
                        content: format!("result-{}", call_count),
                        report: ComposeReport::new(),
                        context_groups: Default::default(),
                    })
                })
                .unwrap();
            assert!(result.content.starts_with("result-"));
        }

        assert_eq!(call_count, 3);
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn cache_hit_on_second_request() {
        let cache = RunLocalCache::new(CacheAccessMode::ReadWrite);
        let call_count = std::sync::atomic::AtomicUsize::new(0);

        let result1 = cache
            .get_or_compute_compose("key1", || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ComposeResult {
                    content: "hello".to_string(),
                    report: ComposeReport::new(),
                    context_groups: Default::default(),
                })
            })
            .unwrap();

        let result2 = cache
            .get_or_compute_compose("key1", || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ComposeResult {
                    content: "should not compute".to_string(),
                    report: ComposeReport::new(),
                    context_groups: Default::default(),
                })
            })
            .unwrap();

        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(result1.content, "hello");
        assert_eq!(result2.content, "hello");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.writes, 1);
    }

    #[test]
    fn single_flight_contention() {
        use std::sync::Barrier;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache = RunLocalCache::new(CacheAccessMode::ReadWrite);
        let compute_count = Arc::new(AtomicUsize::new(0));
        let thread_count = 8;
        let barrier = Arc::new(Barrier::new(thread_count));

        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for _ in 0..thread_count {
                let cache = cache.clone();
                let compute_count = Arc::clone(&compute_count);
                let barrier = Arc::clone(&barrier);

                handles.push(s.spawn(move || {
                    barrier.wait();
                    cache
                        .get_or_compute_compose(
                            "contested-key",
                            || {
                                compute_count.fetch_add(1, Ordering::SeqCst);
                                std::thread::sleep(Duration::from_millis(50));
                                Ok(ComposeResult {
                                    content: "shared-result".to_string(),
                                    report: ComposeReport::new(),
                                    context_groups: Default::default(),
                                })
                            },
                        )
                        .unwrap()
                }));
            }

            for handle in handles {
                let result = handle.join().unwrap();
                assert_eq!(result.content, "shared-result");
            }
        });

        // With single-flight, at most a couple of threads should compute
        let count = compute_count.load(Ordering::SeqCst);
        assert!(
            count <= 2,
            "Expected at most 2 computations with single-flight, got {}",
            count
        );

        let stats = cache.stats();
        assert!(stats.hits >= 1, "Expected at least one cache hit");
    }

    #[test]
    fn stats_accumulation() {
        let mut stats1 = CacheStats {
            hits: 5,
            misses: 3,
            writes: 3,
            inflight_waits: 1,
            errors: 0,
        };
        let stats2 = CacheStats {
            hits: 2,
            misses: 1,
            writes: 1,
            inflight_waits: 0,
            errors: 1,
        };

        stats1.merge(&stats2);

        assert_eq!(stats1.hits, 7);
        assert_eq!(stats1.misses, 4);
        assert_eq!(stats1.writes, 4);
        assert_eq!(stats1.inflight_waits, 1);
        assert_eq!(stats1.errors, 1);
    }

    #[test]
    fn refresh_mode_bypasses_existing() {
        let cache = RunLocalCache::new(CacheAccessMode::ReadWrite);

        // Populate cache
        cache
            .get_or_compute_compose("key1", || {
                Ok(ComposeResult {
                    content: "original".to_string(),
                    report: ComposeReport::new(),
                    context_groups: Default::default(),
                })
            })
            .unwrap();

        // Create a refresh-mode view sharing the same backing stores
        let refresh_cache = RunLocalCache {
            access_mode: CacheAccessMode::Refresh,
            markdown_documents: Arc::clone(&cache.markdown_documents),
            toc_headings: Arc::clone(&cache.toc_headings),
            compose_results: Arc::clone(&cache.compose_results),
            operation_results: Arc::clone(&cache.operation_results),
            stats: Arc::clone(&cache.stats),
        };

        let result = refresh_cache
            .get_or_compute_compose("key1", || {
                Ok(ComposeResult {
                    content: "refreshed".to_string(),
                    report: ComposeReport::new(),
                    context_groups: Default::default(),
                })
            })
            .unwrap();

        assert_eq!(result.content, "refreshed");
    }

    // ── Operation result tests ───────────────────────────────────────

    #[test]
    fn operation_cache_off_always_computes() {
        let cache = RunLocalCache::new(CacheAccessMode::Off);
        let mut call_count = 0;

        for _ in 0..3 {
            let result = cache
                .get_or_compute_operation("op-key1", || {
                    call_count += 1;
                    Ok(OperationResult {
                        content: format!("op-{}", call_count),
                    })
                })
                .unwrap();
            assert!(result.content.starts_with("op-"));
        }

        assert_eq!(call_count, 3);
    }

    #[test]
    fn operation_cache_hit_on_second_request() {
        let cache = RunLocalCache::new(CacheAccessMode::ReadWrite);
        let call_count = std::sync::atomic::AtomicUsize::new(0);

        let result1 = cache
            .get_or_compute_operation("op-key1", || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(OperationResult {
                    content: "code-block".to_string(),
                })
            })
            .unwrap();

        let result2 = cache
            .get_or_compute_operation("op-key1", || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(OperationResult {
                    content: "should not compute".to_string(),
                })
            })
            .unwrap();

        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(result1.content, "code-block");
        assert_eq!(result2.content, "code-block");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.writes, 1);
    }

    #[test]
    fn operation_single_flight_contention() {
        use std::sync::Barrier;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache = RunLocalCache::new(CacheAccessMode::ReadWrite);
        let compute_count = Arc::new(AtomicUsize::new(0));
        let thread_count = 8;
        let barrier = Arc::new(Barrier::new(thread_count));

        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for _ in 0..thread_count {
                let cache = cache.clone();
                let compute_count = Arc::clone(&compute_count);
                let barrier = Arc::clone(&barrier);

                handles.push(s.spawn(move || {
                    barrier.wait();
                    cache
                        .get_or_compute_operation(
                            "contested-op",
                            || {
                                compute_count.fetch_add(1, Ordering::SeqCst);
                                std::thread::sleep(Duration::from_millis(50));
                                Ok(OperationResult {
                                    content: "shared-op".to_string(),
                                })
                            },
                        )
                        .unwrap()
                }));
            }

            for handle in handles {
                let result = handle.join().unwrap();
                assert_eq!(result.content, "shared-op");
            }
        });

        let count = compute_count.load(Ordering::SeqCst);
        assert!(
            count <= 2,
            "Expected at most 2 computations with single-flight, got {}",
            count
        );
    }

    #[test]
    fn operation_and_compose_keys_are_independent() {
        let cache = RunLocalCache::new(CacheAccessMode::ReadWrite);

        // Same key string, different namespaces
        cache
            .get_or_compute_compose("shared-key", || {
                Ok(ComposeResult {
                    content: "compose-content".to_string(),
                    report: ComposeReport::new(),
                    context_groups: Default::default(),
                })
            })
            .unwrap();

        let op_result = cache
            .get_or_compute_operation("shared-key", || {
                Ok(OperationResult {
                    content: "operation-content".to_string(),
                })
            })
            .unwrap();

        // Operation should NOT hit the compose cache
        assert_eq!(op_result.content, "operation-content");
    }
}
