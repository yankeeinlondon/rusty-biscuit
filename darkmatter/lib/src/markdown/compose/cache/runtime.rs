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
use super::manifest::{
    CACHE_VERSION, ComposedDocumentManifest, DocumentSnapshotManifest, OperationResultManifest,
    RemoteUrlManifest,
};
use super::store::FileStore;
use super::types::{
    ArtifactClass, CacheAccessMode, CacheFreshnessMode, CacheStats, DependencyRef, SourceKind,
};
use crate::markdown::Markdown;
use crate::markdown::compose::ComposeReport;
use crate::markdown::toc::MarkdownTocNode;
use crate::markdown::types::MarkdownResult;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
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
    /// Direct cache dependencies of this composed document.
    pub dependencies: Vec<DependencyRef>,
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

/// Pre-computed source state for a persistable operation result.
pub(crate) struct OperationPersistentContext {
    pub op_kind: &'static str,
    pub entry_key: u64,
    pub source_id: u64,
    pub canonical_source: String,
    pub source_content_hash: u64,
}

enum PersistentLookup<T> {
    Fresh(T),
    Stale(T),
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
    markdown_documents: Arc<DashMap<String, Markdown>>,
    toc_headings: Arc<DashMap<String, Vec<MarkdownTocNode>>>,
    document_snapshots: Arc<DashMap<u64, DocumentSnapshotManifest>>,
    compose_results: Arc<DashMap<String, Arc<SingleFlightSlot<ComposeResult>>>>,
    operation_results: Arc<DashMap<String, Arc<SingleFlightSlot<OperationResult>>>>,
    stats: Arc<Mutex<CacheStats>>,
    access_mode: CacheAccessMode,
    /// Optional persistent file-backed cache store.
    persistent: Option<Arc<FileStore>>,
    /// The run's shared remote-fetch runtime, used to revalidate `RemoteUrl`
    /// dependencies under the active `RemoteReadConfig` during compose-manifest
    /// validation. Absent in caches built without remote reads.
    remote_fetch: Option<crate::markdown::compose::remote_fetch::RemoteFetchRuntime>,
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
            document_snapshots: Arc::new(DashMap::new()),
            compose_results: Arc::new(DashMap::new()),
            operation_results: Arc::new(DashMap::new()),
            stats: Arc::new(Mutex::new(CacheStats::default())),
            access_mode,
            persistent: None,
            remote_fetch: None,
        }
    }

    /// Attaches the run's shared remote-fetch runtime.
    ///
    /// Lets compose-manifest validation revalidate `RemoteUrl` dependencies
    /// under the active `RemoteReadConfig` instead of trusting the persisted
    /// remote artifact hash, so a cached local child cannot serve stale output
    /// when its nested remote URL changes (or `--remote-refresh` is set).
    pub fn with_remote_fetch(
        mut self,
        remote_fetch: crate::markdown::compose::remote_fetch::RemoteFetchRuntime,
    ) -> Self {
        self.remote_fetch = Some(remote_fetch);
        self
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

        if self.access_mode != CacheAccessMode::Off
            && let Some(markdown) = self.markdown_documents.get(&key)
        {
            return Ok(markdown.clone());
        }

        let markdown = Markdown::try_from(path)?;

        // Create a persistent document snapshot (best-effort)
        self.try_write_document_snapshot(path, &markdown);

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
    ///
    /// When `persistent_ctx` is provided, the persistent cache uses a
    /// full multi-dimensional key (source + body + state + context + options)
    /// instead of just the path-based key.
    ///
    /// `persistent_eligible` gates the persistent store: when `false`, run-local
    /// single-flight reuse still applies but no persistent read or write is
    /// attempted. Callers pass `false` when correct reuse would require
    /// process-local instance identity that a persistent key cannot express
    /// (e.g. an attached shell approval handler).
    pub fn get_or_compute_compose<F>(
        &self,
        key: &str,
        persistent_ctx: Option<&PersistentContext>,
        freshness_mode: CacheFreshnessMode,
        persistent_eligible: bool,
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

        // We own the InFlight slot — try persistent cache before computing
        self.record_miss();

        // Check persistent store (skipped entirely when the key is not
        // persistent-eligible — run-local single-flight reuse still applied).
        let persistent_lookup = if persistent_eligible {
            self.try_persistent_read_compose(key, persistent_ctx, freshness_mode)
        } else {
            None
        };
        let stale_fallback =
            match persistent_lookup {
                Some(PersistentLookup::Fresh(result)) => {
                    let arc_result = Arc::new(result);
                    {
                        let mut state = slot.state.lock().unwrap();
                        *state = SlotState::Ready(Arc::clone(&arc_result));
                    }
                    slot.ready.notify_all();
                    self.record_write();
                    return Ok(arc_result);
                }
                Some(PersistentLookup::Stale(result))
                    if freshness_mode == CacheFreshnessMode::Fallback =>
                {
                    Some(result)
                }
                Some(PersistentLookup::Stale(_)) => None,
                None => None,
            };

        // Compute fresh result
        match compute() {
            Ok(result) => {
                // Write to persistent store (best-effort; skipped when the key
                // is not persistent-eligible).
                if persistent_eligible {
                    self.try_persistent_write_compose(key, persistent_ctx, &result);
                }

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
                if let Some(stale) = stale_fallback {
                    let arc_result = Arc::new(stale);
                    {
                        let mut state = slot.state.lock().unwrap();
                        *state = SlotState::Ready(Arc::clone(&arc_result));
                    }
                    slot.ready.notify_all();
                    self.record_stale_hit();
                    return Ok(arc_result);
                }

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
    /// operations like code transclusion and TOC linking, including their
    /// persistent cache manifests when a file-backed store is attached.
    pub fn get_or_compute_operation<F>(
        &self,
        key: &str,
        persistent_ctx: Option<&OperationPersistentContext>,
        freshness_mode: CacheFreshnessMode,
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

        let stale_fallback =
            match self.try_persistent_read_operation(key, persistent_ctx, freshness_mode) {
                Some(PersistentLookup::Fresh(result)) => {
                    let arc_result = Arc::new(result);
                    {
                        let mut state = slot.state.lock().unwrap();
                        *state = SlotState::Ready(Arc::clone(&arc_result));
                    }
                    slot.ready.notify_all();
                    self.record_write();
                    return Ok(arc_result);
                }
                Some(PersistentLookup::Stale(result))
                    if freshness_mode == CacheFreshnessMode::Fallback =>
                {
                    Some(result)
                }
                Some(PersistentLookup::Stale(_)) => None,
                None => None,
            };

        match compute() {
            Ok(result) => {
                self.try_persistent_write_operation(key, persistent_ctx, &result);

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
                if let Some(stale) = stale_fallback {
                    let arc_result = Arc::new(stale);
                    {
                        let mut state = slot.state.lock().unwrap();
                        *state = SlotState::Ready(Arc::clone(&arc_result));
                    }
                    slot.ready.notify_all();
                    self.record_stale_hit();
                    return Ok(arc_result);
                }

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

    /// Attempts to read a compose result from the persistent store.
    fn try_persistent_read_compose(
        &self,
        key: &str,
        persistent_ctx: Option<&PersistentContext>,
        freshness_mode: CacheFreshnessMode,
    ) -> Option<PersistentLookup<ComposeResult>> {
        let store = self.persistent.as_ref()?;

        // Compute the persistent entry key
        let entry_key = match persistent_ctx {
            Some(ctx) => self.compose_entry_key(ctx)?,
            None => biscuit_hash::xx_hash(key),
        };

        // Read the composed document manifest
        let mut manifest: ComposedDocumentManifest =
            match store.read_manifest(ArtifactClass::ComposeDocumentCore, entry_key) {
                Ok(Some(m)) => m,
                Ok(None) => return None,
                Err(e) => {
                    tracing::debug!("Persistent cache manifest read error for {}: {}", key, e);
                    return None;
                }
            };

        let content = self.read_blob_string(key, manifest.payload_blob_hash)?;

        let is_stale = match freshness_mode {
            CacheFreshnessMode::Optimistic | CacheFreshnessMode::Forced => false,
            CacheFreshnessMode::Strict | CacheFreshnessMode::Fallback => {
                manifest.is_expired() || !self.validate_compose_manifest(&manifest)
            }
        };

        if is_stale && freshness_mode == CacheFreshnessMode::Strict {
            tracing::debug!("Persistent compose cache entry stale for {}", key);
            return Some(PersistentLookup::Stale(ComposeResult {
                content,
                report: ComposeReport::new(),
                dependencies: manifest.dependencies.clone(),
            }));
        }

        // Update last-accessed timestamp (best-effort)
        manifest.touch();
        let _ = store.write_manifest(ArtifactClass::ComposeDocumentCore, entry_key, &manifest);

        self.stats.lock().unwrap().persistent_hits += 1;
        let result = ComposeResult {
            content,
            report: ComposeReport::new(),
            dependencies: manifest.dependencies.clone(),
        };

        if is_stale {
            Some(PersistentLookup::Stale(result))
        } else {
            Some(PersistentLookup::Fresh(result))
        }
    }

    /// Attempts to read an operation result from the persistent store.
    fn try_persistent_read_operation(
        &self,
        key: &str,
        persistent_ctx: Option<&OperationPersistentContext>,
        freshness_mode: CacheFreshnessMode,
    ) -> Option<PersistentLookup<OperationResult>> {
        let store = self.persistent.as_ref()?;
        let entry_key = persistent_ctx
            .map(|ctx| ctx.entry_key)
            .unwrap_or_else(|| biscuit_hash::xx_hash(key));

        let mut manifest: OperationResultManifest =
            match store.read_manifest(ArtifactClass::OperationResult, entry_key) {
                Ok(Some(m)) => m,
                Ok(None) => return None,
                Err(e) => {
                    tracing::debug!(
                        "Persistent operation manifest read error for {}: {}",
                        key,
                        e
                    );
                    return None;
                }
            };

        let content = self.read_blob_string(key, manifest.payload_blob_hash)?;
        let is_stale = match freshness_mode {
            CacheFreshnessMode::Optimistic | CacheFreshnessMode::Forced => false,
            CacheFreshnessMode::Strict | CacheFreshnessMode::Fallback => {
                manifest.is_expired() || !self.validate_operation_manifest(&manifest)
            }
        };

        if is_stale && freshness_mode == CacheFreshnessMode::Strict {
            tracing::debug!("Persistent operation cache entry stale for {}", key);
            return Some(PersistentLookup::Stale(OperationResult { content }));
        }

        manifest.touch();
        let _ = store.write_manifest(ArtifactClass::OperationResult, entry_key, &manifest);

        self.stats.lock().unwrap().persistent_hits += 1;
        if is_stale {
            Some(PersistentLookup::Stale(OperationResult { content }))
        } else {
            Some(PersistentLookup::Fresh(OperationResult { content }))
        }
    }

    /// Attempts to write a compose result to the persistent store (best-effort).
    fn try_persistent_write_compose(
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
        let (entry_key, source_id_hash, source_body_semantic_hash, self_hash) = match persistent_ctx
        {
            Some(ctx) => {
                let Some(snapshot) = self.get_document_snapshot(ctx.source_id) else {
                    return;
                };
                let ek = compose_entry_key(
                    ctx.source_id,
                    snapshot.body_semantic_hash,
                    ctx.state_hash,
                    ctx.context_hash,
                    ctx.options_hash,
                );
                (ek, ctx.source_id, snapshot.body_semantic_hash, blob_hash)
            }
            None => {
                let ek = biscuit_hash::xx_hash(key);
                (ek, 0, 0, blob_hash)
            }
        };

        let deps = result.dependencies.clone();
        let c_hash = closure_hash(self_hash, &deps);

        let manifest = ComposedDocumentManifest {
            cache_version: CACHE_VERSION,
            entry_key,
            source_id_hash,
            source_body_semantic_hash,
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

    /// Attempts to write an operation result to the persistent store (best-effort).
    fn try_persistent_write_operation(
        &self,
        key: &str,
        persistent_ctx: Option<&OperationPersistentContext>,
        result: &OperationResult,
    ) {
        let Some(store) = self.persistent.as_ref() else {
            return;
        };

        if self.access_mode == CacheAccessMode::ReadOnly {
            return;
        }

        let Some(ctx) = persistent_ctx else {
            return;
        };

        let blob = result.content.as_bytes();
        let blob_hash = raw_bytes_hash(blob);
        let self_hash = blob_hash;
        let closure_hash = self.operation_closure_hash(self_hash, ctx.source_content_hash);

        let manifest = OperationResultManifest {
            cache_version: CACHE_VERSION,
            entry_key: ctx.entry_key,
            op_kind: ctx.op_kind.to_string(),
            self_hash,
            closure_hash,
            payload_blob_hash: blob_hash,
            canonical_source: ctx.canonical_source.clone(),
            source_id_hash: ctx.source_id,
            source_content_hash: ctx.source_content_hash,
            created_at: std::time::SystemTime::now(),
            last_accessed_at: std::time::SystemTime::now(),
            expires_at: None,
        };

        if let Err(e) = store.write_artifact(
            ArtifactClass::OperationResult,
            ctx.entry_key,
            &manifest,
            blob,
            blob_hash,
            "md",
        ) {
            tracing::debug!("Persistent operation cache write error for {}: {}", key, e);
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

        if self.access_mode == CacheAccessMode::Off || self.access_mode == CacheAccessMode::ReadOnly
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
                let current_modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                if let Ok(Some(existing)) = store.read_manifest::<DocumentSnapshotManifest>(
                    ArtifactClass::DocumentSnapshot,
                    sid_hash,
                ) && existing.is_fresh(current_modified, current_size)
                {
                    self.cache_document_snapshot(existing);
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

        self.cache_document_snapshot(manifest.clone());

        if let Err(e) = store.write_manifest(ArtifactClass::DocumentSnapshot, sid_hash, &manifest) {
            tracing::debug!("Failed to write document snapshot for {:?}: {}", path, e);
        }
    }

    fn cache_document_snapshot(&self, manifest: DocumentSnapshotManifest) {
        self.document_snapshots
            .insert(manifest.source_id_hash, manifest);
    }

    fn get_document_snapshot(&self, source_id: u64) -> Option<DocumentSnapshotManifest> {
        if let Some(snapshot) = self.document_snapshots.get(&source_id) {
            let snapshot = snapshot.clone();
            return Some(snapshot);
        }

        let store = self.persistent.as_ref()?;
        let snapshot: DocumentSnapshotManifest = store
            .read_manifest(ArtifactClass::DocumentSnapshot, source_id)
            .ok()
            .flatten()?;
        self.cache_document_snapshot(snapshot.clone());
        Some(snapshot)
    }

    fn compose_entry_key(&self, ctx: &PersistentContext) -> Option<u64> {
        use super::hashing::compose_entry_key;

        let snapshot = self.get_document_snapshot(ctx.source_id)?;
        Some(compose_entry_key(
            ctx.source_id,
            snapshot.body_semantic_hash,
            ctx.state_hash,
            ctx.context_hash,
            ctx.options_hash,
        ))
    }

    fn read_blob_string(&self, key: &str, blob_hash: u64) -> Option<String> {
        let store = self.persistent.as_ref()?;
        let blob = match store.read_blob(blob_hash, "md") {
            Ok(Some(data)) => data,
            Ok(None) => {
                tracing::debug!(
                    "Persistent cache blob missing for {} (hash {:016x})",
                    key,
                    blob_hash
                );
                return None;
            }
            Err(e) => {
                tracing::debug!("Persistent cache blob read error for {}: {}", key, e);
                return None;
            }
        };

        String::from_utf8(blob).ok()
    }

    fn validate_compose_manifest(&self, manifest: &ComposedDocumentManifest) -> bool {
        self.record_revalidation();

        let Some(snapshot) = self.get_document_snapshot(manifest.source_id_hash) else {
            return false;
        };
        if snapshot.body_semantic_hash != manifest.source_body_semantic_hash {
            return false;
        }

        for dep in &manifest.dependencies {
            let Some(current_closure_hash) = self.resolve_dependency_closure_hash(dep) else {
                return false;
            };
            if current_closure_hash != dep.closure_hash {
                return false;
            }
        }

        true
    }

    fn validate_operation_manifest(&self, manifest: &OperationResultManifest) -> bool {
        self.record_revalidation();

        let current_bytes = match std::fs::read(&manifest.canonical_source) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        raw_bytes_hash(&current_bytes) == manifest.source_content_hash
    }

    fn resolve_dependency_closure_hash(&self, dependency: &DependencyRef) -> Option<u64> {
        let store = self.persistent.as_ref()?;

        match dependency.artifact_class {
            ArtifactClass::ComposeDocumentCore => {
                let manifest: ComposedDocumentManifest = store
                    .read_manifest(ArtifactClass::ComposeDocumentCore, dependency.entry_key)
                    .ok()
                    .flatten()?;
                if !self.validate_compose_manifest(&manifest) {
                    return None;
                }
                Some(manifest.closure_hash)
            }
            ArtifactClass::OperationResult => {
                let manifest: OperationResultManifest = store
                    .read_manifest(ArtifactClass::OperationResult, dependency.entry_key)
                    .ok()
                    .flatten()?;
                if !self.validate_operation_manifest(&manifest) {
                    return None;
                }
                Some(manifest.closure_hash)
            }
            ArtifactClass::RemoteUrl => {
                // The remote dependency's current closure value is the content
                // hash of the cached body. A re-fetch that changes the body
                // updates this hash, invalidating any parent that referenced
                // the prior content.
                let manifest: RemoteUrlManifest = store
                    .read_manifest(ArtifactClass::RemoteUrl, dependency.entry_key)
                    .ok()
                    .flatten()?;

                // Reading only the persisted hash would accept a body that the
                // active RemoteReadConfig (--remote-refresh, expired TTL) should
                // have revalidated — the bug where a cached local child bypasses
                // remote freshness. Drive the revalidation through the run's
                // fetch runtime first, then read its (possibly updated) hash.
                // Single-flight makes this a no-op if the URL is already in
                // flight this run, and it blocks until the fetch settles.
                if let Some(remote_fetch) = self.remote_fetch.as_ref()
                    && let Ok(url) = url::Url::parse(&manifest.url)
                {
                    remote_fetch.register_and_fetch(url.clone());
                    // `None` means the fetch failed with no stale fallback (e.g.
                    // a Strict revalidation error); treat the dependency as
                    // invalid so the parent recomputes and surfaces the failure.
                    return remote_fetch.content_hash(&url);
                }

                Some(manifest.content_hash)
            }
            ArtifactClass::DocumentSnapshot => None,
        }
    }

    fn operation_closure_hash(&self, self_hash: u64, source_content_hash: u64) -> u64 {
        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&self_hash.to_le_bytes());
        data.extend_from_slice(&source_content_hash.to_le_bytes());
        biscuit_hash::xx_hash_bytes(&data)
    }

    pub fn compose_dependency_ref(
        &self,
        persistent_ctx: &PersistentContext,
    ) -> Option<DependencyRef> {
        let store = self.persistent.as_ref()?;
        let entry_key = self.compose_entry_key(persistent_ctx)?;
        let manifest: ComposedDocumentManifest = store
            .read_manifest(ArtifactClass::ComposeDocumentCore, entry_key)
            .ok()
            .flatten()?;
        Some(DependencyRef {
            artifact_class: ArtifactClass::ComposeDocumentCore,
            entry_key,
            source_id_hash: manifest.source_id_hash,
            closure_hash: manifest.closure_hash,
        })
    }

    pub fn operation_dependency_ref(
        &self,
        persistent_ctx: &OperationPersistentContext,
    ) -> Option<DependencyRef> {
        let store = self.persistent.as_ref()?;
        let manifest: OperationResultManifest = store
            .read_manifest(ArtifactClass::OperationResult, persistent_ctx.entry_key)
            .ok()
            .flatten()?;
        Some(DependencyRef {
            artifact_class: ArtifactClass::OperationResult,
            entry_key: persistent_ctx.entry_key,
            source_id_hash: manifest.source_id_hash,
            closure_hash: manifest.closure_hash,
        })
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

    fn record_revalidation(&self) {
        self.stats.lock().unwrap().revalidations += 1;
    }

    fn record_stale_hit(&self) {
        self.stats.lock().unwrap().stale_hits += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::cache::operation::CacheableOperation;
    use crate::markdown::compose::cache::operation::CodeOperation;
    use std::fs;
    use tempfile::tempdir;

    fn persistent_cache() -> (tempfile::TempDir, RunLocalCache) {
        let dir = tempdir().unwrap();
        let cache = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        (dir, cache)
    }

    #[test]
    fn cache_off_always_computes() {
        let cache = RunLocalCache::new(CacheAccessMode::Off);
        let mut call_count = 0;

        for _ in 0..3 {
            let result = cache
                .get_or_compute_compose("key1", None, CacheFreshnessMode::Strict, true, || {
                    call_count += 1;
                    Ok(ComposeResult {
                        content: format!("result-{}", call_count),
                        report: ComposeReport::new(),
                        dependencies: vec![],
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
            .get_or_compute_compose("key1", None, CacheFreshnessMode::Strict, true, || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ComposeResult {
                    content: "hello".to_string(),
                    report: ComposeReport::new(),
                    dependencies: vec![],
                })
            })
            .unwrap();

        let result2 = cache
            .get_or_compute_compose("key1", None, CacheFreshnessMode::Strict, true, || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ComposeResult {
                    content: "should not compute".to_string(),
                    report: ComposeReport::new(),
                    dependencies: vec![],
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
                            None,
                            CacheFreshnessMode::Strict,
                            true,
                            || {
                                compute_count.fetch_add(1, Ordering::SeqCst);
                                std::thread::sleep(Duration::from_millis(50));
                                Ok(ComposeResult {
                                    content: "shared-result".to_string(),
                                    report: ComposeReport::new(),
                                    dependencies: vec![],
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
            .get_or_compute_compose("key1", None, CacheFreshnessMode::Strict, true, || {
                Ok(ComposeResult {
                    content: "original".to_string(),
                    report: ComposeReport::new(),
                    dependencies: vec![],
                })
            })
            .unwrap();

        // Create a refresh-mode view sharing the same backing stores
        let refresh_cache = RunLocalCache {
            access_mode: CacheAccessMode::Refresh,
            markdown_documents: Arc::clone(&cache.markdown_documents),
            toc_headings: Arc::clone(&cache.toc_headings),
            document_snapshots: Arc::clone(&cache.document_snapshots),
            compose_results: Arc::clone(&cache.compose_results),
            operation_results: Arc::clone(&cache.operation_results),
            stats: Arc::clone(&cache.stats),
            persistent: cache.persistent.clone(),
            remote_fetch: cache.remote_fetch.clone(),
        };

        let result = refresh_cache
            .get_or_compute_compose("key1", None, CacheFreshnessMode::Strict, true, || {
                Ok(ComposeResult {
                    content: "refreshed".to_string(),
                    report: ComposeReport::new(),
                    dependencies: vec![],
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
                .get_or_compute_operation("op-key1", None, CacheFreshnessMode::Strict, || {
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
            .get_or_compute_operation("op-key1", None, CacheFreshnessMode::Strict, || {
                call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(OperationResult {
                    content: "code-block".to_string(),
                })
            })
            .unwrap();

        let result2 = cache
            .get_or_compute_operation("op-key1", None, CacheFreshnessMode::Strict, || {
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
                            None,
                            CacheFreshnessMode::Strict,
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
            .get_or_compute_compose("shared-key", None, CacheFreshnessMode::Strict, true, || {
                Ok(ComposeResult {
                    content: "compose-content".to_string(),
                    report: ComposeReport::new(),
                    dependencies: vec![],
                })
            })
            .unwrap();

        let op_result = cache
            .get_or_compute_operation("shared-key", None, CacheFreshnessMode::Strict, || {
                Ok(OperationResult {
                    content: "operation-content".to_string(),
                })
            })
            .unwrap();

        // Operation should NOT hit the compose cache
        assert_eq!(op_result.content, "operation-content");
    }

    #[test]
    fn compose_dependency_invalidation_detects_child_source_changes() {
        let (_dir, cache) = persistent_cache();
        let workspace = tempdir().unwrap();
        let parent_path = workspace.path().join("parent.md");
        let child_path = workspace.path().join("child.md");

        fs::write(&parent_path, "# Parent\n\n::file ./child.md\n").unwrap();
        fs::write(&child_path, "# Child\n\nVersion 1\n").unwrap();

        cache.load_markdown(&parent_path).unwrap();
        cache.load_markdown(&child_path).unwrap();

        let parent_source = compose_cache_key(&parent_path);
        let child_source = compose_cache_key(&child_path);
        let parent_ctx = PersistentContext {
            source_id: source_id_hash(&parent_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };
        let child_ctx = PersistentContext {
            source_id: source_id_hash(&child_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };

        cache
            .get_or_compute_compose(
                "child",
                Some(&child_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Child\n\nVersion 1\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();

        let child_dep = cache.compose_dependency_ref(&child_ctx).unwrap();

        cache
            .get_or_compute_compose(
                "parent",
                Some(&parent_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Parent\n\n# Child\n\nVersion 1\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![child_dep.clone()],
                    })
                },
            )
            .unwrap();

        fs::write(&child_path, "# Child\n\nVersion 2\n").unwrap();

        let refresh_cache = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(_dir.path().join("cache"));
        refresh_cache.load_markdown(&child_path).unwrap();

        let lookup = refresh_cache.try_persistent_read_compose(
            "parent",
            Some(&parent_ctx),
            CacheFreshnessMode::Strict,
        );
        assert!(
            matches!(lookup, Some(PersistentLookup::Stale(_))),
            "expected stale parent cache entry after child change"
        );
    }

    #[test]
    fn persistent_operation_cache_hits_across_runs() {
        let (dir, cache) = persistent_cache();
        let workspace = tempdir().unwrap();
        let source_path = workspace.path().join("main.rs");
        fs::write(&source_path, "fn main() {}\n").unwrap();

        let canonical_source = compose_cache_key(&source_path);
        let source_id = source_id_hash(&canonical_source);
        let source_content_hash = raw_bytes_hash(&fs::read(&source_path).unwrap());
        let op = CodeOperation;
        let entry_key = op.variant_cache_key(source_id, &Default::default());
        let persistent_ctx = OperationPersistentContext {
            op_kind: "code",
            entry_key,
            source_id,
            canonical_source,
            source_content_hash,
        };

        cache
            .get_or_compute_operation(
                "code-op",
                Some(&persistent_ctx),
                CacheFreshnessMode::Strict,
                || {
                    Ok(OperationResult {
                        content: "```rust\nfn main() {}\n```".to_string(),
                    })
                },
            )
            .unwrap();

        let hit_cache = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        let compute_count = std::sync::atomic::AtomicUsize::new(0);

        let result = hit_cache
            .get_or_compute_operation(
                "code-op",
                Some(&persistent_ctx),
                CacheFreshnessMode::Strict,
                || {
                    compute_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(OperationResult {
                        content: "should not compute".to_string(),
                    })
                },
            )
            .unwrap();

        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(result.content, "```rust\nfn main() {}\n```");
    }

    #[test]
    fn persistent_compose_cache_hits_across_runs() {
        let (dir, cache) = persistent_cache();
        let workspace = tempdir().unwrap();
        let source_path = workspace.path().join("doc.md");
        fs::write(&source_path, "# Title\n\nBody\n").unwrap();

        cache.load_markdown(&source_path).unwrap();

        let canonical_source = compose_cache_key(&source_path);
        let persistent_ctx = PersistentContext {
            source_id: source_id_hash(&canonical_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };

        cache
            .get_or_compute_compose(
                "compose-op",
                Some(&persistent_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Title\n\nBody\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();

        let hit_cache = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        hit_cache.load_markdown(&source_path).unwrap();
        let compute_count = std::sync::atomic::AtomicUsize::new(0);

        let result = hit_cache
            .get_or_compute_compose(
                "compose-op",
                Some(&persistent_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    compute_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(ComposeResult {
                        content: "should not compute".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();

        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(result.content, "# Title\n\nBody\n");
    }

    #[test]
    fn persistent_ineligible_key_never_reads_or_writes_persistent_store() {
        let (dir, cache) = persistent_cache();
        let workspace = tempdir().unwrap();
        let source_path = workspace.path().join("doc.md");
        fs::write(&source_path, "# Title\n\nBody\n").unwrap();
        cache.load_markdown(&source_path).unwrap();

        let canonical_source = compose_cache_key(&source_path);
        let persistent_ctx = PersistentContext {
            source_id: source_id_hash(&canonical_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };

        // Ineligible: run-local single-flight still works, but nothing is
        // written to the persistent store.
        cache
            .get_or_compute_compose(
                "ineligible-key",
                Some(&persistent_ctx),
                CacheFreshnessMode::Strict,
                false,
                || {
                    Ok(ComposeResult {
                        content: "# Title\n\nBody\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();
        assert_eq!(
            cache.stats().persistent_writes,
            0,
            "ineligible key must not write persistent cache"
        );

        // A fresh cache over the same store cannot read that (never-written)
        // entry back, so it recomputes rather than serving a persistent hit.
        let second = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        second.load_markdown(&source_path).unwrap();
        let recomputed = std::sync::atomic::AtomicUsize::new(0);
        second
            .get_or_compute_compose(
                "ineligible-key",
                Some(&persistent_ctx),
                CacheFreshnessMode::Strict,
                false,
                || {
                    recomputed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(ComposeResult {
                        content: "# Title\n\nBody\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();
        assert_eq!(
            recomputed.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "no persistent entry exists to serve, so the closure runs"
        );
        assert_eq!(second.stats().persistent_hits, 0);

        // Contrast: the same key when eligible writes through to the store.
        let eligible = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        eligible.load_markdown(&source_path).unwrap();
        eligible
            .get_or_compute_compose(
                "eligible-key",
                Some(&persistent_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Title\n\nBody\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();
        assert_eq!(
            eligible.stats().persistent_writes,
            1,
            "eligible key writes persistent cache"
        );
    }

    #[test]
    fn fallback_mode_serves_stale_operation_when_recompute_fails() {
        let (dir, cache) = persistent_cache();
        let workspace = tempdir().unwrap();
        let source_path = workspace.path().join("main.rs");
        fs::write(&source_path, "fn main() {}\n").unwrap();

        let canonical_source = compose_cache_key(&source_path);
        let source_id = source_id_hash(&canonical_source);
        let op = CodeOperation;
        let entry_key = op.variant_cache_key(source_id, &Default::default());

        let initial_ctx = OperationPersistentContext {
            op_kind: "code",
            entry_key,
            source_id,
            canonical_source: canonical_source.clone(),
            source_content_hash: raw_bytes_hash(&fs::read(&source_path).unwrap()),
        };

        cache
            .get_or_compute_operation(
                "code-op",
                Some(&initial_ctx),
                CacheFreshnessMode::Strict,
                || {
                    Ok(OperationResult {
                        content: "cached content".to_string(),
                    })
                },
            )
            .unwrap();

        fs::write(&source_path, "fn main() { println!(\"changed\"); }\n").unwrap();

        let fallback_ctx = OperationPersistentContext {
            op_kind: "code",
            entry_key,
            source_id,
            canonical_source,
            source_content_hash: raw_bytes_hash(&fs::read(&source_path).unwrap()),
        };

        let fallback_cache = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        let result = fallback_cache
            .get_or_compute_operation(
                "code-op",
                Some(&fallback_ctx),
                CacheFreshnessMode::Fallback,
                || Err(std::io::Error::other("boom").into()),
            )
            .unwrap();

        assert_eq!(result.content, "cached content");
        assert_eq!(fallback_cache.stats().stale_hits, 1);
    }

    #[test]
    fn fallback_mode_serves_stale_compose_when_recompute_fails() {
        let (dir, cache) = persistent_cache();
        let workspace = tempdir().unwrap();
        let parent_path = workspace.path().join("parent.md");
        let child_path = workspace.path().join("child.md");
        fs::write(&parent_path, "# Parent\n\n::file ./child.md\n").unwrap();
        fs::write(&child_path, "# Child\n\nVersion 1\n").unwrap();

        cache.load_markdown(&parent_path).unwrap();
        cache.load_markdown(&child_path).unwrap();

        let parent_source = compose_cache_key(&parent_path);
        let child_source = compose_cache_key(&child_path);
        let parent_ctx = PersistentContext {
            source_id: source_id_hash(&parent_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };
        let child_ctx = PersistentContext {
            source_id: source_id_hash(&child_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };

        cache
            .get_or_compute_compose(
                "child",
                Some(&child_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Child\n\nVersion 1\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();

        let child_dep = cache.compose_dependency_ref(&child_ctx).unwrap();

        cache
            .get_or_compute_compose(
                "parent",
                Some(&parent_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Parent\n\n# Child\n\nVersion 1\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![child_dep.clone()],
                    })
                },
            )
            .unwrap();

        fs::write(&child_path, "# Child\n\nVersion 2\n").unwrap();

        let fallback_cache = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        fallback_cache.load_markdown(&child_path).unwrap();

        let result = fallback_cache
            .get_or_compute_compose(
                "parent",
                Some(&parent_ctx),
                CacheFreshnessMode::Fallback,
                true,
                || Err(std::io::Error::other("boom").into()),
            )
            .unwrap();

        assert_eq!(result.content, "# Parent\n\n# Child\n\nVersion 1\n");
        assert_eq!(fallback_cache.stats().stale_hits, 1);
    }

    #[test]
    fn optimistic_mode_accepts_stale_compose_without_recompute() {
        let (dir, cache) = persistent_cache();
        let workspace = tempdir().unwrap();
        let parent_path = workspace.path().join("parent.md");
        let child_path = workspace.path().join("child.md");
        fs::write(&parent_path, "# Parent\n\n::file ./child.md\n").unwrap();
        fs::write(&child_path, "# Child\n\nVersion 1\n").unwrap();

        cache.load_markdown(&parent_path).unwrap();
        cache.load_markdown(&child_path).unwrap();

        let parent_source = compose_cache_key(&parent_path);
        let child_source = compose_cache_key(&child_path);
        let parent_ctx = PersistentContext {
            source_id: source_id_hash(&parent_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };
        let child_ctx = PersistentContext {
            source_id: source_id_hash(&child_source),
            state_hash: 0,
            context_hash: 0,
            options_hash: 0,
        };

        cache
            .get_or_compute_compose(
                "child",
                Some(&child_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Child\n\nVersion 1\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();

        let child_dep = cache.compose_dependency_ref(&child_ctx).unwrap();

        cache
            .get_or_compute_compose(
                "parent",
                Some(&parent_ctx),
                CacheFreshnessMode::Strict,
                true,
                || {
                    Ok(ComposeResult {
                        content: "# Parent\n\n# Child\n\nVersion 1\n".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![child_dep.clone()],
                    })
                },
            )
            .unwrap();

        fs::write(&child_path, "# Child\n\nVersion 2\n").unwrap();

        let optimistic_cache = RunLocalCache::new(CacheAccessMode::ReadWrite)
            .with_persistent(dir.path().join("cache"));
        optimistic_cache.load_markdown(&child_path).unwrap();
        let compute_count = std::sync::atomic::AtomicUsize::new(0);

        let result = optimistic_cache
            .get_or_compute_compose(
                "parent",
                Some(&parent_ctx),
                CacheFreshnessMode::Optimistic,
                true,
                || {
                    compute_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(ComposeResult {
                        content: "should not compute".to_string(),
                        report: ComposeReport::new(),
                        dependencies: vec![],
                    })
                },
            )
            .unwrap();

        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(result.content, "# Parent\n\n# Child\n\nVersion 1\n");
    }
}
