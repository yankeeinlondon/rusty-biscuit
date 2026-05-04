---
phases: 5
starting_phase: 5
---

# Performance Review Implementation Plan

Generated from: `2026-05-02-performance-review/review.md`

## Overview

This plan addresses **10 performance findings** (2 High, 5 Medium, 3 Low) from the sniff performance review. Work is grouped into 5 phases that minimize cross-phase dependencies while ensuring each phase is independently testable and measurable.

---

## Phase 1: Production Inventory Fix and Baseline Validation

**Goal:** Restore correctness for production inventory/language output and establish valid benchmark baselines.

### Tasks

1. **Fix `scan_inventory_parallel` worker-local flush** (Finding 1 — High)
   - Replace the lost `_shared` clone with a `Drop`-based flush pattern (matching `system_view.rs`)
   - Ensure each worker pushes `local` classifications into the shared `Arc<Mutex<Vec<_>>>` on drop
   - Keep the existing final sort

2. **Add production-path regression test**
   - Add a non-test integration test or unit-test hook that exercises the parallel inventory path
   - Assert that classifications are non-empty after parallel scanning on a fixture tree

3. **Add inventory benchmark cases**
   - Add Criterion cases for full inventory scans
   - Capture baseline numbers before subsequent optimizations

4. **Rerun benchmarks**
   - Run `cargo bench -p sniff --bench perf -- filesystem_staged inventory`
   - Validate that production inventory output is no longer empty

### Success Criteria
- `scan_inventory_parallel` returns populated classifications in release builds
- Regression test fails before fix and passes after fix
- Benchmark baselines are recorded

---

## Phase 2: Git Diff Batching

**Goal:** Eliminate per-file diff setup cost in dirty repos.

### Tasks

1. **Batch git diff stats** (Finding 2 — High)
   - Build staged diff once with `diff_tree_to_index`
   - Build unstaged diff once with `diff_index_to_workdir`
   - Accumulate `LineStats` per path by iterating each diff once via `foreach`
   - Replace per-file `get_file_diff_stats` calls with a single batched pass

2. **Batch patch generation for deep mode**
   - Emit per-file unified patches from the same whole-repo diffs
   - Group patches by path instead of reconstructing diffs per pathspec
   - Gate full patch payloads behind `GitRequest::deep()` explicitly

3. **Add git dirty-file benchmarks**
   - Add Criterion cases for 10, 100, and 1,000 dirty files with `GitRequest::full()` and `GitRequest::deep()`

4. **Focused tests for edge cases**
   - Rename/delete handling
   - Staged + unstaged combinations for the same file
   - Mixed binary/text deltas

### Success Criteria
- Git status latency scales sub-linearly with dirty-file count
- All existing git tests pass
- New dirty-file benchmarks show improvement vs. baseline

---

## Phase 3: Caching and Index Normalization

**Goal:** Remove repeated parsing, canonicalization, and PATH traversal.

### Tasks

1. **Cache manifest parsing** (Finding 4 — Medium)
   - Introduce `PackageBuildContext` with a `ManifestCache`
   - Parse each manifest once and derive name, version, features, dependencies, and ecosystem from the cached representation
   - Wire `PackageBuildContext` through `create_package` and manifest helpers
   - Remove or activate the existing dead `ManifestCache` code

2. **Normalize `ManifestIndex` entries at build time** (Finding 3 — Medium)
   - Add a `ManifestEntry` struct storing both `original` and `normalized` paths
   - Perform canonicalization/lexical normalization once during index construction
   - Update `package_dirs_in_tree` to compare normalized paths without syscalls

3. **Restore eager PATH index and deduplicate names** (Finding 6 — Medium)
   - Implement `ExecutableIndex::build_eager_path()` to scan `PATH` once into `HashMap<OsString, PathBuf>`
   - Use eager index in `ProgramsInfo::detect` for bulk detection
   - Deduplicate `names_to_search` before lookup in `CategoryDetector::new_with_index`
   - Keep lazy `which(program)` behavior for single lookups

4. **Add repo/package and PATH-length benchmarks**
   - Add a full repo fixture with hundreds of manifests and thousands of classified files
   - Add `hyperfine` or Criterion cases for `sniff programs` with long `PATH` values

### Success Criteria
- `create_package` no longer re-reads the same manifest file
- `ManifestIndex` queries perform zero canonicalization syscalls
- `sniff programs` latency is predictable and lower on long/slow PATHs
- Benchmarks show reduced I/O and allocation

---

## Phase 4: Algorithmic Optimizations

**Goal:** Improve asymptotic complexity for package enrichment and blast-radius scanning.

### Tasks

1. **Optimize package boundary enrichment** (Finding 5 — Medium)
   - Sort packages by relative path
   - Build a package-prefix index once from the sorted list
   - Assign each classification to the deepest containing package in a single pass
   - Accumulate language/file stats directly per package instead of cloning `FileInventory` per package
   - Use the same prefix structure (or a trie) for nested-package descendant detection, replacing the O(P²) pairwise scan

2. **Add blast-radius-only docs scanner** (Finding 7 — Medium)
   - Add `DocParseMode::BlastRadiusOnly` to `parse_markdown_meta`
   - Read only until the closing frontmatter delimiter (plus a small prefix for title if needed)
   - Parse only the `blast_radius` key
   - Skip content hashing and mtime unless full `MarkdownMeta` is requested
   - Update `find_blast_radius_documents` to use the lightweight mode

3. **Add docs and package-enrichment benchmarks**
   - Add a docs fixture with many small markdown files and a few large markdown files
   - Compare `detect_docs` vs blast-radius-only parser
   - Measure `refresh_package_boundaries` on the Phase 3 repo fixture

### Success Criteria
- Package enrichment is sub-linear in `packages × files`
- Blast-radius queries skip full markdown parsing for non-matching docs
- Benchmarks validate improvements on large repo fixtures

---

## Phase 5: Concurrency and Dependency Cleanup

**Goal:** Address low-severity async and compile-time issues.

### Tasks

1. **Parallelize deep git remote refresh** (Finding 8 — Low)
   - Fetch remotes with small bounded parallelism if multiple remotes are configured
   - For containment checks, walk ancestry once from each remote tip up to the oldest requested commit
   - Build `HashMap<Oid, Vec<remote>>` instead of pairwise `graph_descendant_of` loops
   - Add a request knob for max remote branches inspected
   - Preserve `GIT_TERMINAL_PROMPT=0`

2. **Evaluate CLI async boundaries** (Finding 9 — Low)
   - Document the current single-shot command model if no changes are made
   - OR: Move heavyweight local detection behind `tokio::task::spawn_blocking` only for command paths that concurrently await network operations
   - Avoid sprinkling `spawn_blocking` everywhere without real concurrent async work

3. **Measure and clean up dependency features** (Finding 10 — Low)
   - Run `cargo bloat -p sniff-cli --release --bin sniff`
   - Run `cargo llvm-lines -p sniff-cli --release`
   - If confirmed, remove unused `which` features (`regex`, `tracing`)
   - Consider a `sniff-cli/remote` feature split for commands that need remote providers
   - Do not split remote support unless build-size data justifies it

4. **Add PATH-length and deep-git benchmarks**
   - Capture measurements for Phase 5 changes

### Success Criteria
- Deep git containment latency improves for repos with many remotes/branches
- CLI async model is documented or safely refactored
- Build-size data informs any Cargo.toml changes

---

## Dependencies Between Phases

| Dependency | From | To | Reason |
|------------|------|-----|--------|
| Benchmark validity | Phase 1 | Phase 2 | Phase 1 restores correct production inventory output, making subsequent benchmark comparisons meaningful |
| Structural improvements | Phase 3 | Phase 4 | Phase 4 package-enrichment optimization builds on the package structures and manifest caching from Phase 3 |
| Measurement baselines | Phase 1–4 | Phase 5 | Compile-time/dependency cleanup should be guided by measurement data collected in earlier phases |

**Independent phases:** Phase 2 (git batching) does not depend on Phase 1 code changes and could begin in parallel once the benchmark framework is established. Phase 3 (caching) is also largely independent of Phase 2.

---

## Risk Summary

| Phase | Primary Risk | Mitigation |
|-------|-------------|------------|
| Phase 1 | Mutex contention at drop time | One acquisition per worker, not per file; pattern already proven in `system_view.rs` |
| Phase 2 | Rename/delete and staged+unstaged combinations | Focused edge-case tests before merging |
| Phase 3 | Public API breakage | Keep `Package` output unchanged; refactor internals only |
| Phase 4 | Nested-package exclusion semantics drift | Tests for root packages, nested packages, and package areas |
| Phase 5 | Parallel fetches increase network/credential load | Keep concurrency low; preserve `GIT_TERMINAL_PROMPT=0` |
