---
date: 2026-05-05
review: 2026-05-05-bench
phases: 5
starting_phase: 5
status: draft
---

# Performance Optimization Implementation Plan

**Review:** `sniff/reviews/2026-05-05-bench/review.md`
**Total suggestions:** 15 (1 critical / 8 important / 6 nice-to-have)

## Phase Overview

| Phase | Focus | Risk | Suggestions |
|-------|-------|------|-------------|
| 1 | Caching & early exits | Low | #5 (OnceLock), #1 (re-baseline) |
| 2 | Request-scoped short circuits | Low-Medium | #3 (GitRequest::minimal), #4 (HardwareRequest flags), #7 (skip shared walk) |
| 3 | Git & diff optimizations | Medium | #6 (pre-size HashMaps), #10 (clean-repo early exit), #12 (GitConfig cache) |
| 4 | Repo & manifest parallelism | Medium-High | #8 (parallel ManifestIndex), #9 (shared walk for detect_repo_with_inventory) |
| 5 | Nice-to-have polish | Low | #13 (defer sorting), #14 (rayon docs), #15 (sorted() method), #2 (blast-radius lazy packages) |

---

## Phase 1: Caching & Early Exits (Low Risk)

**Goal:** Eliminate repeated I/O for stateless lookups that don't change within a process lifetime.

### Suggestion #5: Cache ExecutableIndex with OnceLock

**Files to change:**

1. **`sniff/lib/src/executable_index.rs`**
   - Add `static EAGER_PATH_CACHE: OnceLock<HashMap<OsString, PathBuf>>` at module level
   - Add `static BUNDLE_INDEX_CACHE: OnceLock<HashMap<String, PathBuf>>` at module level (under `#[cfg(target_os = "macos")]`)
   - In `build_eager_path()`, wrap `scan_path_executables()` and `build_bundle_index()` calls with `get_or_init()` on the respective statics, then `.clone()` the result
   - Existing code clones the maps anyway (via `Self::build_with_bundles`), so the OnceLock only removes redundant re-scans

**Testing strategy:**
- Existing tests (`test_eager_path_index_*`) already exercise `build_eager_path()`
- Add a new test that calls `build_eager_path()` twice and verifies the second call is faster (or simply that both return consistent results)
- Existing `ENV_MUTEX`-gated tests already validate PATH manipulation correctness

### Suggestion #1: Re-baseline `languages_shallow_deep_mix` (Critical, no code change)

**Action:**
- Run `cargo bench --bench perf filesystem_languages` to establish new baseline
- Update the benchmark baseline file if one exists, or note the new expected value in the review

**No files to change.** This is purely a measurement artifact.

**Phase 1 acceptance criteria:**
- `cargo test` passes in `sniff/lib`
- `ExecutableIndex::build_eager_path()` calls `scan_path_executables()` at most once per process
- No API changes; all existing callers unaffected

---

## Phase 2: Request-Scoped Short Circuits (Low-Medium Risk)

**Goal:** Add missing request flags so callers can skip work they don't need.

### Suggestion #3: Add `GitRequest::minimal()` and skip expensive git work

**Files to change:**

1. **`sniff/lib/src/request.rs`**
   - Add `GitRequest::minimal()` preset (identical to `summary()` but will be used to signal "skip everything")
   - No existing callers change — `summary()` stays as-is for backward compat

2. **`sniff/lib/src/filesystem/git/types.rs`** — `detect_with_request()` (lines 649–726)
   - Add an `is_minimal` check: `request.commit_count == 0 && !request.include_file_changes && !request.include_worktrees`
   - When `is_minimal`:
     - Skip `get_remotes()` — return empty `Vec`
     - Skip `get_local_branches()` — return empty `Vec`
     - Skip `get_tracking_status()` — return empty `Vec`
     - Skip `get_git_config()` — return `GitConfig::default()`
   - The status path already skips `get_repo_status_with_changes` when `!include_file_changes`, so no change needed there
   - `recent` commits already skipped when `commit_count == 0`

**Testing strategy:**
- Add unit test: `GitRequest::minimal()` produces expected field values
- Add integration test: `detect_with_request(&GitRequest::minimal())` returns a `GitInfo` with empty branches, remotes, tracking, and default config
- Existing `detect_with_request` tests continue to pass (they use `summary()`, `full()`, `deep()`)

### Suggestion #4: Add `include_cpu` / `include_memory` to HardwareRequest

**Files to change:**

1. **`sniff/lib/src/request.rs`** — `HardwareRequest`
   - Add fields: `pub include_cpu: bool`, `pub include_memory: bool`
   - Update `summary()`: `include_cpu: true, include_memory: true` (backward compatible)
   - Update `full()`: `include_cpu: true, include_memory: true`
   - Add builder methods `include_cpu()`, `include_memory()`

2. **`sniff/lib/src/hardware/mod.rs`** — `detect_hardware_with_request()`
   - Gate `System::new_with_specifics(...)` behind `request.include_cpu || request.include_memory`
   - When neither is needed, use `CpuInfo::default()` and `MemoryInfo::default()`
   - When only one is needed, still init `System` but skip the unused computation

**Testing strategy:**
- Update existing test `test_detect_hardware_summary_skips_expensive` to verify summary still includes CPU + memory
- Add test: `HardwareRequest::summary().include_gpu(true).include_cpu(false).include_memory(false)` skips `System` init (verify `CpuInfo` is default)
- Existing serialization roundtrip test covers new fields

### Suggestion #7: Skip shared manifest walk for structure-only repo requests

**Files to change:**

1. **`sniff/lib/src/filesystem/mod.rs`** — `detect_filesystem_with_request()` (lines 78–93)
   - Change the `need_shared_view` computation to exclude `request.repo.is_some()` when the repo request is structure-only
   - Current code: `need_shared_view = need_repo_full || ... || request.repo.is_some()`
   - New code: remove the `|| request.repo.is_some()` term — it's already covered by `need_repo_full` for non-structure requests
   - Also adjust `SharedWalkOptions.collect_manifests` to check `!repo.structure_only` when setting manifest collection

**Testing strategy:**
- Existing test `need_shared_view_true_for_repo_detection` uses `RepoRequest::full()` — still passes
- Add test: `RepoRequest::structure()` does NOT trigger `need_shared_view` when no other shared features are requested
- Verify `filesystem_summary_request` benchmark fixture returns identical results

**Phase 2 acceptance criteria:**
- `cargo test` passes in `sniff/lib`
- `GitRequest::minimal()` produces `GitInfo` with no branches/remotes/config
- `HardwareRequest` with `include_cpu: false, include_memory: false` skips `System` init
- Structure-only repo requests skip the shared manifest walk
- All existing API callers unaffected (no breaking changes)

---

## Phase 3: Git & Diff Optimizations (Medium Risk)

**Goal:** Reduce allocation overhead and skip unnecessary work in git status detection.

### Suggestion #6: Pre-size diff HashMaps

**Files to change:**

1. **`sniff/lib/src/filesystem/git/status.rs`** — `get_repo_status_with_changes()` (lines 30–57)
   - After `repo.statuses()`, compute `let estimated_dirty = statuses.iter().count();`
   - Pre-size: `let mut staged_patches = HashMap::with_capacity(estimated_dirty);`
   - Pre-size: `let mut unstaged_patches = HashMap::with_capacity(estimated_dirty);`
   - Also pre-size `diff_stats`: `let mut diff_stats = HashMap::with_capacity(estimated_dirty);`

**Testing strategy:**
- Existing tests in `status.rs` (`batched_diff_*`) already exercise the function with pre-sized maps
- No functional change — just allocation optimization

### Suggestion #10: Early-exit diff building for clean repos

**Files to change:**

1. **`sniff/lib/src/filesystem/git/status.rs`** — `get_repo_status_with_changes()` (lines 30–53)
   - After `repo.statuses()`, check `if statuses.is_empty()` (or `statuses.iter().all(|e| e.status().is_ignored())`)
   - When truly clean, return early with a clean `RepoStatus` and empty `Vec<FileChange>`
   - This skips `head_tree` resolution, `diff_tree_to_index`, `diff_index_to_workdir`

**Testing strategy:**
- Add unit test: clean repo (no dirty files) triggers early exit and returns `is_dirty: false`
- Existing tests use repos with changes, so they still exercise the full path
- Verify no behavior change for dirty repos

### Suggestion #12: Cache GitConfig in GitRepo with OnceLock

**Files to change:**

1. **`sniff/lib/src/filesystem/git/types.rs`** — `GitRepo` struct
   - Add field: `config_cache: OnceLock<GitConfig>`
   - Update `GitRepo::open()` and `GitRepo::discover()` to initialize with `OnceLock::new()`

2. **`sniff/lib/src/filesystem/git/types.rs`** — `config()` method (line 604)
   - Change from `super::remote_refresh::get_git_config(&self.repo)` to `self.config_cache.get_or_init(|| super::remote_refresh::get_git_config(&self.repo)).clone()`

3. **`sniff/lib/src/filesystem/git/types.rs`** — `detect_with_request()` (line 688)
   - Replace `let config = super::remote_refresh::get_git_config(&self.repo);` with `let config = self.config();`

**Testing strategy:**
- Existing tests that exercise `detect_full()` or `detect_with_request()` implicitly test config
- Add test: calling `config()` twice returns the same object (no re-read)
- Verify `OnceLock` doesn't cause issues with the `RefCell<HashMap>` already on `GitRepo`

**Phase 3 acceptance criteria:**
- `cargo test` passes in `sniff/lib`
- Clean repos skip diff building entirely
- Diff HashMaps are pre-sized based on status count
- GitConfig is read at most once per `GitRepo` instance
- No API changes

---

## Phase 4: Repo & Manifest Parallelism (Medium-High Risk)

**Goal:** Replace single-threaded walks with parallel alternatives and eliminate redundant walks.

### Suggestion #8: Parallelize ManifestIndex build + reduce syscalls

This is the highest-impact optimization (~50-75% on `repo_full_huge`).

**Files to change:**

1. **`sniff/lib/src/filesystem/repo/manifest_index.rs`** — `ManifestIndex::build()` (lines 97–149)
   - Replace `walkdir::WalkDir` with `ignore::WalkBuilder::build_parallel()` (same pattern as `system_view.rs`)
   - Use `Arc<Mutex<HashMap>>` for the grouped map, with per-thread local buffers flushed on `Drop` (same `WorkerBuffers` pattern as `system_view.rs`)
   - Apply the same directory filter (skip `.git`, `node_modules`, `target`, etc.)

2. **`sniff/lib/src/filesystem/repo/manifest_index.rs`** — `from_grouped()` (lines 180–193)
   - Replace `canonicalize_path(&original)` with `normalize_path(&original)` for deduplication
   - Lexical normalization is sufficient unless symlinks are present in manifest directories (rare)
   - This eliminates N `canonicalize` syscalls (one per package)

3. **`sniff/lib/src/filesystem/repo/detection.rs`** — `refresh_package_boundaries()` (lines ~820–833)
   - The inventory paths are relative to `repo_root`. Instead of `repo_root.join(&classification.path)` followed by `normalize_path()`, work directly with relative paths
   - Pre-build a `HashMap<&Path, usize>` mapping normalized package relative paths to indices
   - For each classification, walk parent directories of `classification.path` (relative) against the pre-built map
   - This eliminates `repo_root.join()` + `normalize_path()` + repeated `normalize_path(parent)` per file

**Testing strategy:**
- Existing `ManifestIndex` tests (`manifest_index_*`) exercise `build()` and `from_manifest_paths()`
- Integration test: `detect_repo()` on the current repo returns same results before and after
- Test on a monorepo fixture to verify package discovery parity
- Cross-platform note: `ignore::WalkBuilder` is already used cross-platform in `system_view.rs` and `classify.rs`

### Suggestion #9: Eliminate double directory walk in detect_repo_with_inventory

**Files to change:**

1. **`sniff/lib/src/filesystem/repo/types.rs`** — `detect_repo_with_inventory()` (lines 306–310)
   - Change from calling `detect_repo_inner(root, false)` (which builds its own ManifestIndex) to:
     - Build a `FilesystemSystemView` via `system_view::build_filesystem_system_view()` with `collect_manifests: true, collect_inventory: true`
     - Pass the shared manifest index and inventory into `detect_repo_inner_with_shared()`

2. **`sniff/lib/src/filesystem/repo/detection.rs`** — `detect_repo_inner()` (lines 39–44)
   - Keep as-is (delegates to `detect_repo_inner_with_shared(None, None)`) for backward compat
   - The new path in `types.rs` calls `detect_repo_inner_with_shared` directly

**Testing strategy:**
- Existing `detect_repo_with_inventory()` callers get identical results
- Add test: both paths (with and without shared view) produce the same `(RepoInfo, FileInventory)` for a test fixture
- Verify the double-walk is eliminated via tracing/performance metrics

**Phase 4 acceptance criteria:**
- `cargo test` passes in `sniff/lib`
- `ManifestIndex::build()` uses parallel `ignore::WalkBuilder`
- `from_grouped()` uses lexical normalization instead of `canonicalize`
- `refresh_package_boundaries` works with relative paths
- `detect_repo_with_inventory()` uses a single shared walk
- All existing API callers unaffected

---

## Phase 5: Nice-to-Have Polish (Low Risk)

**Goal:** Minor improvements that round out the optimization work.

### Suggestion #2: Eliminate redundant repo detection in blast-radius mode

**Files to change:**

1. **`sniff/lib/src/filesystem/docs.rs`** — `detect_blast_radius_docs()` (lines 173–207)
   - Remove the `let packages = collect_repo_packages(&repo_root);` call
   - Pass `&[]` as the packages parameter to `parse_markdown_meta_with_mode()`
   - Blast-radius mode (`DocParseMode::BlastRadiusOnly`) doesn't use the package field for filtering, so the empty vec is correct

**Testing strategy:**
- Existing blast-radius tests verify `has_blast_radius` detection works
- Add test: `detect_blast_radius_docs()` returns same results with/without package info

### Suggestion #13: Defer sorting in language aggregation

**Files to change:**

1. **`sniff/lib/src/filesystem/file_types/aggregate.rs`** — `build_language_summary()` (lines 96–183)
   - Remove the `entry.direct_files.sort()` and `entry.framework_files.sort()` calls (lines 109–110)
   - Add a `LanguageSummary::sorted()` method that sorts all file lists
   - Update callers that need sorted output to call `.sorted()`

2. **Callers that consume language summary** — audit and add `.sorted()` where deterministic order is needed (e.g., JSON serialization)

**Testing strategy:**
- Existing language detection tests verify content, not order
- Add test: `LanguageSummary::sorted()` produces deterministically ordered output
- Verify JSON serialization still produces consistent output

### Suggestion #15: Add `sorted()` method to LanguageSummary

This is part of Suggestion #13 above — implement together.

### Suggestion #14: Parallelize markdown frontmatter parsing with Rayon

**Files to change:**

1. **`sniff/lib/src/filesystem/docs.rs`** — `collect_markdown_files()` and `detect_blast_radius_docs()`
   - After collecting markdown paths from the walker, use `rayon::prelude::*` to parse in parallel:
     ```rust
     let paths: Vec<_> = walker.filter_map(|e| e.ok()).filter(|e| /* md check */).collect();
     let docs: Vec<MarkdownMeta> = paths.into_par_iter()
         .filter_map(|entry| parse_markdown_meta_with_mode(...))
         .collect();
     ```
   - Gate behind `#[cfg(feature = "rayon")]` or add unconditionally since `rayon` is already a dependency

**Testing strategy:**
- Existing doc detection tests verify content correctness regardless of parallelism
- Verify no thread-safety issues with `serde_yaml_ng` (profile before committing)
- Cross-platform note: Rayon already used elsewhere in sniff, so no new dependency

**Phase 5 acceptance criteria:**
- `cargo test` passes in `sniff/lib`
- `detect_blast_radius_docs()` no longer calls `detect_repo()`
- Language summary sorting is deferred until `.sorted()` is called
- Markdown parsing can optionally be parallelized with Rayon

---

## Cross-Phase Testing Strategy

After each phase:

1. **Unit tests:** `cargo test -p sniff-lib`
2. **Benchmarks:** `cargo bench --bench perf` to verify improvements
3. **No regressions:** Compare benchmark outputs against the previous run
4. **Cross-platform:** All changes use cross-platform primitives (`ignore::WalkBuilder`, `OnceLock`, `rayon`, `std::fs`)
5. **No API breaks:** Each phase preserves the public API; new fields/methods are additive only

## Dependency Graph

```
Phase 1 (caching)        ← no dependencies
Phase 2 (short circuits)  ← no dependencies
Phase 3 (git/diff)        ← Phase 2 (GitRequest::minimal may use early-exit patterns)
Phase 4 (repo/manifest)   ← Phase 2 (structure-only skip shared walk)
Phase 5 (nice-to-have)    ← Phase 3 (deferred sorting benefits from git optimizations)
```

Phases 1 and 2 can be implemented in either order. Phase 3 and 4 can be implemented in parallel by separate developers since they touch different files (git/ vs repo/).

## Expected Cumulative Impact

| Benchmark | Current | After All Phases | Primary Fix |
|-----------|---------|------------------|-------------|
| `programs_info_detect` | ~24.7ms | ~3ms | Phase 1: OnceLock cache |
| `programs_detect` | ~25.0ms | ~3ms | Phase 1: OnceLock cache |
| `filesystem_summary_request` | ~16.5ms | ~7ms | Phase 2: Skip shared walk |
| `git_summary_small` | ~10.4ms | ~8ms | Phase 2: GitRequest::minimal |
| `detect_gpus_only` | varies | -5-10% | Phase 2: Defer System init |
| `git_deep/100` | ~18.7ms | ~14ms | Phase 3: Pre-size HashMaps |
| `git_full_monorepo` | ~16.5ms | ~14ms | Phase 3: Clean-repo early exit |
| `repo_full_huge` | ~56.5ms | ~18ms | Phase 4: Parallel manifest walk |
| `repo_with_inventory_monorepo` | ~15.8ms | ~8ms | Phase 4: Shared walk |
| `filesystem_full_request` | ~29.0ms | ~18ms | Phases 2-4 combined |
