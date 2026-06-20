---
ready: false
implemented: true
agent: codex/default
created: 2026-06-19T23:36:15
---

# Review 3: Runaway-Output Guards + Ctrl+C Hardening

## Findings

### High: The new “Level 3” Ctrl+C tests are not Level 3 under the review rubric

The spec requires Ctrl+C user-key behavior to be verified for every spawn/wait path, including with a wall-clock timeout configured (`spec.md:623`, `spec.md:684`). The review rubric is stricter than “a terminal-like key name was sent”: Level 3 means real OS keyboard events (`cliclick` on macOS, `xdotool` on Linux) injected into the terminal window so the terminal emulator's input encoder participates. The new tests use tmux command injection instead:

- `claudine/cli/tests/level3_wrap_ctrl_c.rs:9` describes `tmux send-keys C-c` as the mechanism.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:171` claims this is “genuine keyboard injection”.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:174` calls `harness.send_key("C-c")`, which routes through the tmux harness, not OS keyboard injection.
- `claudine/docs/topics/signal-handling.md:350` marks the Unix Ctrl+C surface as “Verified on macOS” by these tests, and `signal-handling.md:356` repeats the “genuine Ctrl+C key event” claim.

This is useful real-terminal/multiplexer coverage, but it is still terminal-CLI injection. It does not prove what bytes a real terminal emulator emits when the user presses Ctrl+C, which is the exact distinction the rubric calls out.

Verification level present: effectively Level 2-style tmux injection plus lower-level process-signal coverage. Required level: Level 3 OS keyboard injection for the user-key requirement. Add a `RUN_LEVEL3=1` test that opens/focuses a real terminal window and injects Ctrl+C with `cliclick`/`xdotool`, or reclassify the tmux tests as L2 and keep the feature not ready until true L3 exists.

### High: Windows Ctrl+C parity is still explicitly unverified

The spec makes Windows in-scope and not best-effort: “Windows: an equally robust Ctrl+C / group-kill implementation” (`spec.md:626`) and a matrix test or proof on “both Unix and Windows” (`spec.md:684`). The current artifact is an ignored placeholder that panics:

- `claudine/cli/tests/level3_wrap_ctrl_c.rs:263` says the Windows record is `#[ignore]`d by default.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:272` defines `windows_ctrl_c_verification_record`.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs:278` immediately panics that Windows parity is not exercised by an automated harness.
- `claudine/docs/topics/signal-handling.md:354` records Windows as “NOT verified”.

That honesty is good, but it means the success criterion is not met. A documented placeholder is not a test or proof that Ctrl+C terminates the Job Object child, especially because the Windows implementation is a distinct `#[cfg(not(unix))]` path.

Verification level present: compile/documentation record only. Required level: Windows-host runtime verification for the Ctrl+C behavior, ideally automated or at least a recorded manual run with concrete date, command, host, and result. Until then, the all-OS requirement blocks production readiness.

## Notes

The prior review's config and model-scope findings look addressed. Guard resolution now fails closed on present-but-invalid repo/frontmatter declarations, and the sink can re-scope exit expressions when `SessionStart` reports the actual model. The new L1 tests cover that re-scope behavior.

Targeted checks run:

- `cargo nextest run --color=never -p claudine-cli content_guard` — passed 12/12.
- `cargo nextest run --color=never -p claudine-cli --test level3_wrap_ctrl_c --no-run` — compiled.
- `cargo nextest run --color=never -p claudine-cli --test level2_interrupt_feedback_capture --no-run` — compiled.

I did not run `just test-l2`, `just test-l3`, `just test`, or `just lint` in full during this review. Even if the tmux tests pass, the Level 3 requirement remains unmet because the injection mechanism is below the required level.

## Recommendation

Do not mark this feature production-ready yet. Reclassify the tmux Ctrl+C tests as L2 coverage, add true OS-keyboard-injection tests for Unix, and produce a real Windows-host verification of the Job Object / console-control path before closing the feature.
