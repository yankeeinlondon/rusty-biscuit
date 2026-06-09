---
ready: false
agent: codex
model: ""
---

# Review: Better `--perf` Metrics, Iteration 2

## Findings

### High: Sequence step subtrees nest composition under a step total that does not include composition

Spec G-2 requires the report to be a single timeline tree where every node is a fraction of its parent and children never exceed their parent. TM-3 also says sequence per-step wall-clocks are Structural children under `steps`, with per-step subtrees reusing the same timeline model.

The iteration now renders a `steps` node, but the step total is only the Phase 2 execution window. `execute_sequence` closes `environment setup` before the step loop, then starts `let start = Instant::now()` immediately before `execute_composition_request_inner` and stores `wall_clock: duration` after that call ([sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:457), [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:518), [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:580)). The step's `compose_perf` is produced earlier during sequence setup (`run_phase_1c_with_schema`) and is already inside the top-level `environment setup` window, not inside this `wall_clock`.

`build_steps_node` then renders that earlier composition work under each `step N` child while documenting that it was "metered during the shared environment-setup phase" and is "not part of its execution window" ([perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:357)). Because the composition node is a `Breakdown`, TR-1 still passes, but the user-facing tree can be false: a slow compose step followed by a dry-run or fast agent can show `step 1: alpha 5ms` with `composition 900ms` nested beneath it. That violates the parent/child timeline contract and makes sequence percentages misleading.

Fix before production by making each sequence step's Structural total cover the full per-step timeline that owns the nested children, or by moving setup-phase composition detail out from under `step N` into a node under `environment setup` such as `step preparation -> step N -> composition`. Whichever shape you choose needs a Level 1 model test that proves a slow per-step composition cannot exceed its displayed parent.

## Verification Assessment

| Requirement | Strongest current verification | Required level | Status |
| --- | --- | --- | --- |
| True wall-clock headline and top-level reconciliation | Level 1 unit tests, including the motivating dry-run contradiction shape ([perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:2154)) | Level 1 | OK |
| Tree roles and no sibling double-counting for single-shot compose / inline-compose | Level 1 unit tests and process tests | Level 1 | OK |
| Sequence per-step tree | Level 1 tests assert `steps -> step N`, but not that child timings fit the step parent | Level 1 | Gap |
| Glyphs, alignment, styling, HOT marker in a real terminal | Level 2 tmux and WezTerm capture tests ([level2_perf_capture.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_perf_capture.rs:189)) | Level 2 | OK |
| `--perf` stderr-only contract | Level 1 assert-command tests | Level 1 | OK |
| Dry-run placeholder rendering | Level 1 render/process tests plus Level 2 terminal capture | Level 2 for visual render | OK |
| Context capture placement for single-shot compose | Level 1 model test verifies prep placement, not composition placement ([perf.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/perf.rs:2066)) | Level 1 | OK |
| Per-shell directive spans and redaction | Level 1 darkmatter unit tests and claudine render test | Level 1 | OK |

## Verification Run

- `cargo test -p claudine-cli perf --color=never` passed, including `level2_perf_capture`.
- `cargo test -p biscuit-terminal metrics_tree --color=never` passed.
- `cargo test -p darkmatter redact_shell_command --color=never` passed.
- `cargo test -p darkmatter records_shell_spans_and_capture_timings --color=never` passed.

## Readiness

Not ready for production. The prior high-severity gaps were mostly addressed, including real-terminal Level 2 coverage, but sequence reports still violate the core timeline-tree contract for per-step composition work.
