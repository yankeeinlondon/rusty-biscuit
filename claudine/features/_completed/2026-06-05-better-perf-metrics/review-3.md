---
ready: true
agent: codex
model: ""
---

# Review: Better `--perf` Metrics, Iteration 3

## Findings

No blocking findings.

The iteration-2 sequence timing issue is addressed. Per-step composition is no longer nested under the per-step execution window; it now renders under `environment setup → step preparation`, while `steps → step N` carries only the step execution wall-clock and agent breakdown. That keeps the user-facing tree truthful when composition is slow and execution is fast.

## Verification Assessment

| Requirement | Strongest current verification | Required level | Status |
| --- | --- | --- | --- |
| True wall-clock headline from the threaded `process_start` baseline | Level 1 unit/process tests; all emit sites route through `emit_report(&collector.into_report())` / `emit_report(&acc.into_report())` | Level 1 | OK |
| Top-level and nested Structural reconciliation with explicit `unattributed` remainder | Level 1 tree-model tests plus debug-build reconciliation assertion | Level 1 | OK |
| Compose/inline-compose report avoids sibling double-counting by nesting `composition` under `prep phase` | Level 1 tree-model and CLI process tests | Level 1 | OK |
| Sequence per-step timing tree does not place setup-phase composition under execution-phase step totals | Level 1 tree-model regression (`perf_tree_sequence_slow_compose_never_exceeds_parent`) and process test (`sequence_perf_renders_single_aggregated_report`) | Level 1 | OK |
| Glyphs, alignment, styling, percent column, and `HOT` marker render in a real terminal | Level 2 tmux and WezTerm captures in `level2_perf_capture.rs` | Level 2 | OK |
| `--perf` remains stderr-only; stdout is unchanged for compose/inline-compose/provider output | Level 1 assert-command tests in `wrap_commands.rs` | Level 1 | OK |
| Darkmatter shell spans are redacted and exposed without misleading unavailable fields | Level 1 darkmatter unit tests for span collection and redaction | Level 1 | OK |

## Verification Run

- `cargo test -p claudine-cli sequence_perf --color=never` passed.
- `cargo test -p claudine-cli perf --color=never` passed, including `level2_perf_capture` tmux and WezTerm tests on this host.
- `cargo test -p biscuit-terminal metrics_tree --color=never` passed.
- `cargo test -p darkmatter records_shell_spans_and_capture_timings --color=never` passed.
- `cargo test -p darkmatter redact_shell_command --color=never` passed.

## Readiness

Ready for production. The prior high-severity gaps are closed, and the user-observable terminal rendering requirements have the requested Level 2 coverage.
