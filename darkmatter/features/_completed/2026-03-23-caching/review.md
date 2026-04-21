# Compose Pipeline Caching Feature Review

**Reviewed:** 2026-03-23
**Scope:** Full implementation review against caching spec and tech design (Phases 1-3)

## Overall Assessment

The caching implementation lays a solid foundation for both run-local single-flight execution and persistent file-backed storage. The code is clean, well-architected into modular components (`hashing`, `manifest`, `runtime`, `store`), and implements the core atomicity and single-flight primitives properly.

However, the implementation is incomplete relative to the Phase 2 and Phase 3 requirements. The Merkle-style dependency closure hash mechanism is currently stubbed out, and persistent caching is missing entirely for Phase 3 operations (Code and TOC linking). `CacheFreshnessMode` logic is defined but completely ignored during lookups.

---

## Findings

### 1. Missing Closure Hash Dependency Tracking (Phase 2)
**Severity:** High (Core functionality gap)
The design explicitly requires Merkle-style closure hashes to invalidate parents when child documents change without requiring a graph database. 

- In `runtime.rs:try_persistent_write`, dependencies are hardcoded to an empty vector (`let deps = vec![];`).
- In `runtime.rs:try_persistent_read`, the loaded manifest's `closure_hash` is never validated against the current dependency state of the document.
**Impact:** If Document A transcludes Document B, and B changes, Document A's cache entry will incorrectly remain valid because its closure hash is never recomputed/validated with B's new state.

### 2. Missing Persistent Caching for Operations (Phase 3)
**Severity:** High (Core functionality gap)
The plan requires `get_or_compute_operation` to support persistent caching for `::code` and `::toc-linking` using `OperationResultManifest`.

- Currently, `get_or_compute_operation` only checks and writes to the in-memory run-local cache (`operation_results`).
- `try_persistent_read` and `try_persistent_write` are only implemented for `ComposeResult`, not `OperationResult`.

### 3. CacheFreshnessMode is Ignored
**Severity:** Medium
`CacheFreshnessMode` (`Strict`, `Fallback`, `Optimistic`, `Forced`) is defined in `types.rs` but is never evaluated in `try_persistent_read`. 

- Currently, `if manifest.is_expired()` unconditionally drops the entry.
- `Fallback` mode should attempt recomputation and return the stale blob on failure, but this behavior is missing.

### 4. `CacheableOperation` Trait Not Fully Integrated
**Severity:** Medium
`operation.rs` defines the `CacheableOperation` trait and `ParamBuckets` to cleanly separate conditional, pre, variant, and post parameters.

- The file is marked with `#![allow(dead_code)]` and the doc comment notes it "will be used by future consumers".
- This indicates that Phase 3's refactoring of code and TOC transclusions to use these buckets for cache key generation is incomplete.

### 5. Performance: Excessive File Reads for `body_semantic_hash`
**Severity:** Medium (Ergonomics/Performance)
In `try_persistent_read`, generating the `entry_key` using a `PersistentContext` requires reading the `DocumentSnapshotManifest` from disk on every cache check just to retrieve the `body_semantic_hash`. 

- **Suggestion:** Cache the `DocumentSnapshotManifest` in memory within `RunLocalCache` during `load_markdown` so subsequent compose key computations don't require an extra disk I/O operation.

### 6. Contention: `Mutex<HashMap>` in Rayon Context
**Severity:** Low (Performance Opportunity)
`RunLocalCache` uses `Arc<Mutex<HashMap>>` for its internal maps. Since transclusions are evaluated concurrently via Rayon, wrapping the entire map in a single `Mutex` creates a bottleneck where all threads serialize during cache checks.

- **Suggestion:** Consider using `dashmap::DashMap` for concurrent, granular locking, or a sharded mutex approach.

### 7. Weak Test Coverage for Edge Cases
**Severity:** Medium (Testing gap)

- **Dependency Invalidation:** There are no integration tests verifying that modifying a child document correctly invalidates a parent document's cached compose core. (This test would fail currently due to Finding #1).
- **Freshness Modes:** No tests cover `CacheFreshnessMode` behavior.
- **Cache Persistence:** `FileStore` is well-tested, but `RunLocalCache` lacks robust tests verifying cross-run persistent hits/misses.

---

## Separation of Concerns

The division between `RunLocalCache` (single-flight deduplication) and `FileStore` (atomic persistent I/O) is highly cohesive and properly models the two-layer design. The hashing logic isolates `biscuit_hash` details well, ensuring cache keys remain deterministic.

---

## Prioritized Suggestions

### Should Fix (Before Merge)

1. **Implement Dependency Closure Hashes:** Update `get_or_compute_compose` to collect child dependency hashes and validate the `closure_hash` on reads.
2. **Wire Persistent Caching for Operations:** Add `try_persistent_read`/`write` for `OperationResult` so `::code` and `::toc-linking` survive across runs.
3. **Integrate Parameter Buckets:** Hook up the `CacheableOperation` implementations to generate variant cache keys in the pipeline.

### Should Consider

1. **Implement Freshness Modes:** Apply `CacheFreshnessMode` logic when handling expired manifests.
2. **Optimize Snapshot Lookup:** Keep `DocumentSnapshotManifest` in memory to avoid extra file reads during `compose_entry_key` generation.

### Nice to Have

1. **Migrate to `DashMap`:** Replace `Mutex<HashMap>` to reduce thread contention in the single-flight registry.
2. **Optimize `canonical_json_sorted`:** The current implementation recursively stringifies JSON, which allocates heavily. This can be optimized if performance becomes an issue.
