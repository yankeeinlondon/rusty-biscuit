---
ready: false
agent: unknown/default
created: 2026-06-19T21:50:04
---

# Review 1: Runaway-Output Guards + Ctrl+C Hardening

## Findings

### High: Repo/frontmatter guard config errors are silently ignored

The spec requires invalid exit-expression regexes and unknown `scope` agents to fail at config-load across the three layers. The implementation only validates the merged `ClaudineConfig` path; the wrapper resolver then loads repo/frontmatter guard config independently and suppresses failures:

- `claudine/cli/src/commands/wrap/runaway_guard.rs:56` defaults an unreadable/invalid user config to built-ins.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:59` drops repo override load errors with `.ok().flatten()`.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:68` drops malformed frontmatter `exit_expressions`.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:96` catches compile failures and disables the compiled exit-expression set.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:152` drops malformed frontmatter `guard_settings`.

That means a repo can declare a safety rule with a typo, or a prompt can declare an invalid scoped rule, and Claudine proceeds with weaker or empty guards. This violates the safety contract in the spec's Cluster E3 / success criteria and can make the feature look enabled while the intended stop condition is absent.

Verification level present: Level 1 unit coverage for validation helpers exists in `claudine/lib/src/runaway/config.rs`, but there is no test proving wrapper resolution fails closed for repo/frontmatter invalid values. Required level: Level 1 is enough for this config contract, but it must cover the actual resolver path.

### High: Ctrl+C user-key behavior is not verified at the required level

The spec requires that pressing Ctrl+C terminates the wrapped child on every spawn/wait path, including configured wall-clock timeouts, and that visible interrupt feedback appears. Under the review rubric, "when the user presses key X, Y happens" needs Level 3 OS keyboard injection, because process-level `libc::kill(pid, SIGINT)` does not exercise terminal/OS key delivery.

Current coverage is below that bar:

- `claudine/cli/tests/wrap_sigint.rs:89` sends SIGINT directly with `libc::kill`.
- `claudine/cli/src/commands/wrap/exec/spawn.rs:1463` and `:1505` cover timeout reaping with real child processes, but not a user Ctrl+C keypress through a terminal.
- `claudine/cli/src/commands/wrap/exec/termination.rs:1448` unit-tests ladder selection, not terminal input.
- `claudine/cli/src/commands/wrap/exec/termination.rs:409` documents that Windows runtime behavior is unverified.

Verification level present: Level 1/process-signal tests on Unix, plus a Windows compile/smoke path only. Required level: Level 3 for actual Ctrl+C key behavior; Level 2 capture is also appropriate for the visible feedback line. Production readiness should wait for at least one L3 test that launches Claudine inside a real terminal window and injects Ctrl+C, plus coverage for each spawn/wait matrix class or a documented matrix proof backed by representative tests. Windows parity also needs a Windows-host CI or manual validation record.

### High: No end-to-end test proves composed prompt frontmatter enables the guards

The detector and sink are well covered in-process, but the user-facing composition path is where frontmatter `exit_expressions` and `guard_settings` are supposed to apply. The current resolver is private CLI wiring and silently drops malformed frontmatter, but there is no binary-level test that runs `claudine compose` with frontmatter guard settings, streams a fake provider response, and observes `ProcessTermination::Aborted` / `error_kind`.

Verification level present: Level 1 detector tests and sink seam tests, plus handler-payload tests. Required level: Level 1 process-level integration is sufficient for the config-to-wrapper-to-handler contract; Level 2 is not necessary unless asserting terminal rendering. Without this, the strongest tests prove the algorithm, not that the feature works from the user's document.

## Notes

The pure detector implementation matches the main algorithmic requirements: line reassembly, exact normalized group-cycle detection, blank-line handling, OutputText/Reasoning-only scanning, volume cap, and bounded ring memory are covered by Level 1 tests. Handler payload propagation for `exit_expression`, `runaway_repetition`, and `runaway_volume` is also covered at Level 1.

I ran `just test` from `claudine/`. The `claudine` library suite passed 2782/2782 tests, and `claudine-contract` passed 47/47 tests. The run failed in `claudine-cli::command_routing::force_color_enables_ansi_in_non_tty_context` at `claudine/cli/tests/command_routing.rs:309` because it expected ANSI output but received plain output. The CLI run stopped after 111/1652 tests due to fail-fast, so the full CLI suite was not completed.

## Recommendation

Do not mark this feature production-ready yet. Fail closed on invalid repo/frontmatter guard config, add an end-to-end composition wrapper test for frontmatter-driven guards, and add L3 Ctrl+C coverage plus L2 feedback rendering coverage before closing the feature.
