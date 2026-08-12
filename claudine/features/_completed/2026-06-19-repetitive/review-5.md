---
ready: false
agent: codex/default
created: 2026-06-20T16:31:52
implemented: true
---

# Review 5: Runaway-Output Guards + Ctrl+C Hardening

## Findings

### High: Kimi wire mode still bypasses the Windows Ctrl+C / Job-Object termination path

The spec requires every spawn/wait path to share the unified signal-aware wait loop, with Windows parity via `CREATE_NEW_PROCESS_GROUP`, `SetConsoleCtrlHandler`, `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, ...)`, and Job Object termination (`spec.md:271`, `spec.md:623`, `spec.md:684`). Kimi wire mode is still a separate wait implementation:

- `claudine/cli/src/commands/wrap/exec/wiring/session.rs:59` only puts the child in a new process group on Unix. There is no Windows `CREATE_NEW_PROCESS_GROUP` branch here, unlike the standard spawn paths.
- `claudine/cli/src/commands/wrap/exec/wiring/session.rs:291` makes `install_sigint_forwarder` a no-op on non-Unix, so Windows console Ctrl+C / Ctrl+Break does not set the cancel flag for Kimi wire sessions.
- `claudine/cli/src/commands/wrap/exec/wiring/session.rs:309` uses `wait_for_child_exit`, not `wait_with_signal_and_early_termination`, so it never installs the Windows console handler from `windows_wait_loop`.
- `claudine/cli/src/commands/wrap/exec/wiring/session.rs:411` handles non-Unix content trips with `child.kill()` only, not Job Object tree termination.

This means the normal structured stream path may have Windows Job Object handling, but Kimi wire mode does not. A Windows user pressing Ctrl+C during a Kimi wire run is not covered by the advertised parity path, and a content guard trip on Windows can kill only the immediate child rather than the whole process tree.

Verification level present: Level 1 Kimi wire content-trip integration now verifies `ProcessTermination::Aborted`, `error_kind`, and guard context on the local host, and the previous Kimi receiver gap appears closed. Required level: implementation must route Kimi wire through the same Windows process-group / Job Object / console-control machinery, then verify the Windows runtime behavior on a Windows host. This is a functional gap, not just a test gap.

### High: Windows Ctrl+C parity is still not runtime verified

The spec says Windows support is in scope and "not best-effort" (`spec.md:684`). The implementation and documentation are honest that the Windows path has not yet been proven at runtime:

- `claudine/docs/topics/signal-handling.md:340` says the Windows path is cross-compile checked but full parity is not claimed until a Windows workflow or equivalent manual command has a recorded green run.
- `claudine/docs/topics/signal-handling.md:358` records the Windows console Ctrl+Break test as an ignored, attached-console test with no recorded green Windows runtime run.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:320` defines the Windows parity test, but it is `#[cfg(windows)]` and `#[ignore]`, so it does not establish an automated production gate today.

Verification level present: compile-only plus an opt-in ignored Windows-host test artifact. Required level: Windows-host runtime verification for the user-observable "press Ctrl+C / Ctrl+Break and the wrapped child terminates" requirement, with the result recorded or CI-gated. Under the review rubric, this remains a high-severity readiness gap.

## Notes

The Review 4 Kimi receiver issue is materially improved: both harness and direct wrapper paths now pass `content_early_rx` into `run_kimi_wire_session`, and `exec/wiring/tests.rs` asserts a fake Kimi wire exit-expression trip produces `Aborted`, `error_kind = "exit_expression"`, and guard context.

Unix/macOS coverage is much stronger than in earlier iterations: there are L3 `cliclick` tests for real OS Ctrl+C input, L2 tmux tests for multiplexer injection, and L2 capture for visible interrupt feedback. The remaining blocker is the Windows behavior and the Kimi wire exception to the unified wait design.

I did not run the full test suite during this review. This was a source-level review focused on the specification requirements, prior review closure, and verification-level evidence.

## Recommendation

Do not mark this feature production-ready yet. Route Kimi wire mode through the same cross-platform termination abstraction as the other spawn paths, including Windows process-group / Job Object handling, then obtain a real Windows-host green run for the Ctrl+C parity test.
