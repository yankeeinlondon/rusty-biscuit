---
ready: false
implemented: true
agent: codex/default
created: 2026-06-20T15:04:46
---

# Review 4: Runaway-Output Guards + Ctrl+C Hardening

## Findings

### High: Kimi wire-mode content trips are armed but cannot terminate the child

The spec requires content-guard trips to converge on the same termination plumbing, synthesize `error_kind`, and map to `ProcessTermination::Aborted` so failure handlers run without retrying the runaway (`spec.md:71`, `spec.md:461`, `spec.md:510`, `spec.md:543`, `spec.md:681`). The structured wrapper path arms the detector before building parser plumbing, including Kimi wire runs:

- `claudine/cli/src/commands/wrap/harness_orch/attempt.rs:141` installs `runaway_guards.detector`.
- `claudine/cli/src/commands/wrap/harness_orch/attempt.rs:147` builds the content early-termination channel.
- `claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs:115` feeds all `OutputText` / `Reasoning` semantic events into the detector.
- `claudine/cli/src/commands/wrap/exec/wiring/session.rs:144` feeds Kimi wire stdout into the same semantic parser/sink.

But the Kimi wire branch then discards the receiver:

- `claudine/cli/src/commands/wrap/harness_orch/attempt.rs:160` claims there is no structured stdout stream to scan, even though the parser/sink is fed above.
- `claudine/cli/src/commands/wrap/harness_orch/attempt.rs:162` drops `content_early_rx`.
- The direct wrapper path does the same at `claudine/cli/src/commands/wrap/wrapper_exec.rs:95`.

Result: a detector trip in Kimi wire mode can set `content_tripped` and suppress rendering, but no wait loop receives the trip. The child is not killed as `Aborted`, no guard `error_kind` / `guard_context` reaches the summary or handler payload, and the loop can continue until prompt completion, timeout, or manual interrupt.

Verification level present: Level 1 sink tests prove detector trips send on a channel when a receiver is wired, but there is no Kimi wire integration test covering the receiver path. Required level: at least Level 1 real-process/wire-session test with a fake Kimi wire server emitting repetitive `OutputText` or matching an exit expression, asserting `ProcessTermination::Aborted`, `error_kind`, and guard context. This is a functional gap, not only a test gap.

### High: Windows Ctrl+C parity is still not runtime verified

The spec makes Windows in scope and not best-effort: an equally robust Ctrl+C / group-kill implementation and a matrix test or documented proof on both Unix and Windows (`spec.md:626`, `spec.md:684`). The implementation now has a Windows wait-loop and a Windows-only integration test, but the documentation still records that it has not been runtime-run:

- `claudine/docs/topics/signal-handling.md:356` says the Windows console-event test is cross-compile checked but "NOT yet runtime-run on a Windows host / CI".
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:376` gates the Windows verification test behind `#[cfg(windows)]`.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:378` marks it `#[ignore]`, requiring a manual ignored run in an attached Windows console.

That is honest progress, but it does not satisfy the all-OS success criterion. The Windows implementation is a distinct code path using `SetConsoleCtrlHandler`, `GenerateConsoleCtrlEvent`, and Job Objects; macOS execution plus cross-compile cannot prove the runtime behavior.

Verification level present: compile-only plus an ignored Windows-host integration test artifact. Required level: Windows-host runtime verification, automated in CI if possible or recorded as a concrete manual run with date, host, command, and result. Until then, the feature cannot be marked production-ready under the review rubric.

## Notes

The prior Unix Level 3 gap appears addressed: `level3_wrap_ctrl_c.rs` now uses `cliclick` against a real WezTerm window for the user-key path, with L2 tmux coverage separated into `level2_wrap_ctrl_c_tmux.rs`.

Targeted checks run:

- `cargo nextest run --color=never -p claudine-cli content_guard` — passed 12/12.
- `cargo nextest run --color=never -p claudine-cli --test level3_wrap_ctrl_c --no-run` — compiled.

I did not run full `just test`, `just test-l2`, `just test-l3`, or any Windows-host test during this review.

## Recommendation

Do not mark this feature production-ready yet. Wire Kimi wire-mode content trips into its wait loop or explicitly disable detector arming for that path with matching documentation and tests. Then runtime-verify the Windows Ctrl+C / Job Object path before closing the feature.
