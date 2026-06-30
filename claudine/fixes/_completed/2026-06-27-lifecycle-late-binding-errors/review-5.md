---
ready: false
agent: codex/default
created: 2026-06-28T11:39:45
implemented: true
---

# Review 5: Lifecycle Late Binding Errors

## Findings

### High: `loop` evaluation errors do not fire `finalize` with `err`

The spec requires late-binding evaluation errors in any terminal-phase event, including `loop`, to surface, halt, and fire `finalize` with the evaluation error exposed as the `err` global (`spec.md:70`, `spec.md:133`). The implementation handles `loop` differently: `run_loop_gate` converts `loop_outcome.evaluation_error` directly into `LoopGateOutcome::Fail(CompositionError::lifecycle_evaluation("loop", ...))` and returns before running any catch/finalize path (`claudine/lib/src/composition/loop_engine.rs:1152`).

That means authors cannot react to a crashed `loop.when` or loop action interpolation from `finalize`, even though the behavior matrix explicitly includes `loop`.

Verification level: current coverage is Level 1 only, and it asserts the direct failure path before condition/mutation. It does not assert the required `finalize` catch behavior for `loop`, so the test currently locks in the missing behavior rather than the spec.

Suggested fix: route a `loop` evaluation error through the same terminal evaluation catch semantics used by success/failure: run `finalize` once with the `loop` evaluation error as `err`, then surface `catch_evaluation_error(...)` with `finalize` precedence. Add an L1 loop-engine test proving `finalize.stack` sees `err.variant`/`err.msg` from the loop evaluation error.

### Medium: evaluation errors are not emitted before further lifecycle events

Decision #2 says the evaluation error should be rendered to stderr immediately, before any further events fire (`spec.md:98`). The terminal handler currently detects `outcome.evaluation_error`, runs `finalize`, and only then returns the `CompositionError` for the outer CLI error renderer to print (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1261`). The setup/preflight paths follow the same pattern: they run catch events first and surface the typed error afterward.

The user does eventually get a styled error and a non-zero result, but the ordering is not the one specified. If `finalize` is slow, noisy, or itself performs blocking side effects, the original lifecycle crash is not visible at the point where it happened.

Verification level: Level 1 tests assert the final rendered error exists, but there is no test for stderr ordering relative to `finalize` output. Because this is user-observable stderr ordering, L1 process-level capture is sufficient; no Level 2/3 terminal harness is required.

Suggested fix: either emit the styled lifecycle evaluation block at the catch point before running catch events and suppress duplicate outer rendering, or update the spec if delayed render-after-finalize is now intentional. If keeping the spec as written, add an L1 process or capture test that proves the lifecycle evaluation error appears before any `finalize` stderr/status output.

## Coverage Assessment

- `initialize` evaluation errors: Level 1 process coverage exists for non-zero exit and stderr surfacing.
- `success` evaluation errors: Level 1 orchestration coverage exists for finalize-with-err and non-zero propagation, but not a full process-level success-path exit/stderr test.
- `failure` catch evaluation errors: Level 1 orchestration and setup process coverage exist.
- `finalize` evaluation errors: Level 1 orchestration/rendering coverage exists for no re-entry and styled surfacing.
- `loop` evaluation errors: Level 1 coverage exists for halting before condition/mutation, but misses the spec-required finalize catch.
- Terminal rendering/keyboard behavior: no Level 2/3 coverage is needed for this spec; the behavior is process exit, stderr text, and lifecycle control flow, not terminal emulator encoding or OS keyboard input.

## Summary

The implementation makes the right core architectural move by separating expression-layer `evaluation_error` from side-effect `action_error`, and the targeted tests I ran passed:

```text
cargo nextest run -p claudine-cli -E 'test(compose_initialize_when_evaluation_error_exits_non_zero) | test(compose_initialize_error_with_failure_raise_surfaces_failure_evaluation_error) | test(success_evaluation_error_runs_finalize_with_err_and_returns_failure) | test(success_when_evaluation_error_is_not_swallowed_as_action_error) | test(dispatch_failure_on_terminal_event_is_not_escalated) | test(finalize_evaluation_error_aborts_without_reentry) | test(emit_preflight_blocked_and_finalize_surfaces_blocked_evaluation_error)' --no-fail-fast
```

Result: 6 tests run, 6 passed.

However, the feature is not production ready yet because the `loop` event does not satisfy the required finalize catch behavior, and the stderr emission ordering still diverges from Decision #2.
