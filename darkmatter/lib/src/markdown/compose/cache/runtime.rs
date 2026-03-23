//! Run-local in-memory cache with single-flight deduplication.
//!
//! `RunLocalCache` replaces the old `PipelineCache` and adds single-flight
//! behavior for child compose operations and individual operations (code
//! transclusion, TOC linking): when multiple rayon threads request the same
//! result simultaneously, only one computes while others wait.

use super::hashing::{
    body_semantic_hash, body_template_hash, compose_cache_key, frontmatter_hash, raw_bytes_hash,
    source_id_hash,
};
use super::manifest::{CACHE_VERSION, DocumentSnapshotManifest};
use super::store::FileStore;
use super::types::{ArtifactClass, CacheAccessMode, CacheStats, SourceKind};
use crate::markdown::compose::ComposeReport;
use crate::markdown::types::MarkdownResult;
use crate::markdown::Markdown;
use crate::markdown::toc::MarkdownTocNode;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
}

/// The cached result of an individual operation (code transclusion, TOC linking).
pub(crate) struct OperationResult {
    /// The core operation output (before parent-specific wrappers).
    pub content: String,
}

/// Pre-computed hash dimensions for persistent cache key computation.
///
/// Carries the state, context, and options hashes that are available at the
/// call site. Combined with `body_semantic_hash` from the document snapshot,
/// these form the full multi-dimensional persistent cache key via
/// [`compose_entry_key`](super::hashing::compose_entry_key).
pub(crate) struct PersistentContext {
    pub source_id: u64,
    pub state_hash: u64,
    pub context_hash: u64,
    pub options_hash: u64,
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
    compose_results: Arc<Mutex<HashMap<String, Arc<SingleFlightSlot<ComposeResult>>>>>,
    operation_results: Arc<Mutex<HashMap<String, Arc<SingleFlightSlot<OperationResult>>>>>,
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
            operation_results: Arc::new(Mutex::new(HashMap::new())),
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
    ///
    /// When persistent caching is enabled, also creates a
    /// [`DocumentSnapshotManifest`] for the loaded document (Step 2.6).
    pub fn load_markdown(&self, path: &Path) -> MarkdownResult<Markdown> {
        let key = compose_cache_key(path);

        if self.access_mode != CacheAccessMode::Off {
            let cache = self.markdown_documents.lock().unwrap();
            if let Some(markdown) = cache.get(&key) {
                return Ok(markdown.clone());
            }
        }

        let markdown = Markdown::try_from(path)?;

        // Create a persistent document snapshot (best-effort)
        self.try_write_document_snapshot(path, &markdown);

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
    ///
    /// When `persistent_ctx` is provided, the persistent cache uses a
    /// full multi-dimensional key (source + body + state + context + options)
    /// instead of just the path-based key.
    pub fn get_or_compute_compose<F>(
        &self,
        key: &str,
        persistent_ctx: Option<&PersistentContext>,
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

            if self.access_mode != CacheAccessMode::Refresh
                && let Some(existing) = map.get(key)
            {
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
                        return self.wait_for_compose_slot(key, &slot, compute);
                    }
                    SlotState::Failed(_) => {
                        // Previous attempt failed — we'll recompute below
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
        if let Some(result) = self.try_persistent_read(key, persistent_ctx) {
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
                self.try_persistent_write(key, persistent_ctx, &result);

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
    /// operations like code transclusion and TOC linking. No persistent
    /// cache integration yet (run-local only).
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

        let slot = {
            let mut map = self.operation_results.lock().unwrap();

            if self.access_mode != CacheAccessMode::Refresh
                && let Some(existing) = map.get(key)
            {
                let state = existing.state.lock().unwrap();
                match &*state {
                    SlotState::Ready(result) => {
                        self.record_hit();
                        return Ok(Arc::clone(result));
                    }
                    SlotState::InFlight => {
                        let slot = Arc::clone(existing);
                        drop(state);
                        drop(map);
                        return self.wait_for_operation_slot(key, &slot, compute);
                    }
                    SlotState::Failed(_) => {
                        // Previous attempt failed — recompute below
                    }
                }
            }

            let slot = Arc::new(SingleFlightSlot {
                state: Mutex::new(SlotState::InFlight),
                ready: Condvar::new(),
            });
            map.insert(key.to_string(), Arc::clone(&slot));
            slot
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
                    let map = self.operation_results.lock().unwrap();
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
                self.record_error();
                drop(state);
                let result = fallback_compute()?;
                Ok(Arc::new(result))
            }
        }
    }

    /// Attempts to read a compose result from the persistent store.
    ///
    /// When a `PersistentContext` is provided, computes the full entry key
    /// from the document snapshot's `body_semantic_hash` combined with the
    /// state/context/options hashes. Falls back to path-based key otherwise.
    fn try_persistent_read(
        &self,
        key: &str,
        persistent_ctx: Option<&PersistentContext>,
    ) -> Option<ComposeResult> {
        use super::hashing::compose_entry_key;

        let store = self.persistent.as_ref()?;

        // Compute the persistent entry key
        let entry_key = match persistent_ctx {
            Some(ctx) => {
                // Look up the document snapshot to get body_semantic_hash
                let snapshot: DocumentSnapshotManifest = store
                    .read_manifest(ArtifactClass::DocumentSnapshot, ctx.source_id)
                    .ok()
                    .flatten()?;

                compose_entry_key(
                    ctx.source_id,
                    snapshot.body_semantic_hash,
                    ctx.state_hash,
                    ctx.context_hash,
                    ctx.options_hash,
                )
            }
            None => biscuit_hash::xx_hash(key),
        };

        // Read the composed document manifest
        let mut manifest: super::manifest::ComposedDocumentManifest =
            match store.read_manifest(ArtifactClass::ComposeDocumentCore, entry_key) {
                Ok(Some(m)) => m,
                Ok(None) => return None,
                Err(e) => {
                    tracing::debug!("Persistent cache manifest read error for {}: {}", key, e);
                    return None;
                }
            };

        // Check expiration
        if manifest.is_expired() {
            tracing::debug!("Persistent cache entry expired for {}", key);
            return None;
        }

        // Read the blob using the hash from the manifest
        let blob = match store.read_blob(manifest.payload_blob_hash, "md") {
            Ok(Some(data)) => data,
            Ok(None) => {
                tracing::debug!(
                    "Persistent cache blob missing for {} (hash {:016x})",
                    key,
                    manifest.payload_blob_hash
                );
                return None;
            }
            Err(e) => {
                tracing::debug!("Persistent cache blob read error for {}: {}", key, e);
                return None;
            }
        };

        let content = match String::from_utf8(blob) {
            Ok(s) => s,
            Err(_) => return None,
        };

        // Update last-accessed timestamp (best-effort)
        manifest.touch();
        let _ = store.write_manifest(ArtifactClass::ComposeDocumentCore, entry_key, &manifest);

        self.stats.lock().unwrap().persistent_hits += 1;
        Some(ComposeResult {
            content,
            report: ComposeReport::new(),
        })
    }

    /// Attempts to write a compose result to the persistent store (best-effort).
    ///
    /// When a `PersistentContext` is provided, computes the full entry key
    /// and Merkle-style closure hash. Falls back to path-based key otherwise.
    fn try_persistent_write(
        &self,
        key: &str,
        persistent_ctx: Option<&PersistentContext>,
        result: &ComposeResult,
    ) {
        use super::hashing::{closure_hash, compose_entry_key};

        let Some(store) = self.persistent.as_ref() else {
            return;
        };

        if self.access_mode == CacheAccessMode::ReadOnly {
            return;
        }

        let blob = result.content.as_bytes();
        let blob_hash = raw_bytes_hash(blob);

        // Compute the entry key and self_hash from persistent context
        let (entry_key, self_hash) = match persistent_ctx {
            Some(ctx) => {
                let body_semantic = body_semantic_hash(&result.content);
                let ek = compose_entry_key(
                    ctx.source_id,
                    body_semantic,
                    ctx.state_hash,
                    ctx.context_hash,
                    ctx.options_hash,
                );
                (ek, blob_hash)
            }
            None => {
                let ek = biscuit_hash::xx_hash(key);
                (ek, blob_hash)
            }
        };

        // Compute closure hash (no dependencies tracked yet — leaf documents)
        let deps = vec![];
        let c_hash = closure_hash(self_hash, &deps);

        let manifest = super::manifest::ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key,
            self_hash,
            closure_hash: c_hash,
            dependency_count: deps.len(),
            dependencies: deps,
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

    /// Creates a persistent document snapshot for a loaded markdown file.
    ///
    /// Snapshots provide the foundation for validating composed document
    /// manifests via Merkle-style closure hashes (Plan Step 2.6).
    fn try_write_document_snapshot(&self, path: &Path, markdown: &Markdown) {
        let Some(store) = self.persistent.as_ref() else {
            return;
        };

        if self.access_mode == CacheAccessMode::Off
            || self.access_mode == CacheAccessMode::ReadOnly
        {
            return;
        }

        let canonical = compose_cache_key(path);
        let sid_hash = source_id_hash(&canonical);

        // Skip if we already have a snapshot for this source
        if store.has_manifest(ArtifactClass::DocumentSnapshot, sid_hash) {
            // Fast check: if manifest exists, verify freshness via mtime+size
            if let Ok(meta) = std::fs::metadata(path) {
                let current_size = meta.len();
                let current_modified = meta
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                if let Ok(Some(existing)) = store.read_manifest::<DocumentSnapshotManifest>(
                    ArtifactClass::DocumentSnapshot,
                    sid_hash,
                ) && existing.is_fresh(current_modified, current_size)
                {
                    return; // Snapshot is still valid
                }
            }
        }

        // Compute hashes for the snapshot
        let content = markdown.content();
        let content_bytes = content.as_bytes();
        let fm_map = markdown.frontmatter().as_map();
        let fm_as_serde_map: serde_json::Map<String, serde_json::Value> =
            fm_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        let (modified_at, size_bytes) = std::fs::metadata(path)
            .map(|m| {
                (
                    m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    m.len(),
                )
            })
            .unwrap_or((std::time::SystemTime::UNIX_EPOCH, 0));

        let manifest = DocumentSnapshotManifest {
            cache_version: CACHE_VERSION,
            source_kind: SourceKind::LocalFile,
            canonical_source: canonical,
            source_id_hash: sid_hash,
            raw_bytes_hash: raw_bytes_hash(content_bytes),
            frontmatter_hash: frontmatter_hash(&fm_as_serde_map),
            body_semantic_hash: body_semantic_hash(content),
            body_template_hash: body_template_hash(content),
            modified_at,
            size_bytes,
        };

        if let Err(e) = store.write_manifest(ArtifactClass::DocumentSnapshot, sid_hash, &manifest)
        {
            tracing::debug!("Failed to write document snapshot for {:?}: {}", path, e);
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
                .get_or_compute_compose("key1", None, || {
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
            .get_or_compute_compose("key1", None, || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ComposeResult {
                    content: "hello".to_string(),
                    report: ComposeReport::new(),
                })
            })
            .unwrap();

        let result2 = cache
            .get_or_compute_compose("key1", None, || {
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
                        .get_or_compute_compose("contested-key", None, || {
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
            .get_or_compute_compose("key1", None, || {
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
            operation_results: Arc::clone(&cache.operation_results),
            stats: Arc::clone(&cache.stats),
            persistent: cache.persistent.clone(),
        };

        let result = refresh_cache
            .get_or_compute_compose("key1", None, || {
                Ok(ComposeResult {
                    content: "refreshed".to_string(),
                    report: ComposeReport::new(),
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
                        .get_or_compute_operation("contested-op", || {
                            compute_count.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(Duration::from_millis(50));
                            Ok(OperationResult {
                                content: "shared-op".to_string(),
                            })
                        })
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
            .get_or_compute_compose("shared-key", None, || {
                Ok(ComposeResult {
                    content: "compose-content".to_string(),
                    report: ComposeReport::new(),
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
