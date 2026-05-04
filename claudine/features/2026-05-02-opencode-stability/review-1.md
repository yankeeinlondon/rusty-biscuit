---
ready: false
agent: "codex"
model: ""
---

# Review: OpenCode Stability

## Findings

### High: The wait loop still enforces `timeout` and `step_timeout` outside the unified watchdog

The spec requires "two timeouts, one termination path" and says watchdog breaches must flow through the same early-termination pathway so the synthesized summary records `error_kind: "timeout"` or `"step_timeout"` and the `step_timeout` message can include outstanding subagent diagnostics.

The implementation starts the new watchdog ticker, but then also passes the same `wall_clock_timeout` and `step_timeout_duration` into `wait_with_signal_and_early_termination`:

- `claudine/cli/src/commands/wrap/exec.rs:2255` enables the watchdog ticker.
- `claudine/cli/src/commands/wrap/exec.rs:2289` calls the advanced wait loop with both raw timeout durations.
- `claudine/cli/src/commands/wrap/exec.rs:819` still has a direct wall-clock branch that sets `wall_clock_tripped` and never creates `EarlyTermination::Timeout`.
- `claudine/cli/src/commands/wrap/exec.rs:919` still has a direct `detect_step_timeout` branch that creates `EarlyTermination::StepTimeout` with `outstanding: Vec::new()`.

Those branches poll every 75 ms, while the watchdog ticker defaults to 5 s. In practice the old branches can win the race, kill the child first, and bypass the watchdog-rendered `Agent Error` block and subagent snapshot. For wall-clock timeout, the direct branch also returns `early_termination = None`, so `apply_early_termination_to_summary` never sets `error_kind: "timeout"`.

Verification level present: Level 1/CLI integration tests with fake provider scripts, but they do not make the race deterministic or assert the JSONL/session summary error kind. This is a functional gap, not just a test gap.

Recommended fix: remove `wall_clock_timeout` and `step_timeout` enforcement from the wait loop when the unified watchdog is enabled. The wait loop should only consume `WatchdogTermination` requests, apply the configured kill grace, and carry the converted `EarlyTermination` through summary finalization.

### High: `compose --timeout` / `compose --step-timeout` are ignored on the common non-harness path

The spec requires timeout resolution before launching `compose`, `inline-compose`, and `sequence`, with CLI > frontmatter > env > built-in precedence. Plain compose documents without harness frontmatter take the non-harness path, but that path never passes `request.timeout` or `request.step_timeout` into execution:

- `claudine/cli/src/commands/wrap/composition.rs:1264` calls `execute_without_harness(...)` without timeout arguments.
- `claudine/cli/src/commands/wrap/composition.rs:1784` explicitly resolves non-harness timeouts as `resolve_timeouts(None, None, None, None)`.

That means env defaults and the built-in `step_timeout` apply, but a user-supplied `claudine compose --opencode --timeout ... prompt.md` or `--step-timeout ...` does not override them unless the document also happens to enable the harness. This misses the primary user-facing command shape from the spec.

Verification level present: no targeted Level 1 integration test for CLI timeout precedence on non-harness `compose` / `inline-compose`. Existing watchdog integration tests invoke plain `compose`, but they configure timeouts only through env vars, so they cannot catch this.

Recommended fix: thread the resolved `TimeoutConfig` into `execute_without_harness` / `run_structured_composition` using `request.timeout` and `request.step_timeout`, and add CLI-precedence tests for non-harness compose and inline-compose.

### High: The user-visible timeout diagnostics are not verified at the required level

The spec requires concrete stderr behavior: an `Agent Error` block on timeout, stuck-subagent enumeration for `step_timeout`, and idle diagnostics like `Awaiting subagent: <name> (<elapsed>)`. The strongest current tests are Level 1 process tests using fake provider scripts and plain stderr string assertions.

Requirement verification:

- `step_timeout` terminates a silent OpenCode run: Level 1 integration only. This is acceptable for process-control semantics, but the direct wait-loop race above makes it incomplete.
- Stuck subagent ids/names appear in the breach error: Level 1 only, and the test accepts `sa8 OR Task 8`, so it does not verify the required id/name enumeration shape.
- `Agent Error` block rendering, colors, block quote border, spacing: Level 1 substring only. This requires Level 2 real-terminal capture because it is terminal rendering, width, glyph, and SGR behavior.
- Idle `Awaiting subagent` diagnostic before kill: no integration coverage found. Unit tests cover `diagnostic_lines`, but no test verifies the live stderr ticker emits the line.
- Synthesized summary error kind `timeout` / `step_timeout`: unit tests for `apply_early_termination_to_summary`, but no end-to-end JSONL/session summary assertion on the wrapper path.

Under the requested rubric, the rendered stderr requirements are not production-ready until they have at least Level 2 coverage. Process-control requirements can stay Level 1 if the fake-child tests assert deterministic exit semantics and summary fields.

Recommended fix: add Level 1 tests that assert exact stderr text shape and JSONL summary fields, plus Level 2 tests under tmux/WezTerm/Kitty for the `Agent Error` block and `Awaiting subagent` line. The feature does not involve OS keyboard input, so Level 3 is not required.

### Medium: `--timeout` does not use the spec's duration grammar

The spec says `--timeout` and `--step-timeout` both accept the same duration grammar as frontmatter and env (`30s`, `5m`, `2h`, etc.; bare seconds are not accepted). The implementation only does that for `--step-timeout`:

- `claudine/cli/src/commands/wrap/mod.rs:713` defines `--timeout <SECONDS>` as `Option<u64>`.
- `claudine/cli/src/commands/wrap/composition.rs:139` treats CLI timeout values as seconds directly.

So `--timeout 2h` is rejected by clap, while `--timeout 30` is accepted even though the spec says bare seconds are not accepted. The README also documents `<SECONDS>`, which conflicts with the spec and the new timeout topic.

Verification level present: Level 1 unit tests cover timeout precedence with numeric seconds, but no CLI parser test covers duration strings or rejection of bare seconds.

Recommended fix: make `--timeout` an `Option<String>`, parse it with `claudine::harness::parse_timeout` like `--step-timeout`, reject zero on CLI/frontmatter, and update tests/docs to one grammar.

## Additional Notes

The core state model is reasonable: `WatchdogState` is explicit-clock testable, `TimeoutConfig::resolve` separates user-facing timeout precedence from supporting env knobs, and the OpenCode fixture tests are a good Level 1 foundation. The largest ergonomics improvement is to remove the older timeout fields from `AttemptLaunch` / wait-loop plumbing once `TimeoutConfig` is the single contract; today both representations coexist and made the race easy to introduce.

## Test Run

I started targeted checks:

- `cargo test -p claudine-cli watchdog_subagent_hang_terminates_and_names_stuck_ids --test wrap_commands -- --nocapture`
- `cargo test -p claudine-cli watchdog_wall_clock_timeout_terminates_active_stream --test wrap_commands -- --nocapture`

Both were still compiling dependencies when this review was written. The source-level issues above do not depend on those results.

## Readiness

Not ready for production. The main watchdog concept is implemented, but CLI timeouts are ignored on a common compose path, the old wait-loop timeout branches can bypass the new diagnostics and summary mapping, and user-visible rendering lacks the Level 2 verification required by the review rubric.
