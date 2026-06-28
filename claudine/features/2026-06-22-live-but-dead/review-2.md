---
ready: false
agent: codex/default
created: 2026-06-28T09:29:39
implemented: true
---

# Review 2 - Live-but-Dead Guard

## Verdict

Not production ready.

The issues from review 1 are substantially addressed: stdout-origin progress now resets the shared stalled-generation state, the structured guard context now carries safe OpenCode identity metadata, and there is a new Level 2 tmux capture for the terminal-visible stalled-generation block. I found one remaining production blocker in the public `stall_timeout` configuration surface.

## Findings

### High - `stall_timeout` parsing does not implement the documented configuration contract

The spec says `stall_timeout` uses the same duration grammar and strict source precedence as `timeout` / `step_timeout`, and that `0s` disables the guard from the CLI flag, frontmatter, or `CLAUDINE_OPENCODE_STALL_TIMEOUT`. The implementation is inconsistent across those sources:

- Frontmatter `stall_timeout: "0s"` does not disable the guard. `parse_harness_plan` sends the raw string straight through `parse_timeout`, which rejects zero durations (`claudine/lib/src/harness/parse/mod.rs:168`, `claudine/lib/src/harness/timeout.rs:67`). That violates the spec's `0s` disables rule for frontmatter.
- Direct wrapper `--stall-timeout <bad-value>` is not validated with the other CLI timeouts. `parse_cli_timeouts` validates `--timeout` and `--step-timeout` only (`claudine/cli/src/commands/wrap/wrapper_stages.rs:50`), while the direct wrapper later calls `resolve_stall_timeout` (`claudine/cli/src/commands/wrap/wrapper_exec.rs:85`). `resolve_single_timeout` swallows invalid CLI parse errors and falls through to env/built-in defaults (`claudine/cli/src/commands/wrap/composition/timeouts.rs:59`). A typo such as `--stall-timeout nope` therefore silently uses the default instead of failing like the other CLI duration flags.
- Valid fractional durations that start with zero are treated as disable sentinels. The shared parser accepts decimals (`parse_timeout` includes `.` in the numeric prefix and parses `f64`, `claudine/lib/src/harness/timeout.rs:38` and `:61`), so `0.5s` is a valid duration. However, both zero-literal helpers stop at the first non-digit and treat the leading `0` as a disable literal (`claudine/cli/src/commands/compose/mod.rs:41`, `claudine/cli/src/commands/wrap/composition/timeouts.rs:94`). That means `--stall-timeout 0.5s` and `CLAUDINE_OPENCODE_STALL_TIMEOUT=0.5s` disable the live-but-dead guard instead of arming it for 500ms.

This is user-observable behavior and it directly affects whether the guard is armed. It also makes the new Level 2 test's documented rationale misleading: the test comments say `0.5s` would collapse to `0s`, but according to the canonical duration parser it should be accepted as a fractional second.

Fix by centralizing `stall_timeout` parsing around the same duration grammar used by the other timeouts, with a precise disable check for actual zero durations only. Then add Level 1 tests for CLI, env, and frontmatter cases: `0s` disables from all three sources; `0.5s` resolves to 500ms; invalid direct-wrapper CLI values fail instead of falling through.

## Test Rigor

- Retry-churn trip condition: Level 1 present in the OpenCode bridge tests. Level 1 is appropriate for detector logic.
- Progress reset taxonomy, including stdout-origin progress: Level 1 present through `StalledProgressObserverSink` tests in `opencode/reasoning.rs`. Level 1 is appropriate for this producer/sink wiring.
- Long-tool exemption and liveness-only events: Level 1 present. Level 1 is appropriate because this is event classification, not terminal rendering.
- Termination mapping to `ProcessTermination::Aborted`, `error_kind = "stalled_generation"`, and structured `guard_context`: Level 1 present in `exec/termination.rs`.
- User-visible `Agent Error` / stalled-generation rendering: Level 2 present via `claudine/cli/tests/level2_stalled_generation_capture.rs`. Level 2 is appropriate; Level 3 is not required because this feature does not depend on terminal input encoding.
- `stall_timeout` configuration semantics: Level 1 is incomplete and reveals the blocker above.

## Verification Run

- Attempted `cargo nextest run -p claudine -p claudine-cli -E 'test(/stalled_generation/) | test(/stall_timeout/) | test(/level2_stalled_generation/)' --no-fail-fast --color never`.
- Stopped it with Ctrl+C after about 60 seconds because it was still compiling dependencies in this non-interactive review session. No pass/fail result should be inferred from that attempt.

## Production Readiness

Not production ready. The core guard behavior and prior review findings look addressed, but the public `stall_timeout` surface can silently disable or ignore the guard in common configuration paths.
