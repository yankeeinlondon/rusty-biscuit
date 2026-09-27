---
ready: true
agent: codex
model: ""
---

# Review: `wt list` Cache + Concurrency, Iteration 2

## Findings

No blocking findings.

The issues from review 1 appear addressed:

- The missing-`HEAD` SHA path now skips the cache but still runs live `rev-list` / `merge-tree` for a named non-main branch, with a regression test at `worktree/lib/src/worktree.rs:1132`.
- The warm and cold SLA gates now parse the `list gather` stage from `wt list --perf` instead of asserting only full-command wall time, and they use a mixed multi-worktree fixture with divergent, fast-forward, and behind-only branches.
- The speculative cold-path `merge-tree` tradeoff is now bounded by the mixed cold-cache SLA and deterministic subprocess-count tests.

## Test Coverage Classification

- Cache storage, corrupt/missing cache handling, version invalidation, atomic writes, and canonical repo-root cache path: Level 1 unit coverage in `worktree/lib/src/cache.rs`.
- Cache hit/miss behavior, branch/default tip invalidation, missing-`HEAD` live fallback, and cold/warm subprocess counts: Level 1 recorder-backed library tests in `worktree/lib/src/worktree.rs`.
- CLI graph/list overlap: Level 1 recorder-order coverage in `run_pipeline_graph_git_calls_begin_before_list_gather_completes`.
- Warm/cold performance targets: Level 1 binary-spawn integration tests parsing `wt list --perf` `list gather` timing on a controlled mixed fixture.
- Table/verbose/graph rendering: existing Level 2 tests cover real-terminal rendering surfaces. I did not rerun the Level 2 recipe during this review.
- No Level 3 coverage is required for this feature; the spec does not assert OS keyboard/input-encoder behavior.

## Verification Run

- `cargo test -p worktree --color=never` passed.
- `cargo test -p worktree-cli --color=never --lib` passed.
- `cargo test -p worktree-cli --color=never --test cache_warm_path --test cache_cold_path` passed.
- `cargo test -p worktree-cli --color=never --test perf_command_sla --test perf_flag` passed.
- `md hash worktree/docs/performance-testing.md` matched the committed hash frontmatter.

## Readiness

Ready for production.
