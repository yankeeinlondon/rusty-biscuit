---
ready: false
agent: codex/default
created: 2026-06-28T10:04:53
implemented: true
---

# Review: Lifecycle Late Binding Errors

## Verdict

Not ready for production.

The review-2 findings are addressed for the evaluation-error-triggered catch paths: terminal `success`/`blocked`, setup `initialize`/`start`, pre-flight `blocked`, and harness setup helpers now prefer evaluation errors raised by catch `failure`/`finalize`, and the focused L1 tests pass.

One adjacent class remains broken. Older catch paths that are entered because of explicit lifecycle control (`error`) or a dispatch/action failure still run `failure` and/or `finalize` but discard any evaluation error those events raise. That still violates the spec's requirement that a late-binding evaluation error in any lifecycle event surfaces and halts.

## Findings

### High: `failure` / `finalize` evaluation errors are still swallowed on non-evaluation catch paths

Several routes manually run `failure` and `finalize` after an explicit lifecycle `error(...)`, an initialize/start action error, or a terminal control abort, but they ignore the returned `LifecycleEventOutcome` values:

- `claudine/cli/src/commands/wrap/composition/mod.rs:1819` and `:1823` run `failure` / `finalize` after `initialize` returns explicit `error`, then return the original `eyre!(msg)`.
- `claudine/cli/src/commands/wrap/composition/mod.rs:1889` and `:1890` do the same for `initialize` `routes_to_failure`.
- `claudine/lib/src/composition/loop_engine.rs:1053` and `:1057` do the same in `route_init_failure`, used by loop-mode initialize `error`, proxy-resolution failure, and initialize action-error routing.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:691` / `:702` and `:739` / `:750` do the same for target `initialize`.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1883` / `:1894` and `:1980` / `:1982` do the same for `start`.
- The terminal-control abort branches at `loop_control.rs:2257`, `:2374`, and `:2473` run `finalize` and discard a possible `finalize.evaluation_error`.

Concrete failing shape:

```yaml
initialize:
  stack:
    - action: { error: "stop before launch" }
failure:
  stack:
    - when: "failure_typo == true"
      action: { stderr: "unreached" }
finalize:
  stack:
    - when: "err"
      action: { stderr: "cleanup" }
```

The `failure.when` expression raises under strict DM2, but the non-loop path returns only `stop before launch`; loop mode returns `LifecycleInitializeFailed`; target initialize/start paths return their original error. The user-authored `failure` lifecycle evaluation crash is not surfaced as `LifecycleEvaluationError`.

Another failing shape:

```yaml
initialize:
  stack:
    - action: { error: "stop before launch" }
failure:
  stack:
    - when: "err"
      action: { stderr: "failure observed" }
finalize:
  stack:
    - when: "finalize_typo == true"
      action: { stderr: "unreached" }
```

Here `finalize` raises, but these paths still return the original initialize/start/control error. That directly contradicts the spec's explicit requirement that an evaluation error in `finalize` itself surfaces without recursive re-entry.

Required fix: factor the catch-event handling into a shared helper for all lifecycle catch paths, not only paths whose original trigger was already an evaluation error. The helper should run `failure` with the active `err`, thread a `failure.evaluation_error` into `finalize` if one occurs, and return `CompositionError::catch_evaluation_error(...)` when either catch event raises. Use it for non-loop initialize, loop initialize, target initialize, start, and terminal-control abort finalization.

Verification needed:

- L1 non-loop process test: `initialize.error` + raising `failure.when` exits non-zero and stderr names `lifecycle evaluation error` / `failure`.
- L1 loop-engine test: `initialize.error` or unresolved initialize proxy + raising `failure.when` returns `LifecycleEvaluationError` for `failure`, not `LifecycleInitializeFailed`.
- L1 harness orchestration tests for target initialize/start `error` and `routes_to_failure` with raising `failure.when`.
- L1 terminal-control abort test where `finalize.when` raises and the surfaced error names `finalize`.

## Test Rigor

Requirement-to-level check:

- Evaluation errors are distinct from dispatch failures: L1 unit/orchestration tests present.
- `no_error` does not suppress evaluation raises: L1 unit test present.
- Terminal `success` evaluation error runs `finalize` with `err` and returns non-zero: L1 orchestration tests present.
- Pre-flight `blocked` and harness setup evaluation errors are propagated: L1 helper tests present.
- Nested catch-path raises after an initial evaluation error: L1 tests present and passing.
- User-facing stderr / non-interactive exit for setup `initialize`: L1 process test present for direct initialize evaluation raise.
- Catch-path raises after explicit `error(...)`, action-error routing, or terminal-control abort: missing L1 coverage and currently broken.

No Level 2 or Level 3 tests are required for this spec. The behavior is lifecycle control flow, stderr rendering, and process exit status; it does not depend on terminal-emulator rendering, terminal input encoding, or OS keyboard injection.

## Verification Run

I ran:

```text
just test-cli emit_preflight_blocked_and_finalize_blocked_raise_then_finalize_raise_surfaces_finalize emit_preflight_blocked_and_finalize_surfaces_finalize_evaluation_error_without_reentry emit_failure_finalize_both_raise_surfaces_finalize success_raise_then_finalize_raise_surfaces_finalize setup_raise_then_failure_raise_surfaces_failure_and_threads_into_finalize compose_initialize_when_evaluation_error_exits_non_zero
```

Result: 6 passed, 1923 skipped.
