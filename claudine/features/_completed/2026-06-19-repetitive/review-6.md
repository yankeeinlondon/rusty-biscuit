---
ready: false
agent: codex/default
created: 2026-06-20T16:57:56
---

# Review 6: Runaway-Output Guards + Ctrl+C Hardening

## Findings

### High: Windows watchdog/content-guard termination skips the required graceful rung

The spec requires the content guards to use the same termination plumbing as the timeout watchdog: graceful termination first, then forceful termination after the grace period. It says this explicitly for all content guards (`spec.md:70`) and again for exit-expression behavior (`spec.md:462`), with Windows mapped as Ctrl+Break / graceful followed by `TerminateJobObject` / forceful (`spec.md:294`, `spec.md:304`).

The Windows wait loop implements that ladder only for user console interrupts. A first Ctrl+Break path calls `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child_process_id)` and only a repeated interrupt calls `TerminateJobObject` (`claudine/cli/src/commands/wrap/exec/termination.rs:620`). But wrapper-driven requests bypass that graceful rung:

- content-guard / generic early termination immediately calls `TerminateJobObject(job, 1)` (`termination.rs:672`);
- Kimi completion termination immediately calls `TerminateJobObject(job, 0)` (`termination.rs:691`);
- watchdog timeout / step-timeout immediately calls `TerminateJobObject(job, 1)` (`termination.rs:711`);
- only after that immediate force kill does the code set a `grace_deadline`, whose later branch can only call `TerminateJobObject` again (`termination.rs:733`).

This is not parity with the Unix SIGTERM -> SIGKILL path and not the Windows graceful -> forceful ladder described by the spec. It also weakens cleanup semantics for providers that could handle Ctrl+Break and exit cleanly before the forceful job kill.

Verification level present: no Windows runtime test for wrapper-driven timeout/content-guard graceful escalation. Required level: at least a Windows-host real-process test that triggers a watchdog/content-guard request and proves the first action is the graceful Ctrl+Break path, with forceful Job Object termination reserved for grace expiry.

### High: Windows Ctrl+C parity is still not runtime verified

The spec requires Windows support to be equally robust, not best-effort (`spec.md:626`, `spec.md:664`). The implementation now has a real Windows test and CI workflow, which is an improvement over review 5, but the repository still states that no Windows runtime pass has been recorded:

- `claudine/docs/topics/signal-handling.md:340` says full parity is not claimed until the Windows-host gate has a recorded green run.
- `claudine/docs/topics/signal-handling.md:359` records the Windows Ctrl+Break row as "no recorded green Windows runtime run in this repo yet."
- `claudine/docs/topics/signal-handling.md:405` says the test is cross-compile-checked, but its runtime pass has not yet been recorded.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:379` defines `windows_ctrl_c_verification_record`, but it is `#[cfg(windows)]` and `#[ignore]`, so normal test runs do not prove the user-observable Windows behavior.

The workflow `.github/workflows/claudine-windows-ctrl-c.yml` and `just test-windows-ctrl-c` are useful gates, but the review rubric asks for the strongest verification actually present. For the requirement "a Windows user presses Ctrl+C/Ctrl+Break and the wrapped child terminates," compile-only plus an ignored Windows-host test artifact is below the required runtime verification level.

Verification level present: compile-check evidence plus an opt-in ignored Windows integration test and a path-filtered workflow definition. Required level: a recorded green Windows-host run of `windows_ctrl_c_verification_record` or a CI result that is part of the production-readiness evidence.

## Notes

The review-5 Kimi wire implementation blocker appears addressed. Kimi wire now sets `CREATE_NEW_PROCESS_GROUP` on Windows and routes through `wait_with_signal_early_termination_and_completion`, so it no longer has the separate no-op/non-Unix wait path called out in the previous review.

Unix/macOS coverage remains strong: Level 3 OS-keyboard tests cover real Ctrl+C delivery through WezTerm/cliclick, Level 2 tmux tests cover multiplexer injection, and Level 2 capture verifies the visible interrupt feedback line. The remaining blockers are Windows behavior and Windows verification.

I did not run the full test suite during this review. This was a source-level review focused on the specification requirements, prior review closure, and verification-level evidence.

## Recommendation

Do not mark this feature production-ready yet. Update the Windows wait loop so watchdog, completion, and content-guard termination use the same graceful Ctrl+Break -> forceful Job Object ladder as user interrupts, then record a green Windows-host runtime run for the Ctrl+C parity test and add Windows runtime coverage for wrapper-driven timeout/content-guard termination.
