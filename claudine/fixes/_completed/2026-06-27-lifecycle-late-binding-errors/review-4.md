---
ready: false
agent: codex/default
created: 2026-06-28T11:13:31
implemented: true
---

# Review: Lifecycle Late Binding Errors

## Verdict

Not ready for production.

The review-3 catch-path failures are mostly addressed. The implementation now propagates catch `failure` / `finalize` evaluation errors for non-loop initialize, loop initialize, target initialize, start `routes_to_failure`, pre-flight blocked, and several terminal-control abort paths. The focused L1 tests for those paths pass.

There are still reachable harness-loop branches that call lifecycle events directly and discard `evaluation_error`. That leaves the spec's "any lifecycle event" guarantee false.

## Findings

### High: Pre-start harness blocked/finalize paths still swallow evaluation errors

Two pre-start setup-failure branches in `run_harness_loop` still call `run_lifecycle_event(Blocked)` and `run_lifecycle_event(Finalize)` directly, then return the original setup error:

- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1709`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1720`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1794`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1805`

These are the missing-source branch and the passthrough-mode shell-audit failure branch. If a user-authored `blocked.when` or `finalize.when` raises under DM2 strict mode while handling either failure, the raised lifecycle evaluation error is discarded and the caller sees only `source file does not exist` or `shell audit failed`.

Concrete failing shape for either branch:

```yaml
blocked:
  stack:
    - when: "missing_root == true"
      action: { stderr: "unreached" }
finalize:
  stack:
    - when: "err"
      action: { stderr: "cleanup {{err.variant}}" }
```

Required fix: route these branches through `emit_blocked_finalize_with_err`, like materialization, target-lifecycle parse, and harness-plan parse failures already do. If `blocked` raises, run catch `failure` and `finalize` with the evaluation error as `err`; if `finalize` raises, surface `LifecycleEvaluationError { event: "finalize", ... }` without re-entry.

Verification needed: L1 harness tests for missing-source and passthrough shell-audit failures where `blocked.when` raises, plus a finalize-raise case that proves no recursive finalize re-entry and that the surfaced event is `finalize`.

### High: Interrupt and start-control abort finalization still drop lifecycle evaluation errors

There are still direct terminal/catch calls that ignore evaluation errors:

- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1908` runs `finalize` after a `start` control dispatch abort and discards a possible `finalize.evaluation_error`.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:2169` runs `failure` for an interrupted provider run and discards a possible `failure.evaluation_error`.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:2180` then runs `finalize` for the same interrupt path and discards a possible `finalize.evaluation_error`.

The interrupt branch is especially visible: a user can press Ctrl+C, Claudine can emit `failure`, the user-authored `failure.when` can crash, and the function still returns the child exit code path instead of surfacing the lifecycle evaluation error. That violates the spec requirement that late-binding evaluation errors in any event emit user-facing stderr and become the recorded run outcome.

Required fix: use the same outcome handling as the provider-failure branch: after the interrupt `Failure`, call `handle_terminal_evaluation_error`; after the interrupt `Finalize`, return `CompositionError::lifecycle_evaluation("finalize", ...)` if it raised. For the `start` control abort path, inspect the `finalize_outcome` before returning the original abort.

Verification needed: L1 orchestration tests for interrupted-run `failure.when` raise, interrupted-run `finalize.when` raise, and `start` control abort plus `finalize.when` raise.

## Test Rigor

Requirement-to-level check:

- Evaluation errors are distinct from dispatch failures: L1 unit/orchestration tests present.
- `no_error` does not suppress evaluation raises: L1 unit test present.
- Terminal `success` evaluation error runs `finalize` with `err` and returns non-zero: L1 orchestration tests present.
- Pre-flight `blocked` and harness setup helper evaluation errors are propagated: L1 helper tests present.
- Nested catch-path raises after an initial evaluation error: L1 tests present.
- Catch-path raises after explicit initialize/start `error(...)` and action-error routing: L1 tests present for the main non-loop, loop, target initialize, and start routes.
- Remaining missing-source, passthrough shell-audit, interrupt, and start-control-abort paths: missing L1 coverage and currently broken.

No Level 2 or Level 3 tests are required for this spec. The behavior is lifecycle control flow, stderr rendering, and process exit status; it does not depend on terminal-emulator rendering, terminal input encoding, or OS keyboard injection.

## Verification Run

I ran:

```text
just test-cli emit_preflight_blocked_and_finalize_surfaces_blocked_evaluation_error emit_preflight_blocked_and_finalize_surfaces_finalize_evaluation_error_without_reentry compose_initialize_error_with_failure_raise_surfaces_failure_evaluation_error loop_initialize_error_with_failure_raise_surfaces_failure_evaluation_error target_initialize_error_with_failure_raise_surfaces_failure_evaluation_error terminal_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error
```

Result: 5 passed, 1929 skipped. The loop-engine test name does not belong to the CLI package and was skipped by that invocation.

I then ran:

```text
just test-library loop_initialize_error_with_failure_raise_surfaces_failure_evaluation_error loop_initialize_error_with_failure_and_finalize_raise_surfaces_finalize loop_gate_evaluation_error_fails_before_condition_and_mutation
```

Result: 3 passed, 3009 skipped.
