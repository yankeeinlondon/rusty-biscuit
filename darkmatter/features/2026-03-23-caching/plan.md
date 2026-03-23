---
last_updated: 2026-03-23
sources:
  - "darkmatter/features/2026-03-23. caching/design.md"
---

# Compose Pipeline Caching -- Implementation Plan

## Context

Darkmatter's compose pipeline performs recursive, concurrent transclusion of markdown documents. Today the pipeline has a simple path-keyed in-memory cache for markdown loads and TOC headings, but no single-flight deduplication of concurrent compose work and no persistent cross-run caching. As transclusion workloads grow (remote fetches, future LLM operations), the cost of recomputation becomes significant.

This plan implements the design in `darkmatter/features/2026-03-23. caching/design.md`: a two-layer cache (run-local with single-flight + persistent file-backed artifacts) with Merkle-style dependency invalidation.

---

## Phase 1: Generalize the Run-Local Cache with Single-Flight

**Goal**: Extract caching into its own module, add single-flight behavior for child compose work, surface cache stats in `ComposeReport`.

### Step 1.1: Create the cache module

```
darkmatter/lib/src/markdown/compose/cache/
  mod.rs          -- CacheRuntime, re-exports
  runtime.rs      -- RunLocalCache (in-memory single-flight)
  types.rs        -- CacheAccessMode, CacheFreshnessMode, CacheStats
  hashing.rs      -- Hash helpers wrapping biscuit-hash (stubs in Phase 1, full in Phase 2)
```

Register in `darkmatter/lib/src/markdown/compose/mod.rs` as `pub(crate) mod cache;`.

### Step 1.2: Define core types (`cache/types.rs`)

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CacheAccessMode {
    Off,
    ReadOnly,
    #[default]
    ReadWrite,
    Refresh,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CacheFreshnessMode {
    #[default]
    Strict,
    Fallback,
    Optimistic,
    Forced,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub writes: usize,
    pub inflight_waits: usize,
    pub errors: usize,
}
```

### Step 1.3: Implement `RunLocalCache` (`cache/runtime.rs`)

Single-flight using `std::sync` primitives (rayon is sync, not async):

```rust
use std::sync::{Arc, Mutex, Condvar};

enum SlotState<T> {
    InFlight,
    Ready(Arc<T>),
    Failed(String),
}

struct SingleFlightSlot<T> {
    state: Mutex<SlotState<T>>,
    ready: Condvar,
}
```

`RunLocalCache` struct replaces the existing `PipelineCache`:

```rust
pub(crate) struct RunLocalCache {
    markdown_documents: Arc<Mutex<HashMap<String, Markdown>>>,
    toc_headings: Arc<Mutex<HashMap<String, Vec<MarkdownTocNode>>>>,
    compose_results: Arc<Mutex<HashMap<String, Arc<SingleFlightSlot<ComposeResult>>>>>,
    negative_cache: Arc<Mutex<HashSet<String>>>,
    stats: Arc<Mutex<CacheStats>>,
}
```

Single-flight pattern:
1. Lock map, check for existing slot
2. If `Ready` -- clone result, increment `stats.hits`, return
3. If `InFlight` -- clone Arc to slot, drop map lock, wait on Condvar, read result
4. If absent -- insert `InFlight` slot, drop map lock, compute, lock map, replace with `Ready`, notify all

**Risk**: Rayon deadlock if all pool threads block on slots. Mitigate with `Condvar::wait_timeout` (e.g. 30s) + fallback to duplicate computation on timeout.

### Step 1.4: Refactor `PipelineRuntime`

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:446-531`

- Replace `cache: PipelineCache` with `pub(crate) cache: RunLocalCache`
- Remove `PipelineCache` struct
- `clone_for_child()` clones `RunLocalCache` Arc handles (shared, as before)
- `load_markdown()` and `load_toc_headings()` delegate to `RunLocalCache` methods
- `merge_child()` unchanged

### Step 1.5: Add cache key generation (`cache/hashing.rs`)

Phase 1 stub: cache key = canonical path string (same as current behavior). Full content-aware hashing comes in Phase 2.

```rust
pub(crate) fn compose_core_cache_key(source_path: &Path) -> String {
    std::fs::canonicalize(source_path)
        .unwrap_or_else(|_| source_path.to_path_buf())
        .to_string_lossy()
        .to_string()
}
```

### Step 1.6: Integrate single-flight into transclusion resolution

**File**: `darkmatter/lib/src/markdown/compose/mod.rs` (around line 1050-1082, `PreparedTransclusion::Markdown` arm)

Refactor `render_markdown_transclusion` conceptually into:
1. Compute cache key for this child compose
2. `runtime.cache.get_or_compute(key, || { load + compose child })` -- returns `Arc<(String, ComposeReport)>`
3. Clone composed content and report from cache result
4. Apply post-cache transforms: `exclude`, releveling, wrappers, spacing

The cached artifact is the **composed child core** (after recursive compose, before parent-specific transforms). This matches the design principle of caching expensive work, not cheap insertion work.

### Step 1.7: Add `cache_stats` to `ComposeReport`

**File**: `darkmatter/lib/src/markdown/compose/types.rs`

```rust
// Add to ComposeReport
pub cache_stats: Option<CacheStats>,
```

- Harvest stats from `runtime.cache` at end of `run_compose_pipeline` (line ~320)
- Update `ComposeReport::merge()` to aggregate stats
- Update `summary()` to include cache info

### Step 1.8: Add `cache_access_mode` to `ComposeOptions`

**File**: `darkmatter/lib/src/markdown/compose/types.rs`

```rust
pub cache_access_mode: CacheAccessMode,
```

Default: `ReadWrite`. When `Off`, bypass single-flight and compute directly (backward-compatible).

### Phase 1 Testing

- **Unit**: Single-flight contention test with `std::thread::scope` -- multiple threads request same key, verify only one compute runs
- **Unit**: `CacheStats` accumulation correctness
- **Integration**: Document with duplicate `::file` directives to same target -- verify `cache_stats.hits > 0`
- **Regression**: Run all existing compose tests with `CacheAccessMode::Off` to verify identical output

### Phase 1 Files Summary

| Action | Path |
|--------|------|
| Create | `darkmatter/lib/src/markdown/compose/cache/mod.rs` |
| Create | `darkmatter/lib/src/markdown/compose/cache/runtime.rs` |
| Create | `darkmatter/lib/src/markdown/compose/cache/types.rs` |
| Create | `darkmatter/lib/src/markdown/compose/cache/hashing.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/mod.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/types.rs` |

---

## Phase 2: Persistent File-Backed Cache

**Goal**: Add workspace-local persistent storage for document snapshots and composed document core artifacts with Merkle-style closure hashes.

### Step 2.1: Add persistent types to `cache/types.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactClass { DocumentSnapshot, ComposeDocumentCore, OperationResult }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind { LocalFile, RemoteUrl }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRef { pub source_id_hash: u64, pub closure_hash: u64 }
```

Extend `CacheStats`:
```rust
pub persistent_hits: usize,
pub persistent_writes: usize,
pub revalidations: usize,
pub stale_hits: usize,
```

### Step 2.2: Create manifest structs (`cache/manifest.rs`)

**DocumentSnapshotManifest**: cache_version, source_kind, canonical_source, source_id_hash, raw_bytes_hash, frontmatter_hash, body_semantic_hash, body_template_hash, modified_at, size_bytes

**ComposedDocumentManifest**: cache_version, entry_key, self_hash, closure_hash, dependency_count, dependencies (Vec<DependencyRef>), payload_blob_hash, warnings_hash, created_at, last_accessed_at, expires_at

### Step 2.3: Implement full hashing (`cache/hashing.rs`)

Replace Phase 1 stubs with content-aware hashing using `biscuit_hash`:

- `source_id_hash(canonical_source: &str) -> u64` -- `xx_hash`
- `raw_bytes_hash(content: &[u8]) -> u64` -- `xx_hash_bytes`
- `body_semantic_hash(body: &str) -> u64` -- `xx_hash_variant` with `[BlockTrimming, LeadingWhitespace, TrailingWhitespace]`
- `body_template_hash(body: &str) -> u64` -- placeholder-normalized body, more aggressive normalization
- `frontmatter_hash(fm: &Map<String, Value>) -> u64` -- canonical JSON with recursively sorted keys
- `effective_state_hash(state: &EffectiveState) -> u64` -- canonical JSON
- `context_hash(ctx: &ComposeContext) -> u64` -- canonical JSON
- `options_hash(options: &ComposeOptions) -> u64` -- only output-affecting fields
- `closure_hash(self_hash: u64, deps: &[(u64, u64)]) -> u64` -- Merkle-style

Helper: `canonical_json_sorted(value: &Value) -> String` -- recursively sort object keys

### Step 2.4: Create persistent backend (`cache/store.rs`)

```rust
pub(crate) struct FileStore {
    root: PathBuf,  // <workspace>/.darkmatter/cache/v1/
}
```

Methods:
- `new(root)` -- creates directory structure
- `resolve_cache_root(source: &ComposeSource) -> PathBuf` -- workspace-local or `dirs::cache_dir()` fallback
- `read_manifest<T: DeserializeOwned>(class, key) -> Result<Option<T>>`
- `write_artifact(class, key, manifest, blob, ext) -> Result<()>` -- atomic via tempfile + rename
- `read_blob(hash, ext) -> Result<Option<Vec<u8>>>`
- Lock files in `locks/` using `O_CREAT | O_EXCL` for cross-process safety

On-disk layout with hash fanout:
```
.darkmatter/cache/v1/
  manifests/{class}/{ab}/{cd}/{hex}.json
  blobs/{ext}/{ab}/{cd}/{hex}.{ext}
  locks/
```

### Step 2.5: Integrate persistent cache

Modify compose flow in `render_markdown_transclusion`:
1. Compute full cache key (source_id + body_semantic + state + context + options hashes)
2. Check run-local cache (Phase 1 single-flight)
3. On miss, check persistent store for matching manifest
4. Validate closure hash against current dependency state
5. On persistent hit, load blob, populate run-local cache, return
6. On persistent miss, compute, write to persistent store + run-local cache

### Step 2.6: Document snapshot creation

During `load_markdown()`, compute and store `DocumentSnapshotManifest` when persistent caching is enabled. Snapshots provide the foundation for validating composed document manifests.

### Step 2.7: New `ComposeOptions` fields

```rust
pub cache_freshness_mode: CacheFreshnessMode,
pub cache_root: Option<PathBuf>,
pub cache_namespace: Option<String>,
```

### Phase 2 Testing

- **Unit**: `FileStore` write/read round-trip, atomic write safety, fanout directory creation
- **Unit**: All hashing functions -- determinism, sensitivity to meaningful changes, insensitivity to whitespace-only
- **Unit**: `canonical_json_sorted` -- key ordering, nested objects, arrays
- **Integration**: Compose same document twice -- verify persistent hit on second run
- **Integration**: Modify source file between runs -- verify cache miss
- **Integration**: Three-level chain (A includes B includes C) -- change C, verify B and A both invalidate via closure hash
- **Integration**: Namespace isolation -- different namespaces produce independent entries
- All tests use `tempfile::TempDir` for cache roots

### Phase 2 Files Summary

| Action | Path |
|--------|------|
| Create | `darkmatter/lib/src/markdown/compose/cache/manifest.rs` |
| Create | `darkmatter/lib/src/markdown/compose/cache/store.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/cache/hashing.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/cache/types.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/cache/mod.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/mod.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/types.rs` |

---

## Phase 3: Operation-Level Caching with Parameter Buckets

**Goal**: Formalize conditional/pre/variant/post parameter model, extend caching to `::code` and `::toc-linking`.

### Step 3.1: Define `CacheableOperation` trait (`cache/operation.rs`)

```rust
pub(crate) struct ParamBuckets {
    pub conditional: Vec<(String, String)>,
    pub pre: Vec<(String, String)>,
    pub variant: Vec<(String, String)>,
    pub post: Vec<(String, String)>,
}

pub(crate) trait CacheableOperation {
    fn split_params(&self, options: &BlockOptions) -> ParamBuckets;
    fn variant_cache_key(&self, source_id: u64, buckets: &ParamBuckets, state: &EffectiveState, context: &ComposeContext) -> u64;
    fn artifact_class(&self) -> ArtifactClass;
}
```

### Step 3.2: Implement for each operation

**`::file`**:
- conditional: `when`
- variant: source path, inherited state, replace behavior
- post: `exclude`, `quotation`, `disclosure`, releveling

**`::code`**:
- conditional: `when`
- variant: source path, effective replace map, language
- post: `quotation`, `disclosure`, spacing

**`::toc-linking`**:
- conditional: `when`
- variant: source path, TOC formatting options
- post: `quotation`, `disclosure`

### Step 3.3: Add `OperationResultManifest` (`cache/manifest.rs`)

```rust
pub struct OperationResultManifest {
    pub cache_version: u16,
    pub entry_key: u64,
    pub op_kind: String,
    pub self_hash: u64,
    pub payload_blob_hash: u64,
    pub source_id_hash: u64,
    pub created_at: SystemTime,
    pub last_accessed_at: SystemTime,
}
```

### Step 3.4: Refactor code and TOC transclusion

Extract core computation from post-processing in each:
- `render_code_transclusion`: core = read file + apply replacements + fence. Post = wrappers.
- TOC linking: core = load headings + render links. Post = wrappers.

Each gets a cache check before compute using the operation's variant key.

### Step 3.5: Wire single-flight for operation results

Extend `RunLocalCache` with operation result slots. Same pattern as compose results.

### Phase 3 Testing

- **Unit**: `split_params` on each operation -- correct bucket classification
- **Integration**: Duplicate `::code` directives to same file -- verify cache hit
- **Integration**: Duplicate `::toc-linking` to same target -- verify single compute
- **Integration**: Same file transcluded with different `exclude` patterns -- verify single core entry, different final outputs

### Phase 3 Files Summary

| Action | Path |
|--------|------|
| Create | `darkmatter/lib/src/markdown/compose/cache/operation.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/cache/mod.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/cache/runtime.rs` |
| Modify | `darkmatter/lib/src/markdown/compose/mod.rs` |

---

## Phases 4-5 (Future)

**Phase 4: Remote and LLM Cache Policies** -- TTL-based revalidation, HTTP validators, pinned-vs-floating model rules. Add `cache_llm_ttl` and `cache_remote_ttl` to ComposeOptions.

**Phase 5: SQLite Backend** -- Only if file backend proves limiting. Replace `FileStore` with `SqliteStore` behind same interface.

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Rayon deadlock from single-flight blocking | `Condvar::wait_timeout` (30s) + fallback to duplicate computation |
| Under-keyed cache producing wrong output | Over-key in v1, write sensitivity tests for each key dimension |
| Backward compatibility regression | `CacheAccessMode::Off` must produce byte-identical output; run full test suite with Off |
| Cross-filesystem rename failure | Use `tempfile::NamedTempFile::new_in()` (same dir as target) |
| Cache corruption from crashes | Atomic write via temp+rename; manifests are human-readable JSON for debugging |

---

## Critical Files Reference

| File | Role |
|------|------|
| `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` | PipelineRuntime + PipelineCache (refactor target) |
| `darkmatter/lib/src/markdown/compose/mod.rs` | Main pipeline, render_markdown_transclusion, transclusion resolution |
| `darkmatter/lib/src/markdown/compose/types.rs` | ComposeOptions, ComposeReport, ComposeContext |
| `darkmatter/lib/src/markdown/compose/state.rs` | EffectiveState (must be hashable) |
| `darkmatter/lib/src/markdown/compose/transclusion/types.rs` | BlockDirective, BlockOptions, DirectiveKind |
| `biscuit-hash/lib/src/xx.rs` | xx_hash, xx_hash_variant, HashVariant |

---

## Verification

After each phase:

1. `just test` in `darkmatter/` -- all existing tests pass
2. `just lint` in `darkmatter/` -- no new warnings
3. Phase-specific integration tests confirm cache behavior
4. Regression: compose identical documents with `CacheAccessMode::Off` vs `ReadWrite` and diff output
