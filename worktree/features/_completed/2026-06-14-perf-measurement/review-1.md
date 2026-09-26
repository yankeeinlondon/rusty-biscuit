---
agent: "codex"
model: ""
ready: false
---

# Review 1

Feature is **not ready for production**. The Criterion bench wiring is present and the `--perf` user path has useful Level 1 coverage, but the implementation violates the explicit non-perf overhead contract and can report a graph stage that did not actually run.

## Findings

### High - Non-perf `wt list` still captures stage timings

The spec is explicit that `wt list` without `--perf` should perform no stage timing and no report rendering; the only unconditional perf-related cost should be the top-of-main `Instant::now()` for future pre-dispatch accounting. The implementation still creates per-stage timers and elapsed samples unconditionally in `list::run`: `process_start.elapsed()` is called for `pre-dispatch`, and `Instant::now()` is captured before `list_worktrees`, table rendering, `gather_extras`, graph image rendering, and verbose rendering even when `collector` is `None`.

Affected code:

- `worktree/cli/src/commands/list.rs:32`
- `worktree/cli/src/commands/list.rs:34`
- `worktree/cli/src/commands/list.rs:40`
- `worktree/cli/src/commands/list.rs:58`
- `worktree/cli/src/commands/list.rs:94`
- `worktree/cli/src/commands/list.rs:108`

This fails R3 and acceptance criterion 4. It is also not covered by the current tests: the tests verify no perf output on the default path indirectly through existing behavior/SLA tests, but they do not verify that stage timers are gated behind `perf`.

Suggested fix: branch the timing captures on `perf`/`collector.is_some()` so the non-perf path does not call `Instant::now()` or `elapsed()` for stages. Keep only the top-level `process_start` capture in `main()`.

Verification level: this is internal/performance behavior, so Level 1 code-level tests are appropriate. Add a focused test seam or structure that makes it possible to assert the non-perf path does not invoke stage timing, plus keep the existing SLA test as a regression guard for wall-clock behavior.

### High - `graph gather` can be reported when graph data gathering was skipped

The docs/spec say the perf report shows only stages that actually ran, and `worktree/docs/performance-testing.md` says graph data is gathered only when image support is present **and** the width gate passes. In `list::run`, `graph_eligible` is computed as `image_support != ImageSupport::None`, then `graph gather` is recorded whenever that is true. The actual graph gather decision is `needs_graph`, returned by `gather_extras`, which also applies `graph_eligible(image_support, parsed_width, terminal.width())`.

Affected code:

- `worktree/cli/src/commands/list.rs:56`
- `worktree/cli/src/commands/list.rs:58`
- `worktree/cli/src/commands/list.rs:67`
- `worktree/cli/src/commands/list.rs:139`
- `worktree/cli/src/commands/list.rs:161`

On an image-capable but narrow terminal with default/character width, `gather_extras` correctly skips graph data gathering, but the perf report still includes a `graph gather` stage containing only gating overhead. That contradicts the user-facing report contract and can mislead users diagnosing slow list output.

Suggested fix: record `graph gather` based on whether graph data was actually requested/gathered, not just whether image support exists. If the desired diagnostic includes gating overhead, label it separately as a dispatch/gating stage; do not call it `graph gather`.

Verification level: this is user-observable CLI output. Level 1 integration coverage is acceptable for the label decision because the behavior is not dependent on terminal encoder/decoder behavior, but the current R6 tests only cover non-image omission. Add an image-capable narrow-width regression that forces the width gate and asserts `graph gather` is absent when `needs_graph` is false.

## Requirement Coverage

- R1 Criterion bench for `list_worktrees`: implemented. Bench target compiles.
- R2 `just bench`, `bench-save`, `bench-compare`: implemented in `worktree/justfile`.
- R3 `--perf` runtime diagnostic: partially implemented; blocked by the two findings above.
- R4 README performance docs: implemented.
- R5 performance-testing docs: implemented and hash frontmatter is present.
- R6 `--perf` tests: Level 1 coverage exists for success output, stderr-only output, non-image graph omission, error-path no-report, and perf-tree reconciliation. Missing Level 1 coverage for non-perf timing overhead and image-capable width-gated graph omission.

No Level 2 or Level 3 tests are required for this feature as specified. The user-observable requirements here are plain CLI output labels and stderr/stdout routing, not real terminal rendering fidelity, key input behavior, mouse behavior, paste/IME behavior, or OS keyboard event encoding.

## Verification Run

- `cargo test -p worktree-cli perf --color=never` - passed.
- `cargo check -p worktree-cli --color=never` - passed.
- `cargo test -p worktree --bench list_status --no-run --color=never` - passed.

