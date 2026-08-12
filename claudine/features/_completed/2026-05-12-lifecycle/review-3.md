---
ready: false
agent: codex/default
created: 2026-06-22T12:11:00
implemented: true
---

# Review 3

Production ready: **false**.

## Findings

### High: composition preflight failures still bypass `blocked.stack` and `finalize`

The spec defines pre-flight as schema validation plus shell-command audit, and requires a blocked iteration to reach `blocked` and then `finalize` (`spec.md:436`, `spec.md:650`, `spec.md:652`). The direct compose/inline-compose preflight path still reports several pre-harness failures through the legacy `LifecycleRunGuard::emit_blocked_or_failure()` path instead of the stack-aware event runner.

- Evidence: harness-plan parse failure calls `guard.emit_blocked_or_failure()` at `claudine/cli/src/commands/wrap/composition/mod.rs:1038-1047`.
- Evidence: shell approval failure, including lifecycle stack shell audit denial, calls the same legacy path at `claudine/cli/src/commands/wrap/composition/mod.rs:1062-1075`.
- Evidence: dry-run pre-check failure does the same at `claudine/cli/src/commands/wrap/composition/mod.rs:1121-1125`.
- The legacy emitter only emits a subset of top-level communication and never executes the typed stack or `finalize`: `LifecycleRunGuard::emit_signal` emits `stderr`/`message`/`notify`/audio, but not `info`, `warn`, or any stack actions at `claudine/lib/src/composition/lifecycle.rs:655-700`; `Drop` also never emits `finalize` (`claudine/lib/src/composition/lifecycle.rs:704-714`).

This means a document with a denied lifecycle shell action such as:

```yaml
blocked:
  stack:
    - action: "append_line('events.log', 'blocked')"
finalize:
  stack:
    - action: "append_line('events.log', 'finalize')"
start:
  stack:
    - action: "shell('curl https://example.invalid')"
```

can fail during composition preflight without recording either marker. That violates both the `blocked -> finalize` acceptance criterion and the top-level/stack additive contract.

Verification level: existing L2 blocked coverage uses a legacy harness `pre_checks.file_exists` failure inside `run_harness_loop` (`claudine/cli/tests/level2_lifecycle_dispatch.rs:291-328`), so it does not exercise the earlier composition preflight failure path that owns schema/shell audit. Required coverage is L2 because this is user-observable lifecycle dispatch and terminal/file side-effect behavior.

### High: lifecycle `requeue(...)` is not implemented cross-platform, and the L2 proof is Unix-only

The accepted feature defines `Requeue` as a real control action that pushes the prompt onto the rendezvous deferred-execution queue (`spec.md:343`, `spec.md:682`). The current implementation only wires this through a Unix-domain-socket rendezvous client. On non-Unix platforms, `enqueue_requeue_entry` always returns an error saying the transport requires Unix-domain sockets (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:589-600`), and `dispatch_terminal_control` converts that into `LifecycleRequeueEnqueueFailed` (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:772-801`).

That is not production-ready for this monorepo’s platform contract. The CLI dependencies are Unix-gated (`claudine/cli/Cargo.toml:51-58`), the only L2 requeue test is in a `#![cfg(unix)]` file (`claudine/cli/tests/level2_lifecycle_control.rs:67`), and the rendezvous client/daemon APIs used by the feature are UDS-specific (`claudine/rendezvous/client/src/lib.rs:14-47`, `claudine/rendezvous/daemon/src/server.rs:19-23`, `:284-332`). A Windows user authoring a valid `requeue(...)` lifecycle action gets a hard failure instead of a queued prompt.

Verification level: Unix L2 coverage now proves the happy path records a rendezvous entry (`claudine/cli/tests/level2_lifecycle_control.rs:566-629`), but there is no Windows implementation and no Windows-facing L1/L2 contract test for the fallback. Because `Requeue` is specified user-observable control flow and the repo requires macOS, Windows, and Linux support, this remains a high-severity readiness gap.

## Notes

The review-2 blockers around success/blocked top-level ordering, initialize proxy handoff, and successful resume coverage appear materially improved. New L2 tests now cover success downgrade ordering, initialize proxy target initialization, successful resume with session id, and Unix requeue behavior. The remaining blockers are not about test count; they are spec-level behavior gaps on important execution paths, so I do not consider the feature production-ready.
