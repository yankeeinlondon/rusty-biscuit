---
ready: false
implemented: true
agent: codex/default
created: 2026-06-22T10:10:54
---

# Review 2

Production ready: **false**.

## Findings

### High: `success` and `blocked` still violate the top-level-before-stack contract

The spec says adding `stack:` is additive and that top-level communication properties fire before the stack for every event (`spec.md:642`, `spec.md:657`). The implementation now deliberately special-cases `success` and `blocked` in the opposite order: it runs the stack first with `run_lifecycle_stack_only`, then emits top-level communication only afterward, and skips the original top-level communication entirely when the stack returns `error(...)`.

- Evidence: `execute_terminal_event` documents and implements stack-before-top-level for `Success` / `Blocked` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:29-35` and `:50-82`.
- The error branch returns the `failure` event without emitting the original event's top-level communication at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:63-78`.

This fixes the double-stack execution from review 1, but it does so by changing an accepted user-visible ordering rule. A document with `success.info` plus `success.stack` side effects will now record or display the stack effect before the top-level status. A document with `success.info` plus `success.stack: error('bad')` will never emit the `success.info` at all, even though the spec says top-level properties fire before stack processing.

Verification level: the strongest relevant tests are L2 dispatch tests for event order, but they only put markers in `stack:` blocks. They do not cover top-level-vs-stack ordering or the `success.stack error(...)` downgrade path. Required coverage is L2 because the behavior is user-observable terminal output and externally observable ordered side effects.

### High: `initialize`-time `proxy(...)` does not run the target document's `initialize`

The spec states that `Proxy` hands off to another prompt and that the proxied document enters at its own `initialize`, including respecting target-side `Skip` / `Proxy` / `Error` logic (`spec.md:340`). The initialize-time proxy path swaps the source path and asks the harness loop to reparse the target lifecycle, but it does not reset the already-emitted initialize state. The code comment explicitly says the target's `initialize` stack does not re-run.

- Evidence: initialize proxy resolves the target and passes it into `run_body` at `claudine/cli/src/commands/wrap/composition/mod.rs:1498-1512`; the comment says the target's `initialize` stack does not re-run at `:1501-1505`.
- `run_body` swaps `source_path` and passes `initial_proxy_target` to the harness loop at `claudine/cli/src/commands/wrap/composition/mod.rs:1282-1345`.
- The harness loop reparses the target lifecycle when `proxy_tracking.pending` is set, but only calls `set_config`; it does not call `reset_for_proxy` or emit target `initialize` before proceeding to plan parsing/start at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:603-619`.

This means a target prompt proxied from `initialize` cannot use its own `initialize.stack` for setup, skip, error, or proxy decisions. That is a functional gap, not just missing coverage.

Verification level: I found L2 proxy coverage for `failure.stack -> proxy(...)`, but no L2 coverage for `initialize.stack -> proxy(...)` and no assertion that the target's `initialize` fires. Required coverage is L2 because proxy handoff is an end-to-end lifecycle control flow.

### High: `requeue(...)` is still unsupported at runtime

The spec defines `Requeue` as a real lifecycle action that pushes the prompt onto the deferred-execution queue via `rendezvous` (`spec.md:343`). The runtime instead returns a typed unsupported error whenever `ControlDispatch::Requeue` is reached.

- Evidence: `dispatch_terminal_control` says there is no runtime queue integration and returns `CompositionError::LifecycleRequeueUnsupported` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:486-496`.
- The new L2 test encodes this unsupported behavior as expected: `level2_lifecycle_failure_requeue_surfaces_typed_unsupported_error` at `claudine/cli/tests/level2_lifecycle_control.rs:275-312`.

Surfacing an explicit unsupported error is better than silently dropping the action, but it does not implement the accepted feature. Either the spec must be reduced to say `requeue` is intentionally unsupported in this release, or the runtime needs real queue integration and tests that prove the queued prompt is recorded.

Verification level: current L2 coverage verifies the error surface, not the specified requeue behavior. The requirement's strongest test is therefore at the wrong semantic level: it exercises runtime visibility but not the required behavior.

### High: L2 resume coverage only verifies the error branch, not successful resume control flow

The test strategy requires L2 coverage for `Proxy` / `Retry` / `Resume` / `Requeue` control flow. The implementation has a unit-level dispatch test for a session-backed resume path, but the L2 test uses a fake provider that never reports a session id and asserts `LifecycleResumeWithoutSession`.

- Evidence: the L2 test documents that resume coverage is "without a session id" at `claudine/cli/tests/level2_lifecycle_control.rs:18-22`, then asserts no provider re-invocation at `:232-272`.
- The successful dispatch branch only seeds `next_resume_session_id` / `next_prompt_override` and continues at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:397-422`.

That leaves the user-facing behavior of `resume("...")` unproven in the real CLI path: provider resume capability validation, session id propagation, follow-up prompt delivery, and second-attempt lifecycle ordering. Under the requested rigor rules, this is still a coverage gap for a user-observable keybinding-like control action.

Verification level: successful resume has L1-style branch coverage, but the requested requirement needs L2 end-to-end coverage.

## Notes

The previous critical double-execution issue appears addressed for side-effect count: `success` / `blocked` stacks are no longer run twice. The new L2 dispatch and loop tests also materially improve coverage for ordinary success/failure/blocked order, loop gate ordering, and blocked-first-iteration finalize behavior. The remaining blockers are spec-level behavioral mismatches and control-flow coverage gaps, so I do not consider the feature production-ready.
