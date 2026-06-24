---
ready: false
agent: codex/default
created: 2026-06-22T13:24:08
implemented: true
---

# Review 4

Production ready: **false**.

## Findings

### Critical: `err` is not populated for normal `blocked`, `failure`, or failed `finalize` flows

The spec makes `err` the primary runtime error object for `blocked` and `failure`, and says `finalize` receives it when the iteration ended in failure (`spec.md:92-106`, `spec.md:473-485`). The implementation statically validates where `err` may appear, but many real runtime paths execute those events with `err: None`.

Evidence:

- Pre-check blocked path calls `execute_terminal_event(LifecycleSignal::Blocked, ..., None, ...)` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1399-1408`.
- Provider failure calls `execute_terminal_event(LifecycleSignal::Failure, ..., None, ...)` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1744-1753`.
- Inline-closure failure does the same at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1865-1874`.
- Post-check failure does the same at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:2015-2025`.
- The matching `finalize` calls also pass `None`, for example `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:2047-2056` and `:2062-2071`.

A valid document like this cannot observe the failure cause:

```yaml
failure:
  stack:
    - action: "append_line('events.log', err.kind)"
finalize:
  stack:
    - when: "err"
      action: "append_line('events.log', err.msg)"
```

For a provider non-zero exit, pre-check block, inline closure failure, or post-check failure, `err.kind`/`err.msg` are absent even though the event is explicitly error-bearing. This is a functionality gap, not just a test gap.

Verification level: parser/unit tests cover static `err` placement, but the user-observable requirement is event-time behavior. It needs at least Level 2 end-to-end coverage proving `err.*` renders through `failure`, `blocked`, and failed `finalize` stacks in a real terminal run. Current L2 tests assert event order and control flow, not the `err` payload.

### High: loop-mode `initialize` control actions are ignored

The spec says `initialize` can alter flow with `Skip`, `Proxy`, `Error`, or `Stop`, and looping documents still run `initialize` once before later iterations re-enter at `start` (`spec.md:312-322`, `spec.md:508-511`). The loop driver emits `initialize` but discards the returned `LifecycleEventOutcome`:

- `guard.execute_event(LifecycleSignal::Initialize, &init_ctx);` at `claudine/lib/src/composition/loop_engine.rs:628-641`.

Because the outcome is ignored, a looping document with `initialize.stack: [skip()]` still enters iteration 1 and invokes the provider; `error(...)` does not route to `failure`/`finalize`; and `proxy(...)` is never honored. The non-loop path handles these controls in `execute_composition_attempt`, but `compose --loop` goes through `execute_loop_with_lifecycle`, so the behavior diverges by mode.

Verification level: existing L2 initialize-proxy tests are in the non-loop control suite. The loop L2 suite proves initialize fires exactly once, but does not exercise `skip`, `error`, or `proxy` from `initialize` in a loop document.

### High: loop-gate explicit `error(...)` is ignored

The spec explicitly says `error(...)` is valid in every event, and in the `loop` gate it converts the final outcome to failure and exits the loop (`spec.md:334-341`, `spec.md:571-590`). The loop gate currently executes the event and ignores its outcome:

- `guard.execute_event(LifecycleSignal::Loop, &loop_ctx);` at `claudine/lib/src/composition/loop_engine.rs:861-873`.
- It then evaluates the loop condition and may continue or exit successfully at `claudine/lib/src/composition/loop_engine.rs:875-881`.

That means:

```yaml
loop:
  until: "phase > 1"
  stack:
    - action: "error('gate rejected final state')"
```

does not fail the composition as specified. The same missing outcome handling also means loop-gate action errors are not even deliberately classified, although terminal-phase unintentional errors may be allowed to leave the outcome unchanged.

Verification level: this is observable lifecycle control flow. It needs L1 coverage for the pure loop driver outcome and Level 2 coverage for `claudine compose --loop` proving the process exits/fails correctly and the terminal/file side effects match the gate control.

## Notes

Review-3 blockers around composition preflight dispatch and Unix requeue appear materially improved. New Level 2 coverage now exercises event order, loop gate ordering, initialize proxy handoff, retry/resume/requeue/proxy controls, and top-level-before-stack ordering.

The remaining blockers are core runtime semantics, not lack of test volume. The feature should not be considered production-ready until `err` is present in error-bearing events and loop-mode lifecycle controls are honored.
