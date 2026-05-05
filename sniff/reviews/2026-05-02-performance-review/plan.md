---
agent: ""
created: "2026-05-02T14:28:07"
phases: 5
start_phase: 1
scope:
  - sniff/lib
  - sniff/cli
source_files_during_phase_1:
  - sniff/lib/src/filesystem/file_types/classify.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/benches/cases/filesystem.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - sniff/lib
source_files_during_phase_2:
  - sniff/lib/src/filesystem/git/detection.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/programs/find_program.rs
  - sniff/lib/src/programs/types.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/docs.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - sniff/lib/benches/perf.rs
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/cases/git.rs
  - sniff/lib/benches/cases/programs.rs
  - sniff/lib/benches/support/builder.rs
  - sniff/lib/benches/support/fixtures.rs
docs_updated_during_phase_4:
  - sniff/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - sniff/cli/src/commands.rs
  - sniff/cli/src/main.rs
  - sniff/lib/Cargo.toml
  - sniff/cli/Cargo.toml
---

# Performance Review Implementation Plan

**Review Date:** 2026-05-02
**Source:** `sniff/reviews/2026-05-02-performance-review/review.md`
**Total Findings:** 9 (2 High, 5 Medium, 2 Low)

---

## Phase 1: Critical Production Fix — Parallel Inventory Restoration

**Goal:** Fix the production bug where parallel file inventory drops all classifications, restoring correctness and benchmark validity.

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| Production parallel inventory drops all classifications | High | `classify.rs`, `scan_inventory_parallel` | Adopt the `system_view.rs` drop-flush pattern: create a `LocalClassifications` struct that appends worker-local results to the shared `Arc<Mutex<Vec<FileClassification>>>` on drop. Replace the unused `_shared` clone with the drop-flush wrapper. |

**Deliverables:**
- [x] `LocalClassifications` drop-flush wrapper in `scan_inventory_parallel`
- [x] Non-test integration test or unit-test hook that exercises the parallel path
- [x] Rerun inventory/filesystem benchmarks to validate measurements

**Validation:**
```bash
cargo test -p sniff --lib file_types
cargo test -p sniff --test integration -- file_inventory
cargo bench -p sniff --bench perf -- inventory
```

**Acceptance:**
- Production builds retain classified file inventory results.
- Benchmarks measure real work instead of discarded output.
- No observable behavior change beyond restored correctness.

---

## Phase 2: High-Impact Git Performance — Batched Diff Aggregation

**Goal:** Replace per-file git diff loops with batched whole-repo diffs to eliminate O(dirty_files * diff_setup_cost) scaling.

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| Git file-change stats scale with dirty files times full diff work | High | `git/detection.rs`, `get_repo_status_with_changes` | Build staged (`diff_tree_to_index`) and unstaged (`diff_index_to_workdir`) diffs once. Iterate each diff once with `foreach`, accumulating `LineStats` by path into a `HashMap<PathBuf, LineStats>`. Reuse the same diffs for deep-mode patch generation grouped by path. |

**Deliverables:**
- [x] Single-call staged and unstaged diff construction
- [x] Path-keyed `HashMap` accumulator for per-file stats
- [x] Deep-mode patch emission from the same batched diffs (or explicit gating)
- [x] Focused tests for rename/delete handling and staged-plus-unstaged combinations

**Validation:**
```bash
cargo test -p sniff --lib git
cargo test -p sniff --test integration -- git_status
cargo bench -p sniff --bench perf -- filesystem_git
```

**Acceptance:**
- Large dirty repos show lower git-status latency.
- Per-file change stats remain identical for representative cases.
- Rename/delete and combined staged/unstaged behavior is tested.

---

## Phase 3: Medium-Impact Index, Parsing, and Lookup Optimization

**Goal:** Remove redundant canonicalization, repeated manifest parsing, and unnecessary PATH scans.

### 3.1 ManifestIndex Path Normalization

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| ManifestIndex still performs repeated canonicalization syscalls | Medium | `repo/types.rs`, `ManifestIndex::package_dirs_in_tree` | Store normalized paths at index-build time in a `ManifestEntry { original, normalized, kinds }`. Replace per-query `canonicalize` with pre-normalized comparisons. Keep canonicalization as an explicit index-build option if symlink identity matters. |

### 3.2 Package Manifest Parsing Cache

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| Package creation reparses manifests multiple times | Medium | `repo/detection.rs`, `create_package` | Introduce `PackageBuildContext { manifests: ManifestCache, lock_versions: Option<CargoLockVersions> }`. Parse each manifest once and derive name, version, features, dependencies, and ecosystem from the parsed representation. Keep the public `Package` API unchanged. |

### 3.3 Program Lookup Deduplication and PATH Index

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| Program detection lost the intended shared PATH index | Medium | `programs/find_program.rs`, `programs/types.rs` | Restore eager PATH indexing for bulk detection: `ExecutableIndex::build_eager_path()` scans `PATH` once into `HashMap<OsString, PathBuf>`. Use it in `ProgramsInfo::detect`. Deduplicate `names_to_search` before lookup. Keep lazy `which` behavior for single lookups. |

### 3.4 Blast-Radius-Only Document Parsing

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| Blast-radius reparses every markdown document after collecting changes | Medium | `blast_radius.rs`, `docs.rs` | Add `DocParseMode::BlastRadiusOnly` to `parse_markdown_meta`. Read only until closing frontmatter delimiter; parse only the `blast_radius` key. Skip content hashing, mtime, and full-body processing unless full `MarkdownMeta` is requested. |

**Deliverables:**
- [x] `ManifestEntry` with pre-normalized paths; updated `package_dirs_in_tree`
- [x] `PackageBuildContext` with `ManifestCache` wired through discovery
- [x] Eager `ExecutableIndex::build_eager_path()` and deduplicated `names_to_search`
- [x] `DocParseMode::BlastRadiusOnly` and blast-radius scanner integration
- [x] Tests for mixed Cargo/Node/Python/Go packages and nested-package exclusion

**Validation:**
```bash
cargo test -p sniff --lib repo
cargo test -p sniff --lib programs
cargo test -p sniff --lib filesystem
cargo test -p sniff --test integration -- blast_radius
```

**Acceptance:**
- Index queries perform no syscalls during lookup.
- Each manifest file is read and parsed at most once per detection.
- Bulk program detection traverses `PATH` once.
- Blast-radius queries skip full markdown parsing for non-matching docs.

---

## Phase 4: Benchmarking and Validation Infrastructure

**Goal:** Add targeted benchmarks and profiling to measure dirty-file scaling, package count, docs count, and PATH length before and after changes.

| Activity | Target | Approach |
|----------|--------|----------|
| Git dirty-file scaling | `git/detection.rs` | Criterion cases for 10, 100, and 1,000 dirty files with `GitRequest::full()` and `GitRequest::deep()` |
| Package-boundary scaling | `repo/detection.rs` | Full repo fixture with hundreds of manifests and thousands of classified files to isolate `refresh_package_boundaries` |
| Docs parsing comparison | `docs.rs` | Fixture with many small markdown files and a few large files; compare `detect_docs` vs blast-radius-only parser |
| PATH-length program detection | `programs/` | `hyperfine` runs with long `PATH` values |
| Compile-time/dependency baselines | `Cargo.toml` files | `cargo bloat` and `cargo llvm-lines` before feature cleanup |

**Deliverables:**
- [x] Criterion benchmark for git dirty-file scaling
- [x] Criterion benchmark for repo package-boundary refresh
- [x] Criterion benchmark for docs parsing modes
- [x] Profiling commands documented for `cargo flamegraph` and `hyperfine`
- [x] Baseline measurements recorded for Phase 5 comparison

**Validation:**
```bash
cargo bench -p sniff --bench perf -- filesystem_git filesystem_repo filesystem_staged inventory
cargo flamegraph -p sniff-cli --bin sniff -- repo git-status
hyperfine 'target/release/sniff programs' 'target/release/sniff editors'
cargo bloat -p sniff-cli --release --bin sniff
cargo llvm-lines -p sniff-cli --release
```

**Acceptance:**
- Benchmarks run without error on representative fixtures.
- Measurements capture before/after deltas for Phase 1–3 changes.
- Benchmark fixtures are reproducible and documented.

---

## Phase 5: Cleanup and Architecture Hardening

**Goal:** Address remaining low-severity findings and optional compile-time improvements with measurement-driven decisions.

### 5.1 Deep Git Remote Concurrency

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| Deep git remote refresh and containment are serial and multiplicative | Low | `git/detection.rs` | Keep current behavior for defaults. Cap or batch deep containment: fetch remotes with small bounded parallelism if multiple remotes are configured; walk ancestry once from each remote tip up to the oldest requested commit and build `HashMap<Oid, Vec<remote>>`; add a request knob for max remote branches inspected. |

### 5.2 CLI Async Boundaries

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| CLI async entrypoint mostly runs synchronous blocking work | Low | `cli/src/commands.rs`, `cli/src/main.rs` | Document the single-shot command model explicitly, or move heavyweight local detection behind `tokio::task::spawn_blocking` only for command paths that concurrently await network operations. Avoid broad `spawn_blocking` without real concurrent async work. |

### 5.3 Dependency Feature Cleanup (Measurement-Driven)

| Finding | Severity | Location | Approach |
|---------|----------|----------|----------|
| Narrow compile-time cleanup: unused or broad dependency features | Low | `lib/Cargo.toml`, `cli/Cargo.toml` | Measure with `cargo bloat` and `cargo llvm-lines`. If confirmed, remove unused `which` features (`regex`, `tracing`) and consider a CLI feature split such as `sniff-cli/remote` for commands that need remote providers. Do not split without build-size data. |

**Deliverables:**
- [x] Bounded parallel remote fetch with `GIT_TERMINAL_PROMPT=0` preservation
- [x] Ancestry-walk containment optimization with `HashMap<Oid, Vec<remote>>`
- [x] Documented CLI concurrency model or targeted `spawn_blocking`
- [x] Measurement-backed dependency feature decisions

**Validation:**
```bash
cargo test -p sniff --lib git -- deep_remote
cargo test -p sniff-cli --test integration
cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings
cargo bloat -p sniff-cli --release --bin sniff
cargo llvm-lines -p sniff-cli --release
```

**Acceptance:**
- Deep git defaults unchanged; worst-case latency improved.
- CLI async model is documented and internally consistent.
- Dependency changes are justified by measurement, not speculation.

---

## Cross-Cutting Concerns

### Testing Strategy
- Add or update tests before each optimization to lock current behavior.
- Run `cargo test -p sniff --lib` and `cargo test -p sniff-cli` after every phase.
- Use the new benchmarks from Phase 4 to validate Phase 1–3 impact.

### Risk Mitigation
- **Parallel inventory fix:** Ensure the drop-flush pattern does not introduce panics if a worker panics; consider `std::sync::Mutex` poison handling.
- **Batched git diffs:** Rename/delete and staged-plus-unstaged combinations need focused tests before merge.
- **ManifestIndex normalization:** If current behavior depends on symlink identity, retain canonicalized entries at build time.
- **Eager PATH index:** Memory use is proportional to PATH executable count; acceptable for bulk detection only.

### Dependencies
- Phase 1 must complete before Phase 4 benchmarks are meaningful (the production bug invalidates all inventory measurements).
- Phase 2 should land before Phase 4 git benchmarks so the bench measures the optimized path.
- Phase 3 changes are independent and can be landed in any order, but benefit from Phase 4 fixtures.

---

## Implementation Order

1. Phase 1: Fix `scan_inventory_parallel` and add regression test.
2. Phase 2: Batch git diff stats and patch generation.
3. Phase 3.3: Restore eager PATH index and deduplicate names.
4. Phase 3.1: Normalize `ManifestIndex` entries at build time.
5. Phase 3.2: Cache manifest parsing in `PackageBuildContext`.
6. Phase 3.4: Add blast-radius-only docs parse mode.
7. Phase 4: Add all benchmarks and record baselines.
8. Phase 5.1: Parallel remote fetches and ancestry-walk containment.
9. Phase 5.2: Document or harden CLI async boundaries.
10. Phase 5.3: Measure and, if justified, trim dependency features.

---

## Risk Register

| Risk | Area | Mitigation |
|------|------|------------|
| Drop-flush panics on worker panic | `classify.rs` | Handle mutex poison or use a lock-free accumulation strategy |
| Batched diff changes rename/delete semantics | `git/detection.rs` | Add focused tests for renames, deletes, and combined staged/unstaged |
| Normalized paths miss symlink aliases | `repo/types.rs` | Retain canonicalized entries at build time if symlink identity matters |
| Eager PATH index memory pressure | `programs/` | Scope to bulk detection; keep lazy `which` for single lookups |
| Manifest cache complicates discovery plumbing | `repo/detection.rs` | Keep public `Package` API unchanged; add fixture tests |
| Blast-radius-only parser diverges from full parser | `docs.rs` | Run both parsers on the same fixtures and compare `blast_radius` values |
| Parallel fetches increase credential-helper contention | `git/detection.rs` | Keep concurrency low; preserve `GIT_TERMINAL_PROMPT=0` |
| `spawn_blocking` adds error-handling complexity | `cli/` | Apply only where real concurrent async work exists |
| Feature splitting complicates distribution | `Cargo.toml` | Do not split without `cargo bloat` evidence |

---

## Done Criteria

- All nine findings in `review.md` are addressed in code or explicitly deferred with justification.
- Tests cover each changed behavior or preserve current public behavior.
- Benchmarks from Phase 4 run after the corresponding optimizations.
- No public API break is introduced except internal helper signatures.
- `cargo test -p sniff --lib` and `cargo test -p sniff-cli` pass.
- `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` passes.
