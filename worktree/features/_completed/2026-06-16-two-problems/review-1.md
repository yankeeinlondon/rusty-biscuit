---
ready: false
agent: codex
model: ""
---

# Review: `wt list` cache + concurrency

## Findings

### High: Performance SLA tests do not verify the spec's target workload or metric

The spec's success criteria are about warm `list gather` collapsing below the live dirty-walk cost and cold full-command behavior improving on the monorepo-shaped workload. The new SLA tests in `worktree/cli/tests/cache_warm_path.rs:20` and `worktree/cli/tests/cache_cold_path.rs:20` spawn a tiny fixture with one linked feature worktree, assert full-command wall time only, and never parse `--perf` to verify the `list gather` stage. That can pass while the intended 16-worktree / many-divergent-branches scenario regresses, while graph/list overlap regresses, or while `list gather` itself remains above the target.

Strongest verification present: Level 1 binary-spawn integration tests with best-of-5 wall-clock checks on a small fixture.

Required verification: Level 1 is the right level for this performance requirement, but it needs the right observable: `wt list --perf` stage timings on a controlled multi-worktree fixture, plus a cache-cold run shaped enough to exercise several divergent branches. The current tests are useful smoke tests, not readiness gates for the stated SLA.

### Medium: Missing `HEAD` SHA / detached cache-bypass path returns bogus clean/no-commit status instead of running live

`branch_status_with_cache` returns `(0, 0, true)` whenever `entry.head_sha.is_none()` in `worktree/lib/src/worktree.rs:277`, before checking whether a branch name is available. The plan/spec called for entries without a cache key to skip the cache and run the live branch comparison. The added test at `worktree/lib/src/worktree.rs:1127` locks in the opposite behavior by asserting zero `rev-list` and zero `merge-tree` calls for a non-main entry with a branch but no `HEAD` SHA.

This makes the degraded path silently report no ahead/behind commits and a clean merge result. Even if modern `git worktree list --porcelain` normally emits `HEAD`, the implementation should not turn "cannot build cache key" into "branch is equivalent to main".

Strongest verification present: Level 1 unit test, but it verifies the wrong behavior.

Required verification: Level 1 unit coverage that a non-main branch with no `head_sha` skips cache lookup but still calls live `rev-list` / `merge-tree`, or an explicit documented decision that such entries are unsupported.

### Medium: Cold-path concurrency adds `merge-tree` work for fast-forward branches without measuring the tradeoff on representative fixtures

`gather_ahead_behind_clean` now launches `check_clean_merge` unconditionally for every cache miss in `worktree/lib/src/worktree.rs:319`, then discards the result when `ahead == 0 || behind == 0`. This matches the speculative-concurrency idea for mostly divergent branches, but it is a performance regression for repositories with many fast-forward or behind-only worktrees because those branches previously avoided `merge-tree` entirely. The tests at `worktree/lib/src/worktree.rs:921` and `worktree/lib/src/worktree.rs:986` explicitly expect the extra speculative `merge-tree` call for a fast-forward branch, but there is no benchmark/SLA fixture showing the tradeoff is acceptable outside the divergent-heavy case.

Strongest verification present: Level 1 recorder tests confirm the extra subprocesses happen.

Required verification: Level 1 benchmark/SLA coverage for a mixed fixture with several fast-forward/behind-only branches, or a bounded concurrency policy that avoids making non-divergent-heavy repositories slower.

## Test Coverage Classification

- Cache key hit/miss, tip invalidation, and corrupt/missing cache files: Level 1 unit and integration coverage.
- CLI pipeline graph/list overlap: Level 1 recorder-order coverage in `run_pipeline_graph_git_calls_begin_before_list_gather_completes`.
- Table/verbose/graph rendering: existing Level 2 tests in `level2_list_verbose.rs` cover real-terminal rendering broadly; this feature did not add a new Level 2 regression specific to byte-for-byte output, but the requirement is mostly guarded by existing L2 coverage plus one Level 1 hardcoded output test.
- Performance readiness: Level 1 is appropriate, but current fixtures and measured metric are insufficient for the stated success criteria.

## Verification Run

- `cargo test -p worktree-cli --color=never --lib` passed.
- `cargo test -p worktree-cli --color=never --test cache_warm_path --test cache_cold_path` passed.
- `cargo test -p worktree --color=never` failed once in `config::tests::resolve_base_dir_rejects_nonexistent`; the same test passed when rerun in isolation, so this appears to be pre-existing global environment/test parallelism fragility rather than a direct feature failure.

## Readiness

Not ready for production. The implementation has the core cache shape, but the performance gates do not prove the promised `list gather` improvement on the target workload, and the missing-HEAD cache-bypass behavior is incorrect.
