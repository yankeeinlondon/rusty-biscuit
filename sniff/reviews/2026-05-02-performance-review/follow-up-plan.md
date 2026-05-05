---
agent: ""
created: "2026-05-04T00:00:00"
parent_plan: "sniff/reviews/2026-05-02-performance-review/plan.md"
remaining_items: 0
status: completed
phases: 5
start_phase: 1
source_files_during_phase_1:
  - sniff/lib/src/filesystem/file_types/classify.rs
  - sniff/lib/tests/integration.rs
source_files_during_phase_2:
  - sniff/lib/src/filesystem/git/detection.rs
source_files_during_phase_3:
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/programs/find_program.rs
  - sniff/lib/src/programs/types.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/docs.rs
source_files_during_phase_4:
  - sniff/lib/benches/perf.rs
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/cases/git.rs
  - sniff/lib/benches/cases/programs.rs
  - sniff/lib/benches/cases/repo.rs
  - sniff/lib/benches/support/builder.rs
  - sniff/lib/benches/support/fixtures.rs
docs_updated_during_phase_4:
  - sniff/reviews/2026-05-02-performance-review/baselines.md
  - sniff/reviews/2026-05-02-performance-review/plan.md
  - sniff/reviews/2026-05-02-performance-review/follow-up-plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - sniff/cli/src/commands.rs
  - sniff/cli/src/main.rs
  - sniff/lib/Cargo.toml
  - sniff/cli/Cargo.toml
packages:
  - sniff/lib
  - sniff/cli
---

# Performance Review — Follow-Up Plan

**Original Review:** `sniff/reviews/2026-05-02-performance-review/review.md`  
**Original Plan:** `sniff/reviews/2026-05-02-performance-review/plan.md`  
**Findings:** 9 total (2 High, 5 Medium, 2 Low)  
**Date:** 2026-05-04

---

## Verified Completed Work

The following phases were implemented between 2026-05-02 and 2026-05-02. All code is on `main` and all associated tests pass.

### Phase 1: Critical Production Fix — Parallel Inventory Restoration ✅

| Commit | Description |
|--------|-------------|
| `2b462c87` | `fix(sniff): flush worker-local buffers in parallel inventory scanner` |

**Deliverables confirmed:**
- [x] `LocalClassifications` drop-flush wrapper in `scan_inventory_parallel`
- [x] Integration regression test (`parallel_inventory_collects_all_classifications`)
- [x] Inventory/filesystem benchmarks compile and run

**Validation:**
```bash
cargo test -p sniff --lib file_types         # 10 passed
cargo test -p sniff --test integration -- parallel_inventory_collects_all_classifications  # passes
cargo bench -p sniff --bench perf -- inventory  # compiles and runs
```

### Phase 2: High-Impact Git Performance — Batched Diff Aggregation ✅

| Commit | Description |
|--------|-------------|
| `6721fcba` | `refactor(sniff): optimize repo status diff aggregation` |

**Deliverables confirmed:**
- [x] Single-call staged (`diff_tree_to_index`) and unstaged (`diff_index_to_workdir`) diff construction
- [x] Path-keyed `HashMap<PathBuf, LineStats>` accumulator
- [x] Deep-mode patch emission from batched diffs via `aggregate_diff`
- [x] Focused tests for rename/delete and staged-plus-unstaged combinations (6 tests)

**Validation:**
```bash
cargo test -p sniff --lib -- batched_diff    # 6 passed
cargo test -p sniff --lib git                # 57 passed
cargo bench -p sniff --bench perf -- git     # compiles and runs
```

### Phase 3: Medium-Impact Optimizations ✅

| Commit | Description |
|--------|-------------|
| `4e76ebb9` | `refactor(sniff): eliminate redundant manifest parsing and canonicalize syscalls` |
| `19b89048` | `perf(sniff): optimize bulk program detection with eager PATH indexing` |
| `386beb2e` | `refactor(sniff): add blast-radius-only doc parsing mode` |

**Deliverables confirmed:**
- [x] `ManifestEntry` with pre-normalized paths; `package_dirs_in_tree` updated
- [x] `PackageBuildContext` with `ManifestCache` wired through discovery
- [x] `ExecutableIndex::build_eager_path()` and deduplicated `names_to_search`
- [x] `DocParseMode::BlastRadiusOnly` and blast-radius scanner integration
- [x] Tests for nested-package exclusion (`refresh_boundaries_excludes_nested_package_files_from_parent`)

**Validation:**
```bash
cargo test -p sniff --lib repo      # 28 passed
cargo test -p sniff --lib programs  # 263 passed
cargo test -p sniff --lib filesystem # 251 passed
```

### Phase 4: Benchmarking Infrastructure — Complete

| Commit | Description |
|--------|-------------|
| `dce96ce2` | `feat(sniff): add criterion benches for git dirty-file scaling and docs parsing modes` |
| `4b8faaff` | `docs(sniff): add Phase 4 baseline measurement commands and update plan tracker` |
| `06aa8afc` | `perf(sniff): add criterion benches for repo package boundary scaling` |
| `f5e67147` | `docs(sniff): add performance review baselines and phase_2 tracking` |

**Deliverables confirmed:**
- [x] Criterion benchmark for git dirty-file scaling
- [x] Criterion benchmark for docs parsing modes
- [x] Criterion benchmark for program detection
- [x] Criterion benchmark for repo package-boundary refresh
- [x] Profiling commands documented in `sniff/lib/README.md`
- [x] Baseline measurements recorded for Phase 5 comparison

### Phase 5.1: Deep Git Remote Concurrency ✅

| Commit | Description |
|--------|-------------|
| `91ce6e72` | `perf(sniff): parallelize remote fetch in git detection` |

**Deliverables confirmed:**
- [x] Bounded parallel remote fetch with `GIT_TERMINAL_PROMPT=0` preservation
- [x] Ancestry-walk containment optimization

### Phase 5.2: CLI Async Boundaries ✅

| Commit | Description |
|--------|-------------|
| `807d279b` | `refactor(sniff): CLI core commands and main entry point refactor` |

**Deliverables confirmed:**
- [x] CLI concurrency model documented in `sniff/cli/src/commands.rs` (lines 86–92)

### Phase 5.3: Dependency Feature Cleanup — Partially Complete

**Deliverables confirmed:**
- [x] `which` dependency already has `default-features = false, features = ["real-sys"]` (removes `regex` and `tracing` features)

**Deliverables NOT completed:**
- [ ] Measurement-backed decision documented (`cargo bloat` / `cargo llvm-lines` data)
- [ ] Decision on CLI feature split (e.g., `sniff-cli/remote`) recorded

---

## Remaining Items

All remaining items have been completed. See below for completion status.

### Item 1: Repo Package-Boundary Benchmark — ✅ Complete

**Resolution:** `cases/repo.rs` benchmark module added in commit `06aa8afc`. It creates temporary repos with configurable Cargo package counts (10, 100, 500), measures `refresh_package_boundaries` in isolation, and is registered with `perf.rs`.

**Validation:**
```bash
cargo bench -p sniff --bench perf -- repo  # passes
```

---

### Item 2: Record Baseline Measurements — ✅ Complete

**Resolution:** Baseline measurements recorded in `sniff/reviews/2026-05-02-performance-review/baselines.md` (commit `f5e67147`). Includes compile-time (`cargo bloat`, `cargo llvm-lines`), runtime (`hyperfine`, Criterion), and dependency feature decisions.

---

### Item 3: Document Dependency Feature Decisions — ✅ Complete

**Resolution:** Decision documented in `baselines.md`:
- `which` crate keeps trimmed features (`default-features = false, features = ["real-sys"]`)
- CLI feature split deferred until `cargo bloat` identifies a feature contributing > 5% of binary size

---

### Item 4: Close Out Review Artifacts — ✅ Complete

**Resolution:**
- `plan.md` checkboxes updated to reflect completion
- `follow-up-plan.md` frontmatter updated with completion status
- `plan-for-review.md` retained as a working draft reference
- `review-1.md` retained as the completed review output

---

## Recommended Execution Order

1. **Item 1** → Repo package-boundary benchmark (adds missing Phase 4 deliverable)
2. **Item 2** → Run baselines while benchmark is fresh (saves time by running everything together)
3. **Item 3** → Document dependency decisions using baseline data
4. **Item 4** → Close out review artifacts (depends on items 1–3 for accurate status)

---

## Simplified Done Criteria

- [x] `cargo bench -p sniff --bench perf -- repo` runs without error
- [x] `sniff/reviews/2026-05-02-performance-review/baselines.md` exists with:
  - Compile-time measurements (`cargo bloat`, `cargo llvm-lines`)
  - Runtime measurements (Criterion summaries, `hyperfine` results)
  - Dependency feature decision rationale
- [x] `plan.md` checkboxes accurately reflect completed vs. remaining work
- [x] `cargo test -p sniff --lib` and `cargo test -p sniff-cli` pass
- [x] `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` passes

---

## Risk: None

All code changes are already landed and tested. This follow-up plan only adds benchmarks, measurements, and documentation. No production behavior changes.
