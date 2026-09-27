---
agent: "open_code/minimax/MiniMax-M3"
phases: 4
created: 2026-06-16
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - worktree/cli/src/commands/list.rs
  - worktree/lib/Cargo.toml
  - worktree/lib/src/cache.rs
  - worktree/lib/src/lib.rs
  - worktree/lib/src/worktree.rs
source_files_during_phase_2:
  - worktree/lib/src/worktree.rs
source_files_during_phase_3:
  - worktree/cli/src/commands/list.rs
  - worktree/lib/src/worktree.rs
source_files_during_phase_4:
  - worktree/cli/tests/cache_warm_path.rs
  - worktree/cli/tests/cache_cold_path.rs
  - worktree/cli/tests/perf_command_sla.rs
docs_updated_during_phase_1:
  - worktree/features/2026-06-16-two-problems/plan.md
docs_created_during_phase_1: []
docs_updated_during_phase_2:
  - worktree/features/2026-06-16-two-problems/plan.md
docs_created_during_phase_2: []
docs_updated_during_phase_3:
  - worktree/features/2026-06-16-two-problems/plan.md
docs_created_during_phase_3: []
docs_updated_during_phase_4:
  - worktree/docs/performance-testing.md
  - worktree/README.md
  - docs/dependencies.md
  - worktree/features/2026-06-16-two-problems/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_1:
  - .claude/skills/worktree/SKILL.md
skills_files_updated_during_phase_2: []
skills_files_updated_during_phase_3:
  - .claude/skills/worktree/SKILL.md
skills_files_updated_during_phase_4:
  - .opencode/skill/worktree/SKILL.md
  - .claude/skills/worktree/SKILL.md
source_code:
  - worktree/lib/src/lib.rs
  - worktree/lib/src/worktree.rs
  - worktree/lib/src/cache.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/tests/cache_warm_path.rs
  - worktree/cli/tests/cache_cold_path.rs
  - worktree/cli/tests/perf_command_sla.rs
documentation:
  - worktree/docs/performance-testing.md
  - worktree/README.md
  - docs/dependencies.md
  - worktree/features/2026-06-16-two-problems/plan.md
  - .opencode/skill/worktree/SKILL.md
  - .claude/skills/worktree/SKILL.md
packages:
  - worktree
  - worktree-cli
---

# Execution Plan — `wt list` Slowness (cache + concurrency)

Source spec: [`spec.md`](./spec.md) (status: ready for implementation).
Track 2 (aspect ratio) is parked and receives no work in this plan.

## Plan Overview

Spec's Track 1 has two prongs that target distinct failure modes:

1. **SHA-keyed result cache** (primary; warm-run win) — cache
   `(default_tip_sha, branch_tip_sha) → { ahead, behind, is_clean }` so a
   steady-state `wt list` collapses `list gather` toward the live dirty-walk
   cost.
2. **Concurrency fixes** (cold-run win) — overlap the two expensive per-thread
   git calls (`rev-list` + `merge-tree`) and overlap the `list gather` and
   `graph gather` pipeline stages.

The four phases land in dependency order: cache plumbing + integration (1),
per-thread concurrency on top of it (2), pipeline-level concurrency in the CLI
(3), then SLA guards + docs that ratify the achieved targets (4). Track 2 is
recorded as parked in a single Phase-4 documentation step so the investigation
isn't lost.

**Dependency graph:**

```
Phase 1 (cache module + wire) ──> Phase 2 (per-thread concurrency) ──┐
                                                                     ├──> Phase 4 (SLA + docs)
                                          ┌──> Phase 3 (pipeline) ────┤
                                          │       (parallel with 2)   │
                                          └───────────────────────────┘
```

- **Phase 2 and Phase 3 are parallelizable** — they touch disjoint files
  (`worktree/lib/src/worktree.rs` vs `worktree/cli/src/commands/list.rs`).
  Phase 2 modifies the per-worktree gather loop; Phase 3 modifies the
  CLI pipeline. They meet again in Phase 4.
- Phase 4 (SLA guards + docs) depends on Phases 1, 2, 3 landing so it can
  measure and ratify the achieved warm/cold targets.

**Target outcomes** (per spec success criteria; numeric values to be ratified
in Phase 4 from measured numbers):

- Warm `list gather` (cache hit) collapses toward live dirty-walk cost.
- Cold `list gather` improves via de-serialization of `rev-list` and `merge-tree`.
- Full command improves via `graph gather` overlap with `list gather`.
- Table + graph output byte-for-byte unchanged.
- No regression in cold-path subprocess counts (the existing
  `merge-base` / `log` / `rev-parse --short` bounds still hold).
- Cache correctness: a tip move (commit on a branch, or `main` advancing)
  yields a fresh, correct `(ahead, behind, is_clean)`.

**Reference templates used:**

- Existing fixtures: `temp_repo_with_branches` (`worktree/cli/src/commands/git_graph.rs:503-543`),
  `temp_repo_with_feature_branch` (`worktree/cli/src/commands/list.rs:532-547`).
- Recorder-backed assertions: `worktree::git::recorder` (the `count-git` feature
  gate; pattern used by `list_worktrees_resolves_default_branch_once` in
  `worktree/cli/src/commands/list.rs:551-567`).
- `dirs = "6"` is already a `worktree/lib/Cargo.toml` dependency
  (`worktree/lib/Cargo.toml:14`) — no new crate additions expected for the
  user-cache-dir path.
- `--perf` (Phase 2 of the prior perf-measurement plan) is the measurement
  surface; no new instrumentation is needed beyond optional finer per-call
  attribution during development.

---

## Phase 1 — SHA-keyed result cache (module + integration into `list_worktrees`)

**Goal:** A new `worktree::cache` module stores
`(default_tip_sha, branch_tip_sha) → { ahead, behind, is_clean }` in a small
JSON state file under the user cache dir (keyed by repo root). `list_worktrees`
consults the cache on the per-worktree hot path: on hit it skips `rev-list` and
`merge-tree` entirely; on miss it computes and stores. Both `parse_worktree_list`
and `list_worktrees` are updated to surface the `HEAD` SHA so the cache key can
be built without an extra per-worktree git call.

**Files touched:** `worktree/lib/src/lib.rs`, `worktree/lib/src/worktree.rs`
(new cache module), `worktree/lib/src/cache.rs` (new).

**Parallelizable with:** nothing (foundation).

### Tasks

- [x] **1.1** Capture the `HEAD <sha>` line in `parse_worktree_list` (`worktree/lib/src/worktree.rs:78-123`).
      The line is currently in the porcelain output but skipped (line 108: "We skip HEAD, bare, detached, prunable lines").
      Extend `WorktreeEntry` with `head_sha: Option<String>` (full 40-char SHA), populate it from the `HEAD <sha>` line, and
      update all `WorktreeEntry { ... }` literal construction sites in tests + production code to set it. Update the inline
      `PORCELAIN_SAMPLE` constant in `parse_porcelain_output` (line 506-518) and the `gather_data_shares_one_merge_base_for_graph_and_verbose`
      test fixtures (`worktree/cli/src/commands/list.rs:842-867`) to include realistic `HEAD <sha>` lines (any 40-char hex string is fine;
      the value is opaque to the tests that don't exercise the cache).

- [x] **1.2** Add `default_tip_sha(default_branch: &str) -> Result<String, WorktreeError>` in `worktree.rs` (sibling of
      `default_branch()` at line 57-75). It is a single `git_command(&["rev-parse", default_branch])` — the cost of one extra
      git call total, not one per worktree (per spec §"Cheap key derivation"). Wire it into `list_worktrees` so the default
      tip SHA is computed once and shared across the per-thread handlers (line 161-216).

- [x] **1.3** Create `worktree/lib/src/cache.rs` with the cache surface:
      - `pub const CACHE_FORMAT_VERSION: u32 = 1;` (bump on any structural change to force invalidation).
      - `pub struct CacheKey { pub default_tip_sha: String, pub branch_tip_sha: String, pub version: u32 }`
        (`Eq + Hash + Clone` for the in-memory map; `Serialize + Deserialize` for the on-disk file).
      - `pub struct CacheValue { pub ahead: usize, pub behind: usize, pub is_clean: bool }` (`Serialize + Deserialize`).
      - `pub struct Cache { entries: HashMap<CacheKey, CacheValue> }`.
      - `impl Cache { pub fn load_or_default() -> Self` (graceful on missing file, corrupt JSON, or wrong format version —
        all return an empty cache; corrupt files are not deleted in case a user wants to inspect them), `pub fn get(&self, key: &CacheKey) -> Option<&CacheValue>`,
        `pub fn put(&mut self, key: CacheKey, value: CacheValue)`, `pub fn save_atomic(&self, path: &Path) -> Result<(), WorktreeError>`}.
      - `pub fn cache_path(repo_root: &Path) -> Result<PathBuf, WorktreeError>` — resolves
        `dirs::cache_dir().ok_or(...).join("worktree").join(format!("{}.json", xxhash_of_repo_root))`. Hash the
        canonicalized absolute path so symlinks and the `main` checkout + linked worktree both resolve to the same key
        (use `biscuit_hash::xxhash` per the `Hashing` instruction in `AGENTS.md`; add `biscuit-hash` to
        `worktree/lib/Cargo.toml` `[dependencies]` only if `xxhash` isn't already reachable — confirm during
        implementation; if no new dep is needed, prefer that).
      - `pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), WorktreeError>` — write-temp-then-rename
        (`last-rename-wins` semantics per the rust-testing skill). Don't create parent directories unless they exist
        (we control the path; if a parent is missing it is unexpected and should error).
      - Module-level `//!` doc explaining: deterministic SHA-pair key, self-invalidating on tip change, opportunistic
        stale-entry tolerance, no `dirty_status` cached.

- [x] **1.4** Wire the cache into `list_worktrees` (`worktree/lib/src/worktree.rs:161-216`):
      - Compute `default_tip` once via `default_tip_sha(&default_branch)` before the `std::thread::scope` block.
        Fall back to skipping the cache entirely if this call errors (graceful degraded mode — see spec §"Storage").
      - In the per-thread handler: build `CacheKey { default_tip_sha: default_tip, branch_tip_sha: entry.head_sha.clone(), version: CACHE_FORMAT_VERSION }`.
        If `entry.is_main` skip the cache (always `(0, 0, true)`); if `entry.head_sha` is `None` (detached) skip the cache and run live.
        Else look up in the shared `Cache`; on hit use the cached `(ahead, behind, is_clean)` and skip both `ahead_behind`
        and `check_clean_merge` entirely. On miss, run `ahead_behind` + `check_clean_merge` (only the latter if both are > 0,
        preserving the current fast-forward shortcut) and record the result.
      - Move the `Cache::load_or_default()` call above the `scope` (cheap — one read) and `Cache::save_atomic` after the
        `scope` joins. The shared `Arc<Mutex<Cache>>` lives across threads; on a miss the thread inserts into it via a
        short critical section.
      - Optional: when `perf` is in scope (gated on a public function in `cache.rs` that records which sub-stages ran),
        attribute the cache hit as `"cache hit"` and the miss path as the original `rev-list` / `merge-tree` stages. This
        is opt-in dev instrumentation only — do not change the public CLI surface or the existing `--perf` stage labels.

- [x] **1.5** Add unit tests in `worktree/lib/src/cache.rs` (`#[cfg(test)] mod tests`):
      - `cache_round_trip_atomic` — `put` + `save_atomic` + reload yields the same entries (use `tempfile::tempdir()`).
      - `cache_corrupt_json_returns_empty` — write garbage to the path, `load_or_default` returns empty (does not panic).
      - `cache_missing_file_returns_empty` — `load_or_default` on a non-existent path returns empty.
      - `cache_wrong_version_returns_empty` — bump `CACHE_FORMAT_VERSION` for the saved file's stored version; load returns
        empty without touching the file (proves the version gate works).
      - `cache_path_uses_canonical_repo_root` — two paths that canonicalize to the same location produce the same
        `cache_path` (covers the symlink/`/var` case from `is_current_worktree`, `worktree.rs:131-141`).
      - `atomic_write_concurrent_writers_last_rename_wins` — two threads racing on the same path; final read returns one
        writer's bytes (no torn/half-written file).

- [x] **1.6** Add recorder-backed unit tests in `worktree/lib/src/worktree.rs` (`#[cfg(test)] mod tests`):
      - `list_worktrees_warm_run_skips_rev_list_and_merge_tree` — use `temp_repo_with_branches` style fixture with `main`
        + 2 feature branches that have diverged. First call populates the cache. Second call inside a recorder window
        asserts zero `rev-list` and zero `merge-tree --write-tree` calls. Assert the `(ahead, behind, is_clean)` outputs
        are unchanged between the two calls.
      - `list_worktrees_branch_tip_advance_invalidates_cache_entry` — call once (cache miss, populates entry for branch X).
        Commit one new commit on branch X (advancing its tip). Call again inside a recorder window: assert at least one
        `rev-list` and at least one `merge-tree` call ran (cache miss) and the returned `ahead` reflects the new commit.
      - `list_worktrees_default_tip_advance_invalidates_cache_entry` — symmetric: commit one new commit on `main`,
        assert the next call recomputes and reflects the new `behind`.
      - `list_worktrees_detached_worktree_skips_cache` — a `WorktreeEntry` with `head_sha: None` runs live even on a
        warm cache. (Synthetic test, not a fixture assertion — pass a crafted entry list directly to the per-thread
        logic via a new test seam if needed, or assert via the `parse_worktree_list` path with a `HEAD` line absent.)

### Validation Checkpoint (Phase 1)

- [x] **V1.1** `cargo build -p worktree` succeeds with no new warnings; `cargo build -p worktree-cli` still builds.
- [x] **V1.2** `cargo nextest run -p worktree -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/))'`
      passes (existing tests + new cache tests).
- [x] **V1.3** `wt list --perf` on a real feature branch reports a noticeably lower `list gather` cost on the second
      run than on the first (the cold/warm delta is visible). Compare with `bench-save` / `bench-compare` to record
      the ambient delta.
- [x] **V1.4** `wt list` (no `--perf`) output is byte-for-byte identical to pre-change (table cells, graph instructions,
      color markup all unchanged).
- [x] **V1.5** `just -d worktree lint` passes (clippy clean for the new module).
- [x] **V1.6** On a controlled non-main fixture, the warm path makes zero `rev-list` and zero `merge-tree` calls
      (recorder assertion).

---

## Phase 2 — De-serialize per-thread `rev-list` and `merge-tree` (cold-run win)

**Goal:** On a cache miss, run `ahead_behind` and `check_clean_merge` concurrently
per worktree instead of serially. When `ahead == 0 || behind == 0`, discard the
speculative `merge-tree` result. The per-thread chain collapses from
`rev-list + merge-tree` (~177 ms) toward `max(rev-list, merge-tree)` (~132 ms).

**Files touched:** `worktree/lib/src/worktree.rs` (the per-thread handler at
`worktree.rs:166-210`).

**Parallelizable with:** Phase 3 (disjoint file set; Phase 3 touches
`worktree/cli/src/commands/list.rs`).

**Depends on:** Phase 1 (the per-thread handler is being modified).

### Tasks

- [x] **2.1** Restructure the per-thread handler in `list_worktrees` to fan out
      `ahead_behind` and `check_clean_merge` concurrently. The simplest correct
      shape: one `std::thread::scope` inside each per-thread worktree handler
      that spawns `ahead_behind` on one handle and `check_clean_merge` on another
      (the latter is unconditional; previously it was gated on `ahead > 0 && behind > 0`,
      and the cheap fast-forward shortcut is preserved by discarding its result when
      either count is zero). Join both, then apply the discard. Both calls already
      have a graceful-degraded return (`ahead_behind` → `(0, 0)` on git error;
      `check_clean_merge` → `false` on git error) so the concurrency wrapper can
      stay simple — no error propagation, no cancellation.

- [x] **2.2** When the cache is warm, **skip the entire speculative fan-out**
      (this is the common case). The cache-hit path from Phase 1 must remain
      exactly as fast as it is after Phase 1 — no thread spawn, no extra git
      call. A new helper `fn gather_ahead_behind_clean(...)` keeps the
      cache-hit / cache-miss branches in one place so the two paths are easy to
      read in diff.

- [x] **2.3** When the worktree is `is_main` or has no `head_sha` (detached), keep
      the existing `(0, 0, true)` short-circuit — no fan-out, no cache lookup,
      no git calls.

- [x] **2.4** Optional dev-time per-call attribution: split the existing
      `list gather` stage into `list gather / cache lookups`, `list gather / rev-list`,
      `list gather / merge-tree`. **Gated on the same mechanism Phase 1.4 used for
      `"cache hit"`** (i.e. internal-only, not exposed via `--perf` unless
      `--perf` is re-instrumented as a follow-up). Default behavior is unchanged:
      the public `--perf` stage list still shows `list gather` as a single
      aggregated stage. Document the decision in a `// notes:` comment near
      the new helper so a future contributor doesn't expose the dev-only
      attribution by accident.

- [x] **2.5** Add recorder-backed tests in `worktree/lib/src/worktree.rs`:
      - `list_worktrees_cold_run_invokes_rev_list_and_merge_tree_per_branch`
        — controlled fixture with `main` + divergent and fast-forward branches.
        Cache empty (delete it first). Assert exactly one `rev-list` and one
        speculative `merge-tree --write-tree` per non-main branch, and zero of
        either branch-comparison call on `main`.
      - `list_worktrees_fast_forward_branch_discards_merge_tree_result`
        — fixture with a branch that is ahead but not behind (`ahead > 0, behind == 0`).
        Assert the speculative `merge-tree` is launched but its result is discarded
        (the `is_clean == true` shortcut still wins). This is a behavioral
        assertion, not a timing one — assert via the recorder that the result
        of the speculative call is read but not used.
      - `list_worktrees_warm_run_subprocess_count_unchanged` — after a warm-up
        call populates the cache, the second call's recorder shows zero
        `rev-list` and zero `merge-tree` calls AND zero new `thread::spawn`
        work for the speculative pair (the recorder counts git subprocesses,
        not threads; assert by checking that the warm call is strictly bounded
        by the cold call's count).
      - `list_worktrees_cold_path_subprocess_counts_match_existing_sla`
        — with cache disabled, the existing
        `perf_subprocess_counts_meet_sla` and the cold-path regression
        tests in `git_graph.rs` still pass byte-identically (no regression in
        cold-path subprocess counts; spec §"Scope / non-goals").

### Validation Checkpoint (Phase 2)

- [x] **V2.1** `cargo build -p worktree` succeeds; existing tests still pass.
- [x] **V2.2** `wt list --perf` on a clean cache (delete the user-cache file) reports
      a lower `list gather` wall-clock than the pre-Phase-2 baseline. Compare
      with `bench-save` / `bench-compare` to record the cold delta.
- [x] **V2.3** `wt list --perf` on a warm cache is unchanged from the Phase-1
      warm performance (no regression in the common case).
- [x] **V2.4** `wt list` output is byte-for-byte identical to pre-Phase-2
      (table cells, graph, color markup).
- [x] **V2.5** The existing `gather_branch_uses_one_merge_base_and_no_short_sha`
      (`worktree/cli/src/commands/git_graph.rs:609-649`) and
      `base_graph_subprocess_count_is_bounded` (`worktree/cli/src/commands/git_graph.rs:767-818`)
      tests still pass — no regression in the per-call graph subprocess counts.

---

## Phase 3 — Overlap `graph gather` with `list gather` (pipeline-level concurrency)

**Goal:** Start the `graph gather` (currently `gather_base_graph` or `gather_branch` in
`worktree/cli/src/commands/list.rs:208-240`) as soon as the cheap
`git worktree list --porcelain` parse + `default_branch` are available — before
the dirty/merge work in `list_worktrees` runs. Hide the ~110 ms `graph gather`
cost behind the `list gather` cost so the full command's wall-clock is closer
to `max(list gather, graph gather)` than to their sum.

**Files touched:** `worktree/cli/src/commands/list.rs` (the `run_pipeline`
function at `worktree/cli/src/commands/list.rs:48-145` and the `gather_data`
orchestration at `list.rs:208-240`).

**Parallelizable with:** Phase 2 (disjoint file set).

**Depends on:** Phase 1 (the early-return of `default_branch` and the worktree
list now needs to happen before `list gather`'s expensive work). The cache
itself isn't required, but the early-parse step is.

### Tasks

- [x] **3.1** Split `list_worktrees` into two stages inside `run_pipeline`:
      - **Stage A (cheap):** `git worktree list --porcelain` + `parse_worktree_list` +
        `default_branch` + `default_tip_sha`. All together: ~10 ms.
      - **Stage B (expensive):** per-worktree dirty + ahead/behind + merge analysis.

      The simplest decomposition: introduce `pub fn parse_worktree_state() -> Result<WorktreeList, WorktreeError>` in
      `worktree.rs` that does Stage A and returns a `WorktreeList` with empty `statuses`, plus a separate
      `pub fn fill_worktree_statuses(list: &mut WorktreeList) -> Result<(), WorktreeError>` for Stage B. Keep the existing
      `list_worktrees()` as a thin wrapper that calls both sequentially (preserves the public API for any external callers
      and the existing `list_worktrees_resolves_default_branch_once` test at
      `worktree/cli/src/commands/list.rs:551-567`).

- [x] **3.2** In `run_pipeline` (`worktree/cli/src/commands/list.rs:48-145`), spawn the
      graph gather on a background thread as soon as Stage A completes. The graph
      gather needs only `default_branch` and the branch *names* (cheap strings
      from the parse). Join it before the table render — the table needs the
      statuses but the graph needs neither.

      Sketch:
      ```rust
      let state = parse_worktree_state()?;
      // Stage B: list gather runs on this thread.
      let graph_handle = if needs_graph { Some(scope.spawn(|| {
          gather_data(&state.default_branch, &[], needs_graph, needs_verbose)
      })) } else { None };
      // ... fill_worktree_statuses(&mut state) ...
      let graph_data = graph_handle.map(|h| h.join().expect("graph thread panicked"));
      ```

      Note: the graph gather needs a `WorktreeList`-like input that doesn't yet
      have the populated `statuses` (it only reads `default_branch` and the
      branch *names*). Pass `&[]` for the statuses and the `default_branch`
      string directly, or refactor `gather_data` to take a `default_branch: &str`
      and a `branch_names: &[String]` separately. The latter is cleaner and
      makes the early-parse seam explicit.

- [x] **3.3** Decide on a `std::thread::scope` (not `std::thread::spawn`) so
      the graph handle is joinable before `run_pipeline` returns. The current
      `run_pipeline` signature has no `scope` parameter; the easiest fix is to
      wrap the whole pipeline body in a `std::thread::scope(|scope| { ... })`.
      This is structurally similar to the per-thread worktree handler already
      in `worktree::list_worktrees` (`worktree.rs:166-210`) — same idiom, same
      trade-offs.

- [x] **3.4** Keep the `--perf` stage labels honest. The current stages
      (`pre-dispatch`, `list gather`, `table render`, `graph gather`,
      `graph image render (biscuit-terminal)`, `unattributed`) still make sense
      post-change:
      - `list gather` = Stage B (`fill_worktree_statuses`).
      - `graph gather` = the joined graph gather.
      - The new overlap means `list gather` + `graph gather` < `list gather` + `graph gather` (serial), but the
        per-stage numbers stay individually meaningful. `unattributed` shrinks because more time is attributed to
        known stages.
      - Do not add new stages; do not split `list gather` into pre/post sub-stages. The dev-only per-call attribution
        from Phase 2.4 stays internal.

- [x] **3.5** Add recorder-backed tests in `worktree/cli/src/commands/list.rs`:
      - `run_pipeline_graph_git_calls_begin_before_list_gather_completes`
        — start the recorder *before* `run_pipeline` runs on a controlled
        fixture with a feature branch + image-capable terminal. Inspect the
        recorded calls and assert at least one `merge-base` or `log` call
        appears **before** the last `rev-list`/`merge-tree --write-tree` call
        (which is the trailing per-worktree step in `list gather`). This is a
        behavioral guarantee, not a timing-flaky check — if the implementation
        regresses to serial, the relative order flips.
      - `run_pipeline_output_byte_for_byte_unchanged` — golden-file test or
        hash compare. Compare the `wt list` output (table + graph instructions
        + verbose block if applicable) on a controlled fixture against a stored
        snapshot. Locks the spec's "output byte-for-byte unchanged" success
        criterion in CI.
      - `run_pipeline_narrow_image_omits_graph_gather_stage` — the existing
        test at `worktree/cli/src/commands/list.rs:733-770` should still pass
        unchanged; this is a regression check, not a new test.

### Validation Checkpoint (Phase 3)

- [x] **V3.1** `cargo build -p worktree-cli` succeeds; existing tests still pass.
- [x] **V3.2** `wt list --perf` on a warm cache + image-capable terminal reports
      `list gather` and `graph gather` with similar magnitudes (neither is a
      tail on the other). The `unattributed` line should be smaller than the
      pre-Phase-3 baseline because the overlap moves time from unattributed
      into the two known stages.
- [x] **V3.3** `wt list` (no `--perf`) output is byte-for-byte identical to
      pre-Phase-3 (golden test from V3.5).
- [x] **V3.4** On a non-image terminal, no `graph gather` stage is recorded
      (existing test at `worktree/cli/src/commands/list.rs:733-770` still passes).
- [x] **V3.5** `just -d worktree lint` passes.
- [x] **V3.6** The existing `perf_full_command_non_image_meets_sla` test
      (`worktree/cli/tests/perf_command_sla.rs:29-60`) still meets the 1-second
      SLA on a warm cache. The image-terminal equivalent
      `perf_full_command_image_meets_sla` (if it exists) is not regressed.

---

## Phase 4 — SLA guards, documentation, and target ratification

**Goal:** Add controlled-fixture SLA guards for the warm and cold paths so a
regression past the ratified targets fails CI. Document the new cache module
and the achieved warm/cold targets in `worktree/docs/performance-testing.md`
and `worktree/README.md`. Update the worktree skill so future contributors
know the cache exists. Park Track 2 (aspect ratio) in the perf doc so the
investigation isn't lost if it recurs.

**Files touched:** `worktree/cli/tests/cache_warm_path.rs` (new),
`worktree/cli/tests/cache_cold_path.rs` (new),
`worktree/cli/tests/perf_command_sla.rs` (extended),
`worktree/docs/performance-testing.md`, `worktree/README.md`,
`.opencode/skill/worktree/SKILL.md`, `.claude/skills/worktree/SKILL.md`.

**Depends on:** Phases 1, 2, 3 (SLA must reflect the achieved numbers).

**Parallelizable with:** nothing (final phase).

### Tasks

- [x] **4.1** Create `worktree/cli/tests/cache_warm_path.rs` — SLA guard for the
      warm path. On a controlled fixture (one `main` + one divergent feature
      branch with `core.untrackedCache=true` warm), spawn the built `wt` binary
      five times and assert the best-of-5 wall-clock is below the ratified warm
      target. Pattern after
      `worktree/cli/tests/perf_command_sla.rs:29-60` (serial, best-of-5).
      Mark `#[serial]` so parallel-test contention does not produce false
      failures.
      - **Do not hard-code a duration in this plan.** Run Phases 1-3, measure
        the achieved warm `list gather` (or full non-image `wt list` on a
        controlled fixture), then write the asserted bound in this test. If
        the achieved number is too loose or too tight for a stable CI gate,
        follow the spec's open-question guidance: ratify the bound that
        catches real regressions without flaking on host variation.

- [x] **4.2** Create `worktree/cli/tests/cache_cold_path.rs` — SLA guard for
      the cold path. Same shape as 4.1, but with the user-cache file deleted
      before each timed run (so every run is a cache miss). Mark `#[serial]`.
      Again: assert the bound that catches real regressions without flaking.
      - If the cold-path SLA is too flaky to assert deterministically (the
        spec acknowledges the cold path involves the longest per-thread
        chain of git calls, and host state varies), follow the
        `perf_full_command_non_image_meets_sla` pattern: best-of-5 with a
        tolerance, and a separate deterministic subprocess-count bound.
        Document the chosen bound style in the test's module-level docs.

- [x] **4.3** Extend `worktree/cli/tests/perf_command_sla.rs`:
      - Add a comment block at the top noting the two new sibling SLA tests
        (`cache_warm_path.rs`, `cache_cold_path.rs`) and that the ratified
        warm/cold targets are documented in
        `worktree/docs/performance-testing.md` §"Ratified SLA Targets".
      - Confirm the existing 1-second full-command bound still holds; tighten
        it only if the measured warm+graph-overlap result leaves headroom.
        Do not loosen the existing bound without spec-level review.

- [x] **4.4** Update `worktree/docs/performance-testing.md`:
      - Add a new `## Ahead/Behind + Merge Result Cache` section (placement:
        after `## Graph Data Collection` and before `## Excluded Surfaces`).
        Document: SHA-pair key, deterministic / self-invalidating, storage
        location (`dirs::cache_dir() + "worktree" + <hash>.json`),
        `last-rename-wins` atomic write, `CACHE_FORMAT_VERSION` for forced
        invalidation, opportunistic stale-entry tolerance, no `dirty_status`
        cached. Link the SHA-pair key back to spec §"Approach (decided)".
      - Add a new `## Ratified SLA Targets` section at the end. Replace the
        TBD targets from the spec with the measured numbers from 4.1/4.2.
        Include the achieved best-of-5, the chosen asserted bound, and the
        measurement recipe (`just -d worktree test-perf`). If the bounds
        differ between warm and cold, document both.
      - Add a new `## Track 2 — Aspect Ratio (Parked)` section recording
        Track 2 verbatim from the spec, including the candidate fix
        (`biscuit-visualized/src/src/mermaid/render.rs:223-225` + the
        content-bbox crop at `raster/png.rs::render_tree_to_pixmap`) and
        the `MERMAID_BACKEND` cache-bump note. This is a documentation
        archival step, not a fix.

- [x] **4.5** Rehash `worktree/docs/performance-testing.md` (it has a `hash`
      frontmatter property per the repo hashing convention — the doc currently
      declares `hash: ef46db3751d8e999-5f1753d5627d5caa`):
      ```sh
      md hash worktree/docs/performance-testing.md
      ```
      This uses the Darkmatter CLI's Markdown-aware xxHash (frontmatter vs body
      segmentation), per the `Hashing Content` instruction in
      `/var/folders/l9/.../opencode/AGENTS.md`. Confirm with `git diff` that
      only the `hash` value (and the body) changed.

- [x] **4.6** Update `worktree/README.md` (it does not currently have a
      `## Performance` H2 section; the perf-measurement plan from
      `features/2026-06-14-perf-measurement/plan.md:273` added one for
      `--perf` and the bench workflow). Extend that section with a
      `### Ahead/Behind + Merge Result Cache` subsection that:
      - States that `wt list` caches `(ahead, behind, is_clean)` per branch.
      - States the cache file location (one-line summary, full detail in
        `docs/performance-testing.md`).
      - Notes that the cache is self-invalidating on tip change and that
        `CACHE_FORMAT_VERSION` invalidates it on schema change.
      - Does **not** include numeric SLA targets in the README (they drift;
        per the existing convention from the perf-measurement plan).

- [x] **4.7** Update `.opencode/skill/worktree/SKILL.md` and
      `.claude/skills/worktree/SKILL.md` (both files are kept in sync) to
      note that `list_worktrees` has an internal cache and the
      `parse_worktree_state` / `fill_worktree_statuses` seam from Phase 3.1.
      This is a small structural note in the package overview — no new
      workflows, no new commands, no new gotchas for typical work.

- [x] **4.8** Confirm `docs/dependencies.md` is unchanged. Per
      `AGENTS.md` §"Drift Maintenance", update per-area `docs/dependencies.md`
      when crates are added/removed. Phase 1.3 notes that `biscuit-hash` is
      preferred but only added if a needed primitive (e.g. `xxhash`) is not
      already reachable. If no new dep is added, no `docs/dependencies.md`
      change is required. If a new dep is added, update both the root
      `docs/dependencies.md` and `worktree/docs/dependencies.md` (if it
      exists) in this task.

### Validation Checkpoint (Phase 4)

- [x] **V4.1** `just -d worktree test` passes (Level-1, includes all new and
      existing tests).
- [x] **V4.2** `just -d worktree test-perf` passes (the SLA tests, including
      the new `cache_warm_path` and `cache_cold_path` files, run serially).
- [x] **V4.3** `just -d worktree lint` passes.
- [x] **V4.4** `worktree/docs/performance-testing.md` has the three new
      sections (`Ahead/Behind + Merge Result Cache`, `Ratified SLA Targets`,
      `Track 2 — Aspect Ratio (Parked)`) and its `hash` frontmatter is
      updated.
- [x] **V4.5** `worktree/README.md` `## Performance` section mentions the
      cache (no numeric SLA targets).
- [x] **V4.6** The worktree skill files mention the internal cache and the
      new parse-state seam.
- [x] **V4.7** `docs/dependencies.md` is current (no change required if no
      new dep was added in Phase 1.3).

---

## Final Acceptance Checkpoint (all phases)

Mirrors the spec's success criteria. Run after all four phases land.

- [x] **A1** Warm `wt list` (cache hit) `list gather` is materially faster than
      the pre-change ~470 ms baseline; the asserted bound in
      `cache_warm_path.rs` is a real regression gate.
- [x] **A2** Cold `wt list` (cache miss, all tips stale) `list gather` improves
      via `rev-list` + `merge-tree` concurrency; the asserted bound in
      `cache_cold_path.rs` is a real regression gate.
- [x] **A3** Full non-image `wt list` improves via `graph gather` overlap with
      `list gather`; `perf_full_command_non_image_meets_sla` still meets the
      1-second SLA and is tightened if the measured result has headroom.
- [x] **A4** `wt list` output is byte-for-byte unchanged (golden-file test
      from Phase 3.5 in CI).
- [x] **A5** Cache correctness: a tip move (commit on a branch, or `main`
      advancing) yields a fresh, correct `(ahead, behind, is_clean)` —
      verified by `list_worktrees_branch_tip_advance_invalidates_cache_entry`
      and `list_worktrees_default_tip_advance_invalidates_cache_entry` in
      Phase 1.6.
- [x] **A6** No regression in cold-path subprocess counts — the existing
      `merge-base` / `log` / `rev-parse --short` regression tests still pass.
- [x] **A7** `worktree/docs/performance-testing.md` documents the cache
      module, the ratified warm/cold targets, and the parked Track 2.
- [x] **A8** `worktree/README.md` `## Performance` section mentions the
      cache (no numeric SLA targets).
- [x] **A9** All existing tests pass unchanged; new tests cover cache hit
      / miss / version-bump / atomic-write / tip-advance, per-thread
      concurrency, pipeline concurrency, output byte-equality, and
      warm/cold SLA.

## Notes for the implementer

- **Do not run `cargo fmt`** (repo convention — `AGENTS.md`).
- **Match existing style** in each file; do not refactor adjacent code
  (Rule 3 — Surgical Changes).
- **Comment discipline** (per `AGENTS.md` and `docs/comment-quality.md`): only
  add comments that carry information the code does not. Remove HOW-narration,
  tautological docs, and stale comments in any file you touch.
- **Rustdoc convention**: no `# H1` inside `///`; use `## H2` sections
  (`Examples`, `Returns`, `Errors`, `Panics`, `Notes`).
- **Hashing**: use `biscuit-hash` for non-Markdown xxHash and Darkmatter
  (`md hash <file>`) for the Markdown-aware hash frontmatter (per the
  `Hashing` and `Hashing Content` instructions in
  `/var/folders/l9/.../opencode/AGENTS.md`).
- **Terminal rendering**: use `biscuit-terminal::components` (`Prose`, etc.)
  for any new CLI output; never raw escape codes.
- **File references**: use `biscuit_file::FileReference` for any string-based
  file references introduced in this work.
- **Non-interactive session**: do not run commands that wait on stdin/tty. Use
  one-shot non-interactive forms.
- **Open question** (spec §"Open question"): numeric warm/cold SLA targets are
  TBD in the spec. Phase 4.1 and 4.2 ratify them from the achieved numbers; do
  not pick a bound a priori.
- **Track 2 is parked** — no implementation work. Phase 4.4 is the only step
  that touches the aspect-ratio investigation, and only for archival
  documentation.
