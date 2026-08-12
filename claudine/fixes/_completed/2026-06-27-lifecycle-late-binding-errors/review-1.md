---
ready: false
agent: codex/default
created: 2026-06-28T08:29:10
implemented: true
---

# Review: Lifecycle Late Binding Errors

## Verdict

Not ready for production.

The core implementation moves in the right direction: late-binding expression failures now have a distinct `evaluation_error` channel, `no_error` no longer suppresses those failures, and the main `initialize`/`start`/`success`/`failure`/`finalize`/`loop` paths have focused L1 tests. However, there are still lifecycle event paths that drop `evaluation_error`, so the spec's "any lifecycle event" guarantee is not yet true.

## Findings

### High: Pre-flight `blocked` lifecycle evaluation errors are still dropped

The compose pre-flight helper runs `blocked` and then `finalize`, but it only returns `blocked_outcome.control` and ignores both events' `evaluation_error` fields:

- `claudine/cli/src/commands/wrap/composition/mod.rs:210`
- `claudine/cli/src/commands/wrap/composition/mod.rs:219`

This violates the spec's goals that a late-binding evaluation error in any lifecycle event emits user-facing stderr, halts as the recorded outcome, and lets `finalize` catch that evaluation error as `err`.

Concrete failing shape:

```yaml
blocked:
  stack:
    - when: "missing_root == true"
      action: { stderr: "unreached" }
finalize:
  stack:
    - when: "err"
      action: { stderr: "finalized {{err.variant}}" }
```

If a pre-flight shell approval or harness-plan parse fails, `blocked.when` raises under strict DM2. The current helper ignores that raise, runs `finalize` with the original pre-flight error, and returns the original pre-flight failure. The lifecycle expression crash is not surfaced as `CompositionError::LifecycleEvaluationError`.

Required fix: make this helper return a richer result than `Option<StackControl>` so callers can propagate `LifecycleEvaluationError`. If `blocked` raises, route through `failure`/`finalize` with the evaluation error as `err` and return non-zero with that typed error. If `finalize` raises, surface it without re-entering `finalize`.

Verification needed: L1 process-level test that triggers a compose pre-flight `blocked` path and makes `blocked.when` raise, asserting stderr contains `lifecycle evaluation error`, the event is `blocked`, and the exit is non-zero.

### High: Harness setup-failure helper also ignores lifecycle evaluation errors

`emit_blocked_finalize_with_err` has the same issue in the harness loop. It fires `Blocked` or `Failure`, then `Finalize`, but discards both `LifecycleEventOutcome`s:

- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:314`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:325`

This helper is used for materialization failures, target lifecycle parse failures, harness-plan parse failures, and other setup failures after the lifecycle guard exists. If the user-authored `blocked`/`failure`/`finalize` stack itself raises while reacting to those failures, the implementation hides the lifecycle evaluation error behind the original setup error.

Required fix: have this helper return `Result<(), Report>` or a typed outcome and handle `evaluation_error` the same way as the main setup and terminal helpers. Since this is used from `inspect_err` closures today, call sites may need to avoid `inspect_err` when lifecycle handling can itself fail and should replace the propagated error.

Verification needed: L1 orchestration test for a setup failure routed through this helper with a raising `blocked.when` pre-launch and a raising `failure.when` post-launch. Add a `finalize` raise case to ensure no recursive finalize re-entry.

### Medium: Downgraded success/blocked failures can report the wrong event name

When a `success` or `blocked` stack downgrades via explicit `error`, `execute_terminal_event` runs the `failure` event and returns that `failure_outcome`:

- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:121`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:131`

The success caller then handles any returned `evaluation_error` as event `"success"`:

- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:2247`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:2249`

If the downgraded `failure` stack raises, the run still halts, but the surfaced `CompositionError::LifecycleEvaluationError` names `success` instead of `failure`. That makes the diagnostic point at the wrong lifecycle event and undermines the spec's goal of actionable surfacing.

Required fix: carry the effective event that produced the returned outcome in `TerminalEventOutcome`, or handle the downgraded `failure` evaluation before returning from `execute_terminal_event`.

Verification needed: L1 orchestration test where `success.stack` explicitly errors and `failure.when` raises, asserting the typed error reports event `failure`.

## Test Rigor

Requirement-to-level check:

- Terminal evaluation classification (`when`/interpolation vs dispatch): L1 unit/orchestration tests present.
- `no_error` does not suppress evaluation raises: L1 unit test present.
- `success` evaluation error runs `finalize` with `err` and returns failure: L1 orchestration test present.
- `finalize` evaluation error does not re-enter `finalize`: L1 orchestration test present.
- User-facing stderr rendering for lifecycle evaluation errors: L1 render tests present.
- Non-interactive process exit and stderr: L1 process test present for `initialize`, but not for the original terminal-phase `success` regression.
- Pre-flight `blocked` and harness setup-failure lifecycle raises: missing L1 coverage and currently broken.

No Level 2 or Level 3 tests are required for this spec because the behavior is process stderr/exit and lifecycle control flow, not real terminal rendering, terminal input encoding, or OS keyboard behavior.

## Verification Run

I ran these targeted L1 checks:

- `just test-library lifecycle_evaluation_error no_error_does_not_suppress_evaluation_raise loop_gate_evaluation_error_fails_before_condition_and_mutation`
  - 2 passed, 3008 skipped.
- `just test-cli success_evaluation_error finalize_evaluation_error compose_initialize_when_evaluation_error_exits_non_zero renders_lifecycle_evaluation_error`
  - 5 passed, 1908 skipped.

These passing tests cover the newly added mainline paths, but they do not cover the blocked/setup-failure gaps above.
