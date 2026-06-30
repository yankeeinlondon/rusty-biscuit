---
ready: false
agent: codex/default
created: 2026-06-28T09:33:58
implemented: true
---

# Review: Lifecycle Late Binding Errors

## Verdict

Not ready for production.

The second iteration addresses the review-1 high-severity gaps for the pre-flight `blocked` path and the harness setup-failure helpers. The new outcome plumbing, event-name correction for success-to-failure downgrades, and focused L1 tests are meaningful progress.

One important gap remains: when the implementation runs `failure` or `finalize` as the catch path for an earlier lifecycle evaluation error, it still discards evaluation errors raised by that catch path. That violates the spec's "any lifecycle event" guarantee and can still make the user see the wrong lifecycle error.

## Findings

### High: Catch-path `failure` / `finalize` evaluation errors are still swallowed

Several paths correctly detect an initial lifecycle evaluation error, then run `failure` and/or `finalize` with that error exposed as `err`. But they ignore the `evaluation_error` returned by those catch events and immediately return the original error:

- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1266` runs `finalize` from `handle_terminal_evaluation_error` and ignores the finalize outcome.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1300` and `:1311` run `failure` / `finalize` from `handle_setup_evaluation_error` and ignore both outcomes.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:394` and `:476` run `finalize` after a `blocked` / `failure` evaluation error and ignore its outcome.
- `claudine/cli/src/commands/wrap/composition/mod.rs:255` has the same issue for the compose pre-flight `blocked` helper.
- `claudine/cli/src/commands/wrap/composition/mod.rs:1759` and `claudine/lib/src/composition/loop_engine.rs:671` route `initialize` evaluation errors through `failure` / `finalize` but ignore evaluation errors from those events.

Concrete failing shape:

```yaml
success:
  stack:
    - when: "missing_root == true"
      action: {stderr: "unreached"}
finalize:
  stack:
    - when: "also_missing == true"
      action: {stderr: "unreached"}
```

The `success.when` raise is detected and `finalize` runs, but if `finalize.when` also raises, the helper still returns the `success` `LifecycleEvaluationError`. The `finalize` crash is not surfaced as the halt cause even though the spec explicitly requires an evaluation error in `finalize` itself to surface without recursive re-entry.

The same pattern applies to setup paths:

```yaml
initialize:
  stack:
    - when: "missing_root == true"
      action: {stderr: "unreached"}
failure:
  stack:
    - when: "failure_typo == true"
      action: {stderr: "unreached"}
finalize:
  stack:
    - when: "finalize_typo == true"
      action: {stderr: "unreached"}
```

`initialize` raises, but a later `failure` or `finalize` expression crash can be hidden behind the original `initialize` error. This is still a lifecycle evaluation error in a user-authored event, so it must not be discarded.

Required fix: centralize "run catch event and prefer its evaluation error" behavior. When a catch `failure` raises, return `LifecycleEvaluationError { event: "failure", ... }` and then run `finalize` with that failure error as `err`; if that `finalize` raises, prefer `event: "finalize"`. When a catch `finalize` raises, return the finalize error without re-entering finalize. Apply the same helper to harness, compose pre-flight, non-loop initialize, and loop initialize paths.

Verification needed: L1 tests for nested raises:

- `success.when` raises, then `finalize.when` raises: returned/rendered error names `finalize`.
- `start` or `initialize` raises, then `failure.when` raises: returned/rendered error names `failure`, and `finalize` receives the failure evaluation error.
- `blocked.when` raises, then catch `finalize.when` raises in both pre-flight and harness setup helpers: returned/rendered error names `finalize`.

## Test Rigor

Requirement-to-level check:

- Evaluation errors are distinct from dispatch failures: L1 unit/orchestration tests present.
- `no_error` does not suppress evaluation raises: L1 unit test present.
- Terminal `success` evaluation error runs `finalize` with `err` and returns non-zero: L1 orchestration tests present.
- `blocked` pre-flight and harness setup evaluation errors are propagated: L1 helper tests present.
- Success-to-failure downgrade reports the effective `failure` event: L1 orchestration test present.
- User-facing stderr / non-interactive exit for setup `initialize`: L1 process test present.
- Nested catch-path raises in `failure` / `finalize`: missing L1 coverage and currently broken.

No Level 2 or Level 3 tests are required for this spec. The behavior under review is lifecycle control flow, stderr rendering, and process exit status; it does not depend on real terminal rendering, terminal input encoding, or OS keyboard injection.

## Verification Run

I ran:

```text
just test-cli emit_blocked_finalize_pre_launch_blocked_raise_surfaces_failure_and_finalize emit_blocked_finalize_post_launch_failure_raise_surfaces_finalize emit_blocked_finalize_finalize_raise_surfaces_without_reentry emit_failure_finalize_failure_raise_surfaces_finalize emit_failure_finalize_finalize_raise_surfaces_without_reentry compose_initialize_when_evaluation_error_exits_non_zero
```

Result: 6 passed, 1918 skipped.
