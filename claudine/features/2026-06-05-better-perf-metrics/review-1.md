---
ready: false
agent: codex
model: ""
---

# Review: Better `--perf` Metrics

## Findings

### High: Sequence reports do not implement the required `steps` structural subtree

Spec TM-3 requires `sequence` to add a `steps` Structural node whose children are per-step subtrees, with per-step wall clocks reconciling to the sequence headline plus orchestration / unattributed time. The implementation still collapses all step data into one merged `composition` and one aggregate `agent` value:

- `SequencePerfAccumulator` stores `steps`, but `into_report_with_elapsed` only merges `compose_perf` and aggregates agent totals; it never preserves or renders per-step nodes ([claudine/cli/src/perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:589)).
- `build_perf_tree` attaches sequence composition as a single `Breakdown` child under `environment setup`, not under a `steps` node ([claudine/cli/src/perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:290)).
- The integration test currently locks in the opposite behavior by asserting there is at most one composition node and explicitly saying there are no per-step composition subtrees ([claudine/cli/tests/sequence_perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/sequence_perf.rs:84)).

This means users cannot see which sequence step consumed time, and sequence orchestration cannot be distinguished from step work as designed. Verification level present: Level 1 process/integration. Required before ready: Level 1 tree-model tests and process tests that assert `steps -> step N -> composition/agent` shape, plus reconciliation across completed and partial sequences.

### High: Terminal-rendering requirements only have Level 1 verification

The spec has user-observable terminal rendering requirements: box connectors, unit-boundary alignment, percent column, yellow block-quote frame / markup rendering, and the single `HOT` marker (P-1 through P-4, G-8). Current tests are all in-process or assert-command captures with ANSI stripping:

- `MetricsTree` connector, alignment, percent, highlight, and ASCII fallback tests call `render_optimistic` / synthetic `Terminal` directly ([biscuit-terminal/lib/src/components/metrics_tree.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/metrics_tree.rs:526)).
- claudine CLI tests inspect captured stderr after stripping ANSI and only check substrings for the report surface ([claudine/cli/tests/wrap_commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/wrap_commands.rs:6012)).
- No `level2_` perf / metrics-tree test exists in `claudine/cli/tests` or biscuit-terminal.

Under the requested rigor rubric, real terminal rendering of glyphs, widths, and SGR styling needs Level 2 coverage. The strongest verification present is Level 1, so these visual requirements are not production-ready. Add a Level 2 tmux / WezTerm capture that runs a representative `claudine compose --perf --dry-run`, captures pane text, and asserts connector hierarchy, aligned duration units, visible percent column, HOT marker, dry-run placeholder, and no stdout pollution.

### High: `context capture` is attached under `composition` even though it is outside the measured compose window

The spec's OQ-3 recommendation is timeline-based attachment: context-capture timings belong under `composition` only if capture happened inside `compose_perf.total`; otherwise they belong under `prep phase` as a Structural child. The implementation always copies `options.context().capture_timings()` into `ComposePerfReport` at the end of `run_compose_pipeline_internal` ([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/mod.rs:849)) and claudine always renders those timings under `composition` ([claudine/cli/src/perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:386)).

But the context is commonly captured before the compose pipeline's perf collector starts. Darkmatter CLI captures it before constructing `ComposeOptions` ([darkmatter/cli/src/commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/cli/src/commands.rs:554)), and claudine does the same in composition prep ([claudine/cli/src/commands/compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/compose.rs:475)). That makes the report say the time happened inside composition when it did not. Because the feature's core contract is one truthful timeline, this should be fixed before readiness: either measure capture inside the reported compose total, or attach it to the claudine prep window when it was pre-captured.

### Medium: `ShellCommandSpan` exposes fields that are always wrong or empty

DM-3 specifies per-directive spans carrying `{ command_display, command_hash, elapsed, cached, exit_status }`. The implementation creates the public fields, but `run_shell_expansion_stage` always records `cached: false` and `exit_status: None`, even when a directive was cached or a process exit status is known ([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/mod.rs:1347)). The inline comment calls fuller population a follow-up, but the public type docs describe these values as meaningful ([darkmatter/lib/src/markdown/compose/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/types.rs:1589)).

Either complete the executor API widening now or omit / clearly mark those fields as unavailable until they are accurate. As written, downstream consumers can make incorrect decisions from the new API.

## Verification Assessment

| Requirement | Strongest current verification | Required level | Status |
| --- | --- | --- | --- |
| True wall-clock headline and top-level reconciliation | Level 1 unit tests including motivating dry-run shape ([claudine/cli/src/perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:2022)) | Level 1 | OK |
| Tree roles and no sibling double-counting for compose | Level 1 unit tests ([claudine/cli/src/perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:1735)) | Level 1 | OK |
| Sequence per-step tree | Level 1 tests assert aggregate-only behavior ([claudine/cli/tests/sequence_perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/sequence_perf.rs:84)) | Level 1 | Gap |
| Glyphs, alignment, styling, HOT marker in real terminal | Level 1 render/string tests only ([biscuit-terminal/lib/src/components/metrics_tree.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/metrics_tree.rs:526)) | Level 2 | Gap |
| `--perf` stderr-only contract | Level 1 assert-command stdout/stderr tests ([claudine/cli/tests/wrap_commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/wrap_commands.rs:6053)) | Level 1 | OK |
| Dry-run placeholder | Level 1 render and process tests ([claudine/cli/src/perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:1535)) | Level 1 for content; Level 2 for visual render | Partial |

## Readiness

Not ready for production. The headline/reconciliation work is materially improved, but sequence reporting misses a designed user-facing structure, the real-terminal rendering contract is under-verified for this repo's test rigor, and context-capture placement violates the single-timeline model when the context is pre-captured.
