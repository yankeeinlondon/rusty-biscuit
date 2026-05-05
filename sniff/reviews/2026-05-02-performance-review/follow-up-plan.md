---
agent: ""
created: "2026-05-04T00:00:00"
parent_plan: "sniff/reviews/2026-05-02-performance-review/plan.md"
remaining_items: 4
status: follow-up
phases: 5
start_phase: 1
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

### Phase 4 (Partial): Benchmarking Infrastructure — Partially Complete

| Commit | Description |
|--------|-------------|
| `dce96ce2` | `feat(sniff): add criterion benches for git dirty-file scaling and docs parsing modes` |
| `4b8faaff` | `docs(sniff): add Phase 4 baseline measurement commands and update plan tracker` |

**Deliverables confirmed:**
- [x] Criterion benchmark for git dirty-file scaling (`cases/git.rs`)
- [x] Criterion benchmark for docs parsing modes (`cases/filesystem.rs`)
- [x] Criterion benchmark for program detection (`cases/programs.rs`)
- [x] Profiling commands documented in `sniff/lib/README.md`

**Deliverables NOT completed:**
- [ ] Criterion benchmark for repo package-boundary refresh
- [ ] Baseline measurements recorded for Phase 5 comparison

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

### Item 1: Repo Package-Boundary Benchmark

**Gap:** Phase 4 called for a "Criterion benchmark for repo package-boundary refresh" with "full repo fixture with hundreds of manifests and thousands of classified files to isolate `refresh_package_boundaries`." No `cases/repo.rs` benchmark file exists.

**Action:** Add a `cases/repo.rs` benchmark module that:
- Creates a temporary repo with a configurable number of Cargo packages (10, 100, 500)
- Each package has a `Cargo.toml`, `src/lib.rs`, and a few classified files
- Measures `refresh_package_boundaries` in isolation
- Registers with `perf.rs`

**Validation:**
```bash
cargo bench -p sniff --bench perf -- repo
```

**Effort:** Small (can reuse existing fixture builders from `benches/support/builder.rs`)

---

### Item 2: Record Baseline Measurements

**Gap:** Phase 4 README documents the profiling commands but no actual numbers were captured for before/after comparison.

**Action:** Run the documented profiling commands on current `main` and record results in:
- `sniff/reviews/2026-05-02-performance-review/baselines.md`

**Commands to run:**
```bash
# Compile-time baselines
cargo bloat -p sniff-cli --release --bin sniff
cargo llvm-lines -p sniff-cli --release

# Runtime baselines (on a representative repo)
cargo flamegraph -p sniff-cli --bin sniff -- repo git-status
hyperfine 'target/release/sniff programs' 'target/release/sniff editors'

# Criterion baselines
cargo bench -p sniff --bench perf -- filesystem_git filesystem_repo filesystem_staged inventory
```

**Effort:** Small (mostly mechanical; ~30 min runtime)

---

### Item 3: Document Dependency Feature Decisions

**Gap:** The `which` crate already has trimmed features, but this was not explicitly validated with `cargo bloat` / `cargo llvm-lines` as the plan required. No decision record exists for the CLI feature split idea.

**Action:**
1. Run `cargo bloat` and `cargo llvm-lines` on current `main`
2. Compare against a temporary branch where `which` is restored to default features to quantify the actual savings
3. Document the decision in `sniff/reviews/2026-05-02-performance-review/baselines.md`
4. If savings are < 50 KB and < 1% compile time, explicitly defer the CLI feature split with justification

**Validation:**
```bash
git checkout -b temp-which-default-features
# edit sniff/lib/Cargo.toml: which = "8.0.0"
cargo bloat -p sniff-cli --release --bin sniff > /tmp/bloat-default.txt
cargo llvm-lines -p sniff-cli --release > /tmp/llvm-default.txt
git checkout main
cargo bloat -p sniff-cli --release --bin sniff > /tmp/bloat-trimmed.txt
cargo llvm-lines -p sniff-cli --release > /tmp/llvm-trimmed.txt
diff /tmp/bloat-default.txt /tmp/bloat-trimmed.txt
diff /tmp/llvm-default.txt /tmp/llvm-trimmed.txt
```

**Effort:** Small (~15 min runtime + analysis)

---

### Item 4: Close Out Review Artifacts

**Gap:** The original `plan.md` still has all unchecked boxes and the review directory contains deleted/untracked files (`review.md` deleted, `review-1.md` untracked, `plan-for-review.md` modified).

**Action:**
1. Update `plan.md` checkboxes to reflect actual completion status
2. Move completed deliverables to a "Completed" section
3. Archive `review-1.md` (the completed review output) alongside `plan.md`
4. Delete or archive `plan-for-review.md` (it appears to be a working draft)
5. Add a `COMPLETION.md` or update the frontmatter of `plan.md` with:
   - Actual completion date
   - List of commits that implemented each phase
   - Summary of performance impact (from baseline measurements)

**Validation:**
```bash
git status sniff/reviews/2026-05-02-performance-review/
```

**Effort:** Small

---

## Recommended Execution Order

1. **Item 1** → Repo package-boundary benchmark (adds missing Phase 4 deliverable)
2. **Item 2** → Run baselines while benchmark is fresh (saves time by running everything together)
3. **Item 3** → Document dependency decisions using baseline data
4. **Item 4** → Close out review artifacts (depends on items 1–3 for accurate status)

---

## Simplified Done Criteria

- [ ] `cargo bench -p sniff --bench perf -- repo` runs without error
- [ ] `sniff/reviews/2026-05-02-performance-review/baselines.md` exists with:
  - Compile-time measurements (`cargo bloat`, `cargo llvm-lines`)
  - Runtime measurements (Criterion summaries, `hyperfine` results)
  - Dependency feature decision rationale
- [ ] `plan.md` checkboxes accurately reflect completed vs. remaining work
- [ ] `cargo test -p sniff --lib` and `cargo test -p sniff-cli` pass
- [ ] `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` passes

---

## Risk: None

All code changes are already landed and tested. This follow-up plan only adds benchmarks, measurements, and documentation. No production behavior changes.
