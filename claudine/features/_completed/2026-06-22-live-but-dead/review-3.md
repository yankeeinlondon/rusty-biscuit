---
ready: true
agent: codex/default
created: 2026-06-28T09:56:14
implemented: true
---

# Review 3 - Live-but-Dead Guard

## Verdict

Production ready.

The review 2 blocker in the `stall_timeout` configuration surface has been fixed. I did not find remaining gaps against the spec's acceptance criteria or the required verification levels.

## Findings

No blocking findings.

## Review Notes

- `stall_timeout` now uses the same duration grammar across CLI, frontmatter, and env paths while allowing true zero durations as a disable sentinel. Frontmatter parsing uses `parse_timeout_allow_zero`, direct wrapper CLI validation rejects malformed `--stall-timeout` values before launch, and fractional values such as `0.5s` resolve to 500ms instead of disabling the guard.
- The previous stdout-progress gap remains addressed through `StalledProgressObserverSink`, which shares the OpenCode bridge's stalled-generation progress cell and resets it for stdout progress-class semantic events.
- Structured `guard_context` now includes the required stalled-generation counters plus safe OpenCode identity metadata when present: session id, step, agent, provider id, model id, and mode.
- Documentation and CLI surfaces describe the guard as OpenCode-scoped, not a third general timeout, with `--stall-timeout`, frontmatter `stall_timeout`, `CLAUDINE_OPENCODE_STALL_TIMEOUT`, built-in `10m`, and `0s` disable semantics.

## Test Rigor

- Retry-churn trip condition: Level 1 present in OpenCode bridge tests, including the four streamed generations past budget case. Level 1 is appropriate for the detector logic.
- Progress reset taxonomy, including stdout-origin progress: Level 1 present through `StalledProgressObserverSink` and bridge reset tests. Level 1 is appropriate for producer/sink wiring.
- Long-tool exemption and liveness-only events: Level 1 present. Level 1 is appropriate because this is event classification, not terminal rendering.
- Termination mapping to `ProcessTermination::Aborted`, `error_kind = "stalled_generation"`, and structured `guard_context`: Level 1 present in termination tests.
- `stall_timeout` configuration semantics: Level 1 present for CLI, frontmatter, env, zero disable, fractional duration, and invalid CLI/frontmatter values.
- User-visible stalled-generation rendering: Level 2 present via `claudine/cli/tests/level2_stalled_generation_capture.rs`, which runs the wrapper in tmux and captures the rendered pane. Level 2 is appropriate; Level 3 is not required because this feature does not depend on terminal input encoding.

## Verification Run

Passed:

```text
cargo nextest run -p claudine -p claudine-cli -E 'test(/stalled_generation/) | test(/stall_timeout/) | test(/parse_cli_timeouts/) | test(/parse_timeout_allow_zero/)' --no-fail-fast --color never
```

Result: 32 tests passed, including `level2_stalled_generation_renders_in_tmux`.

## Production Readiness

Ready for production. Each user-observable requirement has verification at the appropriate level, and the prior review's public configuration blocker is covered by focused Level 1 tests.
