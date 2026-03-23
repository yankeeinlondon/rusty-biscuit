//! Run-local in-memory cache with single-flight deduplication.
//!
//! `RunLocalCache` replaces the old `PipelineCache` and adds single-flight
//! behavior for child compose operations: when multiple rayon threads request
//! the same document simultaneously, only one computes while others wait.

use super::hashing::{compose_cache_key, raw_bytes_hash};
use super::store::FileStore;
use super::types::{ArtifactClass, CacheAccessMode, CacheStats};
use crate::markdown::compose::ComposeReport;
use crate::markdown::types::MarkdownResult;
use crate::markdown::Markdown;
use crate::markdown::toc::MarkdownTocNode;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// State of a single-flight slot for a compose result.
enum SlotState {
    /// Computation is in progress — waiters should block on the condvar.
    InFlight,
    /// Computation completed successfully.
    Ready(Arc<ComposeResult>),
    /// Computation failed — waiters should recompute or propagate.
    Failed(String),
}

/// A single-flight slot protected by a mutex and condvar.
struct SingleFlightSlot {
    state: Mutex<SlotState>,
    ready: Condvar,
}

/// The cached result of a child compose operation.
pub(crate) struct ComposeResult {
    /// The composed document content (post-compose, pre-parent-transforms).
    pub content: String,
    /// The compose report from the child pipeline.
    pub report: ComposeReport,
}

/// Timeout for waiting on an in-flight computation before falling back
/// to duplicate computation (mitigates rayon deadlock risk).
const INFLIGHT_TIMEOUT: Duration = Duration::from_secs(30);

/// Run-local cache shared across all threads in a single compose invocation.
///
/// All fields are `Arc`-wrapped so `clone()` produces a shared view
/// (same backing data), matching the old `PipelineCache` clone semantics.
///
/// When a `FileStore` is attached, the cache also reads/writes persistent
/// artifacts on disk for cross-run caching.
#[derive(Clone)]
pub(crate) struct RunLocalCache {
    markdown_documents: Arc<Mutex<HashMap<String, Markdown>>>,
    toc_headings: Arc<Mutex<HashMap<String, Vec<MarkdownTocNode>>>>,
    compose_results: Arc<Mutex<HashMap<String, Arc<SingleFlightSlot>>>>,
    stats: Arc<Mutex<CacheStats>>,
    access_mode: CacheAccessMode,
    /// Optional persistent file-backed cache store.
    persistent: Option<Arc<FileStore>>,
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
            markdown_documents: Arc::new(Mutex::new(HashMap::new())),
            toc_headings: Arc::new(Mutex::new(HashMap::new())),
            compose_results: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats::default())),
            access_mode,
            persistent: None,
        }
    }

    /// Creates a new cache with persistent file-backed storage.
    ///
    /// If the `FileStore` cannot be created (e.g., permission error),
    /// falls back to run-local only and logs the error.
    pub fn with_persistent(mut self, cache_root: PathBuf) -> Self {
        match FileStore::new(cache_root) {
            Ok(store) => {
                self.persistent = Some(Arc::new(store));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize persistent cache: {}", e);
            }
        }
        self
    }

    /// Loads a markdown document, returning a cached copy if available.
    pub fn load_markdown(&self, path: &Path) -> MarkdownResult<Markdown> {
        let key = compose_cache_key(path);

        if self.access_mode != CacheAccessMode::Off {
            let cache = self.markdown_documents.lock().unwrap();
            if let Some(markdown) = cache.get(&key) {
                return Ok(markdown.clone());
            }
        }

        let markdown = Markdown::try_from(path)?;

        if self.access_mode != CacheAccessMode::Off
            && self.access_mode != CacheAccessMode::ReadOnly
        {
            let mut cache = self.markdown_documents.lock().unwrap();
            Ok(cache.entry(key).or_insert_with(|| markdown.clone()).clone())
        } else {
            Ok(markdown)
        }
    }

    /// Loads TOC headings for a document, returning a cached copy if available.
    pub fn load_toc_headings(&self, path: &Path) -> std::io::Result<Vec<MarkdownTocNode>> {
        let key = compose_cache_key(path);

        if self.access_mode != CacheAccessMode::Off {
            let cache = self.toc_headings.lock().unwrap();
            if let Some(headings) = cache.get(&key) {
                return Ok(headings.clone());
            }
        }

        let content = std::fs::read_to_string(path)?;
        let markdown: Markdown = content.into();
        let headings = markdown
            .toc()
            .all_headings()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        if self.access_mode != CacheAccessMode::Off
            && self.access_mode != CacheAccessMode::ReadOnly
        {
            let mut cache = self.toc_headings.lock().unwrap();
            Ok(cache.entry(key).or_insert_with(|| headings.clone()).clone())
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
        let slot = {
            let mut map = self.compose_results.lock().unwrap();

            if self.access_mode != CacheAccessMode::Refresh {
                if let Some(existing) = map.get(key) {
                    let state = existing.state.lock().unwrap();
                    match &*state {
                        SlotState::Ready(result) => {
                            self.record_hit();
                            return Ok(Arc::clone(result));
                        }
                        SlotState::InFlight => {
                            // Clone the Arc to wait on it outside the map lock
                            let slot = Arc::clone(existing);
                            drop(state);
                            drop(map);
                            return self.wait_for_slot(key, &slot, compute);
                        }
                        SlotState::Failed(_) => {
                            // Previous attempt failed — we'll recompute below
                        }
                    }
                }
            }

            // Insert InFlight slot
            let slot = Arc::new(SingleFlightSlot {
                state: Mutex::new(SlotState::InFlight),
                ready: Condvar::new(),
            });
            map.insert(key.to_string(), Arc::clone(&slot));
            slot
        };

        // We own the InFlight slot — try persistent cache before computing
        self.record_miss();

        // Check persistent store
        if let Some(result) = self.try_persistent_read(key) {
            let arc_result = Arc::new(result);
            {
                let mut state = slot.state.lock().unwrap();
                *state = SlotState::Ready(Arc::clone(&arc_result));
            }
            slot.ready.notify_all();
            self.record_write();
            return Ok(arc_result);
        }

        // Compute fresh result
        match compute() {
            Ok(result) => {
                // Write to persistent store (best-effort)
                self.try_persistent_write(key, &result);

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

    fn wait_for_slot<F>(
        &self,
        key: &str,
        slot: &Arc<SingleFlightSlot>,
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
            // Deadlock mitigation: fall back to duplicate computation
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
                // The original computation failed — try again ourselves
                let msg = msg.clone();
                drop(state);
                self.record_miss();
                let result = fallback_compute()?;
                let arc_result = Arc::new(result);
                // Update the slot for future waiters
                {
                    let map = self.compose_results.lock().unwrap();
                    if let Some(existing) = map.get(key) {
                        let mut s = existing.state.lock().unwrap();
                        if matches!(&*s, SlotState::Failed(m) if *m == msg) {
                            *s = SlotState::Ready(Arc::clone(&arc_result));
                        }
                    }
                }
                self.record_write();
                Ok(arc_result)
            }
            SlotState::InFlight => {
                // Shouldn't happen after wait_timeout_while, but handle gracefully
                self.record_error();
                drop(state);
                let result = fallback_compute()?;
                Ok(Arc::new(result))
            }
        }
    }

    /// Attempts to read a compose result from the persistent store.
    fn try_persistent_read(&self, key: &str) -> Option<ComposeResult> {
        let store = self.persistent.as_ref()?;

        // Use the key string hash as the persistent lookup key
        let entry_key = biscuit_hash::xx_hash(key);

        // Read the blob (composed content)
        let blob = match store.read_blob(entry_key, "md") {
            Ok(Some(data)) => data,
            Ok(None) => return None,
            Err(e) => {
                tracing::debug!("Persistent cache read error for {}: {}", key, e);
                return None;
            }
        };

        let content = match String::from_utf8(blob) {
            Ok(s) => s,
            Err(_) => return None,
        };

        self.stats.lock().unwrap().persistent_hits += 1;
        Some(ComposeResult {
            content,
            report: ComposeReport::new(),
        })
    }

    /// Attempts to write a compose result to the persistent store (best-effort).
    fn try_persistent_write(&self, key: &str, result: &ComposeResult) {
        let Some(store) = self.persistent.as_ref() else {
            return;
        };

        if self.access_mode == CacheAccessMode::ReadOnly {
            return;
        }

        let entry_key = biscuit_hash::xx_hash(key);
        let blob = result.content.as_bytes();
        let blob_hash = raw_bytes_hash(blob);

        // Write the manifest
        let manifest = super::manifest::ComposedDocumentManifest {
            cache_version: super::manifest::CACHE_VERSION,
            entry_key,
            self_hash: blob_hash,
            closure_hash: blob_hash, // Simplified for now; full closure hash in future
            dependency_count: 0,
            dependencies: vec![],
            payload_blob_hash: blob_hash,
            warnings_hash: 0,
            created_at: std::time::SystemTime::now(),
            last_accessed_at: std::time::SystemTime::now(),
            expires_at: None,
        };

        if let Err(e) = store.write_artifact(
            ArtifactClass::ComposeDocumentCore,
            entry_key,
            &manifest,
            blob,
            blob_hash,
            "md",
        ) {
            tracing::debug!("Persistent cache write error for {}: {}", key, e);
        } else {
            self.stats.lock().unwrap().persistent_writes += 1;
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
                })
            })
            .unwrap();

        let result2 = cache
            .get_or_compute_compose("key1", || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ComposeResult {
                    content: "should not compute".to_string(),
                    report: ComposeReport::new(),
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
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Barrier;

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
                        .get_or_compute_compose("contested-key", || {
                            compute_count.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(Duration::from_millis(50));
                            Ok(ComposeResult {
                                content: "shared-result".to_string(),
                                report: ComposeReport::new(),
                            })
                        })
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
            ..Default::default()
        };
        let stats2 = CacheStats {
            hits: 2,
            misses: 1,
            writes: 1,
            inflight_waits: 0,
            errors: 1,
            ..Default::default()
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
                })
            })
            .unwrap();

        // Create a refresh-mode view sharing the same backing stores
        let refresh_cache = RunLocalCache {
            access_mode: CacheAccessMode::Refresh,
            markdown_documents: Arc::clone(&cache.markdown_documents),
            toc_headings: Arc::clone(&cache.toc_headings),
            compose_results: Arc::clone(&cache.compose_results),
            stats: Arc::clone(&cache.stats),
            persistent: cache.persistent.clone(),
        };

        let result = refresh_cache
            .get_or_compute_compose("key1", || {
                Ok(ComposeResult {
                    content: "refreshed".to_string(),
                    report: ComposeReport::new(),
                })
            })
            .unwrap();

        assert_eq!(result.content, "refreshed");
    }
}
