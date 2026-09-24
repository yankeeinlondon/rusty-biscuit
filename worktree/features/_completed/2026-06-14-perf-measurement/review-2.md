---
ready: false
agent: codex
model: ""
---

# Review 2 — Performance Measurement

Feature is **not ready for production**. Iteration 2 fixes the two review-1 blockers: non-perf runs no longer capture per-stage timers, and narrow image-capable terminals no longer claim a `graph gather` stage when the width gate skips the graph. The remaining gap is in the verbose path: an owned data-gather stage can still disappear into `unattributed`, which undercuts the runtime diagnostic's purpose.

## Findings

### High — non-image verbose gather is not reported as a stage

The spec says `--perf` covers the full runtime pipeline and that the report covers all pipeline stages that actually ran, including verbose behavior (`spec.md:133-135`, `spec.md:428-432`). The performance docs also call verbose commit details an owned cost center gathered independently of image support (`worktree/docs/performance-testing.md:27-33`).

In the implementation, `run_pipeline` starts a timer before `gather_extras`, but records that elapsed time only when `needs_graph` is true:

- `worktree/cli/src/commands/list.rs:82-95`
- `worktree/cli/src/commands/list.rs:164-169`
- `worktree/cli/src/commands/list.rs:198-205`

For `wt list -v --perf` on a non-image terminal while the current worktree is a feature branch, `needs_graph` is false and `needs_verbose` is true. `gather_extras` still calls `gather_data`, which calls `gather_branch` to collect merge-base and commit details for verbose rendering, but the elapsed time is not recorded as `graph gather`, `verbose render`, or any other named stage. The later `verbose render` timer at `worktree/cli/src/commands/list.rs:134-139` only covers rendering the already-gathered data. A slow verbose gather will therefore be hidden under `unattributed`, even though it is worktree-owned diagnostic information.

Suggested fix: split the stage accounting so data gathering is reported whenever it actually performs work. For example, record `verbose gather` when `needs_verbose && !needs_graph`, or rename the existing stage to a broader `graph/verbose gather` and record it when either `needs_graph` or `needs_verbose` is true. Keep `verbose render` for the pure render step.

Verification level: Level 1 is appropriate because this is process output/stage accounting, not a real-terminal encoder/renderer behavior. Add an in-process `run_pipeline` test or binary integration test for a controlled non-main repo with `--perf --verbose`, `ImageSupport::None`, asserting the report/stage list includes the verbose gather cost and still omits graph-only stages.

## Requirement Coverage

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| R1 Criterion bench for `list_worktrees()` | Bench target compile check: `cargo bench -p worktree --bench list_status --no-run --color=never` | Implemented |
| R2 `bench`, `bench-save`, `bench-compare` recipes | Code inspection | Implemented |
| R3 `wt list --perf` runtime diagnostic | Level 1 unit + binary integration tests | Partially implemented; verbose gather attribution gap above |
| R4 README performance docs | Code inspection | Implemented |
| R5 performance-testing docs + hash | `md hash worktree/docs/performance-testing.md` matched frontmatter | Implemented |
| R6 `--perf` tests | Level 1 unit + binary integration tests | Partially implemented; missing non-image verbose `--perf` path |
| AC4 no non-perf stage timing | Level 1 `run_pipeline_without_perf_produces_no_collector` plus code inspection | Implemented |
| AC5 non-image graph stages omitted | Level 1 integration and in-process tests | Implemented |
| AC6 stdout empty for `--perf` | Level 1 binary integration test | Implemented |

No Level 2 or Level 3 coverage is required for the current `--perf` requirements. The feature does not specify terminal key encoding, OS keyboard injection, glyph-width-sensitive rendering, or SGR color fidelity as acceptance behavior. Level 1 binary and in-process tests are the right tier for the CLI diagnostic content and stage accounting.

## Verification Run

- `cargo test -p worktree-cli perf --color=never` — passed.
- `cargo bench -p worktree --bench list_status --no-run --color=never` — passed.
- `md hash worktree/docs/performance-testing.md` — returned `ef46db3751d8e999-5f1753d5627d5caa`, matching frontmatter.

## Recommendation

Do not ship yet. The implementation is close, but `--perf` must attribute the verbose data-gather path before the runtime diagnostic can be trusted for `wt list -v` slowdowns.
